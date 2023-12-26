// use core::panic;
use crate::database::DbConn;
use crate::database::db_accounts::AccountType;
use crate::ledger::Ledger;
use crate::ledger::LedgerEntry;
use crate::tui::tui_ledger::*;
use crate::tui::tui_user::*;
use crate::tui::tui_accounts::*;
use crate::user::User;
use chrono::{NaiveDate, Weekday};
use inquire::*;

mod tui_ledger;
mod tui_accounts;
pub mod tui_user;

// pub fn login(_users: &mut Vec<User>) -> User {
//     let selected_user: User;
//     let mut users: Vec<&str> = Vec::new();
//     for user in _users.iter() {
//         users.push(user.get_name());
//     }
//     // let selected_user : String::new("User:");
//     return selected_user;
// }

pub fn menu(_db: &mut DbConn) {

    let mut uid: u32;

    // set current user first!
    uid = tui_set_user(_db);

    loop {
        let commands: Vec<&str> = vec!["create", "change", "add", "report", "view", "exit"];
        let command: String = Select::new("What would you like to do:", commands)
            .prompt()
            .unwrap()
            .to_string();

        match command.as_str() {
            "create" => {
                tui_create(uid, _db);
            }
            "change" => {
                uid = tui_set_user(_db);
            }
            "add" => {
                tui_add(uid, _db);
            }
            "view" => {
                tui_view(uid, _db);
            }
            "report" => {
                tui_report(uid,_db);
            }
            "exit" => {
                println!("Exiting...");
                break;
            }
            _ => {
                panic!("Invalid command.");
            }
        }
    }
}

fn tui_create(_uid: u32, _db: &mut DbConn) {
    let mut commands:Vec<&str> = Vec::new();
    if _db.is_admin(_uid).unwrap() {
        commands = vec!["user", "bank", "CD", "health", "investment", "ledger", "retirement", "none"];

    }
    else {
        commands = vec!["bank", "CD", "health", "investment", "ledger", "retirement", "none"];
    }
    let command: String = Select::new("\nWhat would you like to add:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "user" => {
            create_user(_db);
        }
        "bank" => {
            create_account(AccountType::Bank,_uid,  _db);
        }
        "CD" => {
            create_account(AccountType::CD, _uid, _db);
        }
        "health" => {
            create_account(AccountType::Health, _uid, _db);
        }
        "investment" => {
            create_account(AccountType::Investment, _uid, _db);
        }
        "ledger" => {
            create_ledger(_uid, _db);
        }
        "retirement" => {
            create_account(AccountType::Retirement,_uid,  _db);
        }
        "none" => return,
        _ => {
            panic!("Invalid command");
        }
    }
}

fn tui_add(_uid : u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["bank", "CD", "health", "investment", "ledger", "retirement", "none"];
    let command: String = Select::new("\nWhat would you like to add:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "investment" => {
            println!("Not implemented!");
        }
        "ledger" => {
            let ledgers = _db.get_user_accounts(_uid, AccountType::Ledger).unwrap();
            let ref ref_ledgers = &ledgers;
            loop {
                let selected_ledger: String = Select::new("Select which ledger to add to: ", ref_ledgers.to_vec()).prompt().unwrap().to_string();
                let entry = add_ledger(_uid, _db);
                let aid = _db.get_account_id(selected_ledger).unwrap();
                _db.add_ledger_entry(aid, entry);
                let another: bool = Confirm::new("Add another entry?")
                    .with_default(false)
                    .prompt()
                    .unwrap();
                if false == another {
                    break
                }
            }
        },
        "none" => {
            return;
        }
        _ => {
            panic!("Invalid command!");
        }
    }
}

fn tui_report(_user: u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["ledger"];
    let command: String = Select::new("What would you like to report:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "ledger" => {
            // println!("Balance of account: {}", _ledger.sum());
        }
        _ => {
            panic!("Invalid command!");
        }
    }
}

fn tui_view(_user: u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["ledger", "portfolio", "none"];
    let command: String = Select::new("What would you like to view:", commands)
        .prompt()
        .unwrap()
        .to_string();

    // match command.as_str() {
    //     "ledger" => {
    //         print_ledger(_user, _db);
    //     }
    //     _ => {
    //         panic!("Invalid command!");
    //     }
    // }
}
