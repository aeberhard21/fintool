// use core::panic;
use crate::database::DbConn;
use crate::ledger::Ledger;
use crate::tui::tui_ledger::*;
use crate::tui::tui_user::*;
use crate::user::User;
use chrono::{NaiveDate, Weekday};
use inquire::*;

mod tui_ledger;
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

pub fn menu(_user: &mut User, _db: &mut DbConn) {
    println!("Welcome {}!", _user.get_name());

    loop {
        let commands: Vec<&str> = vec!["create", "add", "report", "view", "exit"];
        let command: String = Select::new("What would you like to do:", commands)
            .prompt()
            .unwrap()
            .to_string();

        match command.as_str() {
            "create" => {
                tui_create(_user, _db);
            }
            // "login" => {
            //     current_user = tui_change(_users);
            // }
            "add" => {
                tui_add(_user, _db);
            }
            "view" => {
                tui_view(_user, _db);
            }
            "report" => {
                tui_report(_user, _db);
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

fn tui_create(_user: &mut User, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["ledger", "none"];
    let command: String = Select::new("\nWhat would you like to add:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "ledger" => {
            create_ledger(_user, _db);
        }
        "none" => return,
        _ => {
            panic!("Invalid command");
        }
    }
}

// fn tui_login() {

// }

fn tui_add(_user: &mut User, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["ledger", "investment", "none"];
    let command: String = Select::new("\nWhat would you like to add:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "ledger" => loop {
            add_ledger(_user, _db);

            let another: bool = Confirm::new("Add another entry?")
                .with_default(false)
                .prompt()
                .unwrap();
            if !another {
                break;
            }
        },
        "investment" => {
            println!("Not implemented!");
        }
        "none" => {
            return;
        }
        _ => {
            panic!("Invalid command!");
        }
    }
}

fn tui_report(_user: &mut User, _db: &mut DbConn) {
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

fn tui_view(_user: &mut User, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["ledger", "portfolio", "none"];
    let command: String = Select::new("What would you like to view:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "ledger" => {
            print_ledger(_user, _db);
        }
        _ => {
            panic!("Invalid command!");
        }
    }
}
