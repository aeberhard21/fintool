use core::num;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::format;
use std::hash::Hash;
use std::process::CommandArgs;

use crate::accounts::bank_account;
use crate::accounts::bank_account::BankAccount;
use crate::accounts::investment_account_manager::InvestmentAccountManager;
use crate::database;
// use crate::database::db_accounts::AccountFilter;
// use crate::database::db_accounts::AccountType;
use crate::types::accounts::*;
use crate::types::investments::StockRecord;
use crate::types::investments::StockEntries;
// use crate::database::db_ledger::LedgerEntry;
// use crate::database::db_ledger::TransferType;
use crate::types::ledger::*;
use crate::types::transfer_types::TransferType;
use crate::types::participants::ParticipantType;
use crate::database::DbConn;
// use crate::ledger;
// use crate::ledger::Ledger;
use crate::types::ledger;
use crate::stocks;
use crate::tui::tui_accounts::*;
use crate::tui::tui_ledger::*;
use crate::tui::tui_user::*;
use chrono::NaiveDate;
use inquire::*;
use tokio::runtime::EnterGuard;
use yahoo_finance_api::Dividend;

use self::tui_budgets::amend_budget;

pub mod tui_accounts;
mod tui_ledger;
pub mod tui_budgets;
pub mod tui_user;

pub fn menu(_db: &mut DbConn) {
    let mut uid: u32;

    // set current user first!
    uid = tui_set_user(_db);

    loop {
        let commands: Vec<&str> = vec!["create", "change", "modify", "record", "report", "view", "exit"];
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
            "modify" => {
                tui_modify(uid, _db);
            }
            "report" => {
                tui_report(uid, _db);
            }
            "view" => {
                tui_view(uid, _db);
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
    let mut commands: Vec<&str> = Vec::new();
    let aid;
    if _db.is_admin(_uid).unwrap() {
        commands = vec![
            "user",
            "Bank",
            "Investment",
            "none",
        ];
    } else {
        commands = vec![
            "Bank",
            "CD",
            "Health",
            "Investment",
            "Retirement",
            "none",
        ];
    }
    let command: String = Select::new("\nWhat would you like to add:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "user" => {
            create_user(_db);
        }
        "Bank" => {
            aid = create_account(AccountType::from(command), _uid, _db);
            let bank = BankAccount::create(aid, _db);
        }
        "Investment" => {
            aid = create_account(AccountType::from(command), _uid, _db);
            let investment_account = InvestmentAccountManager::create(aid, _db);
        }
        "none" => return,
        _ => {
            panic!("Invalid command");
        }
    }
}

fn tui_modify(_uid: u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec![
        "budget",
        "none",
    ];
    let command: String = Select::new("\nWhat would you like to modify:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "budget" => {
            let aid = select_account_by_filter(_uid, _db, AccountFilter::Budget);
            amend_budget(aid, _db);
        }
        "none" => {
            return
        }
        _ => {
            panic!("Invalid command!");
        }
    }
    return;

}

fn tui_record(_uid: u32, _db: &mut DbConn) {
    loop {
        let commands: Vec<&str> = vec![
            "Bank",
            "Investment",
            "none",
        ];

        let command: String = Select::new("\nWhat would you like to add:", commands)
            .prompt()
            .unwrap()
            .to_string();

        match command.as_str() {
            "Bank" => {
                let (aid, account) = select_account_by_type(_uid, _db, AccountType::Bank);
                let mut bank_account : BankAccount = BankAccount::new(aid, _db);
                bank_account.record();
            }
            "Retirement"|"Investment"|"Health" => {
                let (aid, account) = select_account_by_type(_uid, _db, AccountType::from(command));
                let mut investment_account : InvestmentAccountManager = InvestmentAccountManager::new(aid, _db);
                investment_account.record();
            }
            "none" => {
                return;
            }
            _ => {
                panic!("Invalid command!");
            }
        }
    }
}

fn tui_report(_uid: u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["bank", "growth", "health", "investment", "ledger", "wealth", "none"];
    let command: String = Select::new("What would you like to report:", commands)
        .prompt()
        .unwrap()
        .to_string();
    let mut aid= 0;
    let account : String;

    match command.as_str() {
        "bank" => {
            (aid, account) = select_account_by_type(_uid, _db, AccountType::Bank);
            let value = _db.get_bank_value(aid).unwrap().amount;
            println!("The value of account {} is: {}", &account, value)
        }
        "health" => {
            // (aid, account) = select_account_by_type(_uid, _db, AccountType::Health);
            // let account = _db.get_account_name(_uid, aid).unwrap();
            // let acct = _db.get_hsa_value(aid).expect("Unable to get HSA account!");
            // let mut total_investment_value = 0.0;
            // for stock in acct.investments {
            //     total_investment_value += stocks::get_stock_at_close(stock.ticker)
            //         .expect("Unable to retrieve stock value!")
            //         * (stock.shares as f64);
            // }
            // let value = acct.fixed.amount as f64 + total_investment_value;
            // println!("The value of account {} is: {}", &account, value);
        }
        "growth" => {
            let account_types = vec![
                "Bank",
                "CD",
                "Health",
                "Investment",
                "Retirement",
                "none",
            ];
            let selected_type: String = Select::new("What would you like to analyze:", account_types)
                .prompt()
                .unwrap()
                .to_string();

            if selected_type != "none" {
                let (aid, account) = select_account_by_type(_uid, _db, AccountType::from(selected_type));
                get_growth(aid,_db);
            }
        }
        "investment" => {
            let (aid, account) = select_account_by_type(_uid, _db, AccountType::Investment);
            let report_all = Confirm::new(
                "Report total of entire account (y) or an individual stock ticker (n)",
            )
            .with_default(false)
            .prompt()
            .unwrap();
            let mut acct = InvestmentAccountManager::new(aid, _db);
            acct.report();
            
        }
        "ledger" => {
            // println!("Balance of account: {}", _ledger.sum());
        }
        "wealth" => {
            println!("Net wealth: {}", get_net_wealth(_uid, _db));
        }
        "none" => {
            return;
        }
        _ => {
            panic!("Invalid command!");
        }
    }

    // let acct = _db.get_account(aid).expect("Unable to retrieve user account!");
    // if acct.has

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

pub trait AccountOperations {
    fn create( account_id : u32, db : &mut DbConn );
    fn record( &mut self );
    fn modify( &mut self );
    fn export( &mut self );
    fn report( &mut self );
}