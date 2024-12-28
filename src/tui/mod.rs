use core::num;
use chrono::Datelike;
use chrono::Days;
use chrono::FixedOffset;
use chrono::Local;
use chrono::Months;
use time::convert::Day;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::format;
use std::hash::Hash;
use std::process::CommandArgs;
use std::vec;

use crate::accounts::bank_account;
use crate::accounts::bank_account::BankAccount;
use crate::accounts::investment_account_manager::InvestmentAccountManager;
use crate::database;
// use crate::database::db_accounts::AccountFilter;
// use crate::database::db_accounts::AccountType;
use crate::types::accounts::*;
use crate::types::investments::StockInfo;
use crate::types::investments::StockRecord;
// use crate::database::db_ledger::LedgerInfo;
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
use chrono::NaiveDateTime;
use chrono::Utc;
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

    let mut menu_options: Vec<&str> = Vec::new();
    if _db.is_admin(uid).unwrap() { 
        menu_options = vec!["Create User", "Change User", "Access Account(s)", "Exit"];
    } else { 
        menu_options = vec!["Change User", "Account Operations", "Exit"];
    }

    let rf = &menu_options;

    loop {

        let command: String = Select::new("What would you like to do:",rf.to_vec())
            .prompt()
            .unwrap()
            .to_string();

        match command.as_str() {
            "Create User" => {
                create_user(_db);
            }
            "Change User" => {
                uid = tui_set_user(_db);
            }
            "Access Account(s)" => {
                access_account(uid, _db);
            }
            "Exit" => {
                println!("Exiting...");
                break;
            }
            _ => {
                panic!("Invalid command.");
            }
        }
    }
}

