use diesel::{self, prelude::*};
use itertools::Itertools;
use rcir;
use rstv;

mod schema {
    table! {
        users {
            id -> Integer,
            username -> Text,
        }
    }

    table! {
        items {
            id -> Integer,
            title -> Text,
            body -> Text,
            done -> Bool,
        }
    }

    table! {
        votes (user_id, item_id) {
            user_id -> Integer,
            item_id -> Integer,
            ordinal -> Integer,
        }
    }

    joinable!(votes -> items (item_id));
    joinable!(votes -> users (user_id));
    allow_tables_to_appear_in_same_query!(users, items, votes);
}

use self::schema::users;
use self::schema::votes;

#[derive(Queryable, Debug)]
pub struct User {
    pub id: i32,
    pub username: String,
}

#[derive(Serialize, Queryable, Debug)]
pub struct Item {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub done: bool,
}

#[table_name = "votes"]
#[derive(Queryable, Insertable, Debug, Clone)]
pub struct Vote {
    pub user_id: i32,
    pub item_id: i32,
    pub ordinal: i32,
}

use self::schema::items::dsl::{done as item_done, items as all_items};
use self::schema::users::dsl::{username as users_uname, users as all_users};
use self::schema::votes::dsl::{item_id, ordinal, user_id, votes as all_votes};

#[derive(Deserialize)]
pub struct Ballot {
    pub votes: Vec<i32>,
}

#[table_name = "users"]
#[derive(FromForm, Insertable)]
pub struct NewUser {
    pub username: String,
}

impl NewUser {
    pub fn login(self, conn: &SqliteConnection) -> User {
        // ensure that the user exists
        let _ = diesel::insert_into(self::schema::users::table)
            .values(&self)
            .execute(conn);

        all_users
            .filter(users_uname.eq(&self.username))
            .get_result::<User>(conn)
            .unwrap()
    }
}

impl Item {
    pub fn for_user(uid: i32, conn: &SqliteConnection) -> Vec<(Item, bool)> {
        all_items
            .left_join(
                self::schema::votes::table
                    .on(user_id.eq(&uid).and(item_id.eq(self::schema::items::id))),
            )
            .filter(self::schema::items::done.eq(false))
            .order((user_id.desc(), ordinal.asc()))
            .select((self::schema::items::all_columns, ordinal.nullable()))
            .load::<(Item, Option<i32>)>(conn)
            .unwrap()
            .into_iter()
            .map(|(i, ord)| (i, ord.map(|_| true).unwrap_or(false)))
            .collect()
    }
}

impl Vote {
    pub fn run_election(conn: &SqliteConnection) -> Vec<Item> {
        let mut results: Vec<Item> = Vec::new();

        let votes = all_votes
            .inner_join(self::schema::items::table)
            .filter(item_done.eq(false))
            .order((user_id.asc(), ordinal.asc()))
            .select((user_id, item_id, ordinal))
            .get_results::<Vote>(conn)
            .unwrap();

        // the extra collections here are sad.
        let mut votes: Vec<Vec<_>> = votes
            .into_iter()
            .group_by(|v| v.user_id)
            .into_iter()
            .map(|(_, ballot)| ballot.into_iter().map(|v| v.item_id).collect())
            .collect();

        //for i in 0..6 {
        //for r in results.iter() {
        //    for ballot in votes.iter_mut() {
        //         ballot.remove_item(&r.id);
        //      }
        //   }
        //   println!("Round {}: {:?}", i, votes);
        let mut election = rstv::STV::new();
        election.add_ballots(votes);
        election
            .run_election(10)
            .unwrap()
            .iter()
            .map(|iid| all_items.find(iid).get_result::<Item>(conn).unwrap())
            .collect()

        //match rcir::run_election(&votes) {
        //Err(e) => {
        //   println!("{:?}", e);
        //  break;
        //}
        //O//k(rcir::ElectionResult::Winner(&iid)) => {
        //  results.push(all_items.find(iid).get_result::<Item>(conn).unwrap());
        //}
        //O//k(rcir::ElectionResult::Tie(iids)) => {
        //  // TODO: maybe pick the oldest one?
        // results.push(all_items.find(*iids[0]).get_result::<Item>(conn).unwrap());
        //}
        //}
        //}
        //results
    }

    pub fn save_ballot(uid: i32, ballot: Ballot, conn: &SqliteConnection) {
        diesel::delete(all_votes.filter(user_id.eq(&uid)))
            .execute(conn)
            .unwrap();

        for (i, iid) in ballot.votes.into_iter().enumerate() {
            diesel::insert_into(self::schema::votes::table)
                .values(Vote {
                    user_id: uid,
                    item_id: iid,
                    ordinal: i as i32,
                })
                .execute(conn)
                .unwrap();
        }
    }

    /*
        let t = Task {
            id: None,
            description: todo.description,
            completed: false,
        };
        diesel::insert_into(tasks::table)
            .values(&t)
            .execute(conn)
            .is_ok()
    }
    
    pub fn toggle_with_id(id: i32, conn: &SqliteConnection) -> bool {
        let task = all_tasks.find(id).get_result::<Task>(conn);
        if task.is_err() {
            return false;
        }
    
        let new_status = !task.unwrap().completed;
        let updated_task = diesel::update(all_tasks.find(id));
        updated_task
            .set(task_completed.eq(new_status))
            .execute(conn)
            .is_ok()
    }
    
    pub fn delete_with_id(id: i32, conn: &SqliteConnection) -> bool {
        diesel::delete(all_tasks.find(id)).execute(conn).is_ok()
    }
    */
}
