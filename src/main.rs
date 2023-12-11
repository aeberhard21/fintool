use rusqlite::Error;
use std::path::{Path, PathBuf};
use std::fs::{self, create_dir};

use crate::ledger::Ledger;
use crate::database::DbConn;
use crate::user::User;
use crate::tui::tui_user;

mod ledger;
mod tui;
mod database;
mod user;

fn main() {

    let db_dir: String = String::from("./db");
   
    let mut _db : DbConn;
    match Path::new(&db_dir).try_exists() {
        Ok(true) => {
        }
        Ok(false) => {
            fs::create_dir(&db_dir);
        }
        Err(_) => {
            panic!("Unable to verify existence of database directory!");
        }
    }

    let mut db = PathBuf::new();
    db.push(&db_dir);
    db.push("finances.db");
    match Path::new(&db_dir).join(&db).try_exists() {
        Ok(_) => {
            // nothing to do
            _db = DbConn::new(db).unwrap();
            println!("Connect to db");
        }
        Err(_) => {
            panic!("Unable to verify existence of the database!");
        }
    }

    let mut _users: Vec<User>= Vec::new();
    _users = _db.restore_users().unwrap();
    println!("number of users: {}", _users.len());

    println!("Welcome to FinTool!");
    let mut _user : User;
    let next_id : u32 = 0;
    if _users.is_empty() {
        _user = tui::tui_user::create_user(next_id);
        // _db.add_user(_user);
        _users.push(_user);
    }
    // } else {
    //     _user = tui::login(_users);
    // }
    tui::menu(&mut _users[next_id as usize], &mut _db);
    
    _db.store_users(&mut _users);
    _db.close();
}