fn access_account(uid : u32, db : &mut DbConn) {
    const ACCOUNT_OPTIONS : [&'static str; 3] = ["Create Account", "Select Account", "Exit"];
    let mut accounts: Vec<AccountRecord> = db.get_user_account_info(uid).unwrap();
    let mut acct: Box<dyn AccountOperations>;
    let mut choice;
    let mut new_account;
    const ACCT_ACTIONS : [&'static str; 3] = ["Record", "Report", "None"];

    let mut accounts_is_empty = accounts.is_empty();

    loop {

        if accounts_is_empty { 
            choice = ACCOUNT_OPTIONS[0].to_string();
        } else {
            choice = Select::new("What would you like to do:", ACCOUNT_OPTIONS.to_vec()).prompt().unwrap().to_string();
        }

        match choice.as_str() {
            "Create Account" => {

                (acct, new_account) = create_new_account(uid, db);
                accounts.push( new_account );
                accounts_is_empty = false;
                acct.record();

                let more = Confirm::new("More actions?").prompt().unwrap();
                if !more { 
                    continue;
                }
            }
            "Select Account" => { 

                let mut account_map : HashMap<String, AccountRecord> = HashMap::new();
                let mut account_names : Vec<String> = Vec::new();
                for account in accounts.iter() { 
                    account_names.push(account.info.name.clone());
                    account_map.insert(account.info.name.clone(), account.clone());
                }

                // add none clause
                account_names.push("None".to_string());
                let selected_account = Select::new("Select account:", account_names).prompt().unwrap().to_string();

                if selected_account == "None" {
                    continue;
                }

                let acctx = account_map.get(&selected_account).expect("Account not found!");
                acct = decode_and_create_account_type(uid, db, acctx);
                // acct.info();                
            }
            "Exit" => {
                return;
            }
            _ => {
                panic!("Invalid option!");
            }
        }

        let selected_menu_item = Select::new("Select action: ", ACCT_ACTIONS.to_vec()).prompt().unwrap().to_string();
        match selected_menu_item.as_str() { 
            "Record" => {
                acct.record();
            }
            "Report" => {
                acct.report();
            }
            "None" => { 
                continue;
            }
            _ => { 
                panic!("Invalid menu option!");
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
            // let bank = BankAccount::create(aid, _db);
        }
        "Investment" => {
            aid = create_account(AccountType::from(command), _uid, _db);
            // let investment_account = InvestmentAccountManager::create(aid, _db);
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
                let mut bank_account : BankAccount = BankAccount::new(_uid, aid, _db);
                bank_account.record();
            }
            "Retirement"|"Investment"|"Health" => {
                let (aid, account) = select_account_by_type(_uid, _db, AccountType::from(command));
                let mut investment_account : InvestmentAccountManager = InvestmentAccountManager::new(_uid, aid, _db);
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
            let mut acct = InvestmentAccountManager::new(_uid, aid, _db);
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
    // fn create( account_id : u32, db : &mut DbConn );
    fn record( &mut self );
    fn modify( &mut self );
    fn export( &mut self );
    fn report( &mut self );
    fn link (&mut self, transacting_account : u32, ledger : LedgerRecord) -> Option<u32>;
}

pub fn decode_and_create_account_type(uid : u32, db : & mut DbConn, account : &AccountRecord) -> Box<dyn AccountOperations> {
    match account.info.atype {
        AccountType::Bank => {
            Box::new(BankAccount::new(uid, account.id, db))
        }
        AccountType::Investment => {
            println!("Here");
            Box::new(InvestmentAccountManager::new(uid, account.id, db))
        }
        _ => {
            panic!("Invalid account type!");
        }
    }   
}

pub trait AccountCreation { 
    fn create() -> AccountInfo;
}

pub fn query_user_for_analysis_period() -> (NaiveDate, NaiveDate) { 
    
    const PERIOD_CHOICES: [&'static str; 10] = ["1 Day", "1 Week", "1 Month", "3 Months", "6 Months", "1 Year", "2 Year", "10 Year", "YTD", "Custom" ];
    let choice: String = Select::new("What period would you like to analyze:", PERIOD_CHOICES.to_vec())
        .prompt()
        .unwrap()
        .to_string();

    // let period_end = Utc::now().naive_local().and_local_timezone(FixedOffset::west_opt(5  * 3600).unwrap())
    // println!("Today's date: {}", period_end);
    // let mut period_start = period_end;
    let mut period_end = Local::now().date_naive();
    let mut period_start = period_end;

    match choice.as_str() {
        "1 Day" => {
            period_start = period_start.checked_sub_days(Days::new(1)).unwrap();
        },
        "1 Week" => {
            period_start = period_start.checked_sub_days(Days::new(7)).unwrap();
        },
        "1 Month" => {
            period_start = period_start.checked_sub_months(Months::new(1)).unwrap();
        },
        "3 Months" => {
            period_start = period_start.checked_sub_months(Months::new(3)).unwrap();
        },
        "6 Months" => {
            period_start = period_start.checked_sub_months(Months::new(6)).unwrap();
        },
        "1 Year" => {
            period_start = period_start.with_year(period_start.year()-1).unwrap();
        },
        "2 Year" => {
            period_start = period_start.with_year(period_start.year()-2).unwrap();
        },
        "5 Year" => {
            // plus 1 accounts for leap year
            period_start = period_start.with_year(period_start.year()-5).unwrap();
        },
        "10 Year" => {
            period_start = period_start.with_year(period_start.year()-10).unwrap();
        }
        "YTD" => {
            // set as January 1st
            period_start = period_start.with_day(1).unwrap();
            period_start = period_start.with_month(1).unwrap();
        },
        "Custom" | _ => {
            period_end = DateSelect::new("Enter ending date").prompt().unwrap();
            period_start = DateSelect::new("Enter starting date").prompt().unwrap();
        }
        _ => {
            panic!("Not found!");
        }
    }
    return (period_start, period_end);
}

pub fn create_new_account(uid : u32, db : &mut DbConn) -> (Box<dyn AccountOperations>, AccountRecord) { 
    const ACCOUNT_TYPES : [&'static str; 2] = ["Bank Account", "Investment Account"];
    let selected_account_type = Select::new("What account type would you like to create: ", ACCOUNT_TYPES.to_vec()).prompt().unwrap().to_string();
    let mut id;
    let mut new_account;
    let mut acct : Box<dyn AccountOperations>;
    match selected_account_type.as_str() { 
        "Bank Account" => {
            new_account = BankAccount::create();
            id = db.add_account(uid, &new_account).unwrap();
            acct = Box::new(BankAccount::new(uid, id, db));

        }
        "Investment Account" => {
            new_account = InvestmentAccountManager::create();
            id = db.add_account(uid, &new_account).unwrap();
            acct = Box::new(InvestmentAccountManager::new(uid, id, db));
        }
        _ => {
            panic!("Unrecognized input!");
        }
    }

    return (acct, AccountRecord { id : id, info: new_account} );
}