use chrono::{NaiveDate, Weekday};
use inquire::*;

use crate::database;
use crate::database::DbConn;
use crate::user;

pub fn create_user(_db: &mut DbConn) -> u32 {
    let name: String = Text::new("Enter user name:").prompt().unwrap();
    let admin: bool = Confirm::new("Elevate user to administrator:").with_default(false).prompt().unwrap();
    println!("admin: {}", &admin);
    _db.add_user(name, admin).unwrap()
}

pub fn create_admin(_db: &mut DbConn) -> u32 {
    let name: String = Text::new("Enter admin name:").prompt().unwrap();
    _db.add_user(name, true).unwrap()
}

pub fn tui_set_user(_db : &mut DbConn) -> u32 {
    let id: u32;
    let users = _db.get_users().unwrap();
    if users.is_empty() {
        id = create_admin(_db);
    }
    else {
        let name: String = Select::new("Select current user:", users.to_vec()).prompt().unwrap().to_string();
        println!("Welcome {}!", name);
        let rid = _db.get_user_id(name);
        match rid {
            Ok(rid) => {
                println!("id is {}", rid);
                id = rid;
            }
            Err(error) => {
                panic!("Error is {}", error);
            }
        }
    }
    return id;
}
