use std::borrow::BorrowMut;
use std::sync::Arc;

// use core::panic;
use crate::database::DbConn;
use crate::database::db_accounts::AccountType;
use crate::tui::tui_ledger::*;
use crate::tui::tui_user::*;
use crate::tui::tui_accounts::*;
use crate::stocks;
use chrono::{NaiveDate, Weekday};
use inquire::*;
use tokio::task::spawn_blocking;
use yahoo::YahooConnector;
use yahoo_finance_api as yahoo;
use tokio;

mod tui_ledger;
mod tui_accounts;
pub mod tui_user;

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
            let aid = select_account(_uid, _db, AccountType::Investment);
            loop {
                match record_stock_purchase(_uid) {
                    Some(record) => {
                       _db.add_stock(aid, record);
                   }
                   None => {
                       return
                   }
               }
               let another: bool = Confirm::new("Add another stock to investment?")
                    .with_default(false)
                    .prompt()
                    .unwrap(); 
                if false == another {
                    break
                }
            }           
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
    let commands: Vec<&str> = vec!["bank", "health", "investment", "ledger", "none"];
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
        "investment" => {
            let aid = select_account(_uid, _db, AccountType::Investment);
            let mut stocks = _db.get_stocks(aid).expect("Unable to retrieve account information!");
            let report_all = Confirm::new("Report total of entire account (y) or stocks (n)").with_default(false).prompt().unwrap();
            if !report_all {
                let tmp = stocks.clone();
                stocks.clear();
                stocks.push(select_stock(tmp));
            }
            let mut value: f64 = 0.0;
            for stock in stocks {
                let record = _db.get_stock_info(aid, stock).unwrap();
                for r in record {
                    value += stocks::get_stock_at_close(r.ticker).unwrap() * r.shares as f64;
                }
            }
            println!("Value at last closing: {}", value);

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
