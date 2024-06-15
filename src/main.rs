use std::fs::{self};
use std::path::{Path, PathBuf};

use crate::database::DbConn;
use crate::user::User;
// use crate::stocks::*;

mod database;
mod ledger;
mod stocks;
mod tui;
mod user;

fn main() {
    let db_dir: String = String::from("./db");

    let mut _db: DbConn;
    match Path::new(&db_dir).try_exists() {
        Ok(true) => {}
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
        }
        Err(_) => {
            panic!("Unable to verify existence of the database!");
        }
    }

    println!("Welcome to FinTool!");
    let mut _user: User;
    let next_id: u32 = 0;
    {
        tui::menu(&mut _db);
    }
    _db.close();
}
