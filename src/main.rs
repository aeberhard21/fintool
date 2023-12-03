use rusqlite::Error;

use crate::ledger::Ledger;
use crate::database::DbConn;
use crate::user::User;
use crate::tui::tui_user;

mod ledger;
mod tui;
mod database;
mod user;

fn main() {

    let mut _db : DbConn  = DbConn::new("db/checking.db").unwrap();
    let mut _users: Vec<User>= Vec::new();

    println!("Welcome to FinTool!");
    let mut _user : User;
    let next_id : u32 = 0;
    if _users.is_empty() {
        _user = tui::tui_user::create_user(next_id, &mut _db);
        // _db.add_user(_user);
        _users.push(_user);
    }
    // } else {
    //     _user = tui::login(_users);
    // }
    tui::menu(&mut _users[next_id as usize]);
    
    _db.close();
}