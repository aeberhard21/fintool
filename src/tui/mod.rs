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
        let commands: Vec<&str> = vec!["create", "change", "record", "report", "view", "exit"];
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
            "record" => {
                tui_record(uid, _db);
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
    let aid;
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
            aid = create_account(AccountType::Bank,_uid,  _db);
            let record = record_f32_amount(_uid, _db);
            _db.record_bank_account(aid, record);
        }
        "CD" => {
            aid = create_account(AccountType::CD, _uid, _db);
        }
        "health" => {
            aid = create_account(AccountType::Health, _uid, _db);
            let record = record_health_account(_uid, _db);
            _db.record_hsa_account(aid, record);

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

fn tui_record(_uid : u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["bank", "CD", "health", "investment", "ledger", "retirement", "none"];
    let command: String = Select::new("\nWhat would you like to add:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "bank" => {
            let aid = select_account(_uid, _db, AccountType::Bank);
            let record = record_f32_amount(_uid, _db);
            _db.record_bank_account(aid, record);
        }
        "CD" => {
            println!("Not implemented");
            // let aid = select_account(_uid, _db, AccountType::CD);
            // let record = record_f32_amount(_uid, _db);
            // _db.record_cd_account(aid, record);
        }
        "health" => {
            let aid = select_account(_uid, _db, AccountType::Health);
            let record = record_health_account(_uid, _db);
            _db.record_hsa_account(aid, record);
    
        }
        "investment" => {
            println!("Not implemented!");
        }
        "ledger" => {
            let aid = select_account(_uid, _db, AccountType::Ledger);
            loop {
                let entry = add_ledger(_uid, _db);
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
        "retirement" => {
            let aid = select_account(_uid, _db, AccountType::Retirement);
            let entry = record_f32_amount(_uid, _db);
        }
        "none" => {
            return;
        }
        _ => {
            panic!("Invalid command!");
        }
    }
}

fn tui_report(_uid: u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["bank", "ledger", "none"];
    let command: String = Select::new("What would you like to report:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "bank" => {
            let aid = select_account(_uid, _db, AccountType::Bank);
            let account = _db.get_account_name(_uid, aid).unwrap();
            let value = _db.get_bank_value(aid).unwrap().amount;
            println!("The value of account {} is: {}", &account, value )
        }
        "health" => {
            let aid = select_account(_uid, _db, AccountType::Health);
            let account = _db.get_account_name(_uid, aid).unwrap();
            let value = _db.get_bank_value(aid).unwrap().amount;
            println!("The value of account {} is: {}", &account, value )
        }
        "ledger" => {
            // println!("Balance of account: {}", _ledger.sum());
        }
        "none" => {
            return;
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
