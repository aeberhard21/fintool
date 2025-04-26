use chrono::Datelike;
use chrono::Days;
use chrono::Local;
use chrono::Months;
use std::collections::HashMap;
use std::vec;

use crate::accounts::bank_account::BankAccount;
use crate::accounts::base::AccountCreation;
use crate::accounts::base::AccountOperations;
use crate::accounts::investment_account_manager::InvestmentAccountManager;
use crate::database::DbConn;
use crate::tui::tui_user::*;
use crate::types::accounts::*;
use chrono::NaiveDate;
use inquire::*;

pub mod tui_accounts;
pub mod tui_budgets;
pub mod tui_user;

pub fn menu(_db: &mut DbConn) {
    let mut uid: u32;

    // set current user first!
    uid = tui_set_user(_db);

    let menu_options: Vec<&str>;
    if _db.is_admin(uid).unwrap() {
        menu_options = vec!["Create User", "Change User", "Access Account(s)", "Exit"];
    } else {
        menu_options = vec!["Change User", "Access Account(s)", "Exit"];
    }

    let rf = &menu_options;

    loop {
        let command: String = Select::new("What would you like to do:", rf.to_vec())
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

fn access_account(uid: u32, db: &mut DbConn) {
    const ACCOUNT_OPTIONS: [&'static str; 3] = ["Create Account", "Select Account", "Exit"];
    let mut accounts: Vec<AccountRecord> = db.get_user_accounts(uid).unwrap();
    let mut acct: Box<dyn AccountOperations>;
    let mut choice;
    let mut new_account;
    const ACCT_ACTIONS: [&'static str; 5] = ["Import", "Modify", "Record", "Report", "None"];

    let mut accounts_is_empty = accounts.is_empty();

    loop {
        if accounts_is_empty {
            choice = ACCOUNT_OPTIONS[0].to_string();
        } else {
            choice = Select::new("What would you like to do:", ACCOUNT_OPTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();
        }

        match choice.as_str() {
            "Create Account" => {
                let user_input = create_new_account(uid, db);
                if user_input.is_none() { 
                    continue;
                }
                (acct, new_account) = user_input.unwrap();
                accounts.push(new_account);
                accounts_is_empty = false;
                // acct.record();
                const ACCT_ACTIONS: [&'static str; 3] = ["Record", "Import", "None"];
                let selected_menu_item = Select::new("Select action: ", ACCT_ACTIONS.to_vec())
                    .prompt()
                    .unwrap()
                    .to_string();
                match selected_menu_item.as_str() {
                    "Record" => {
                        acct.record();
                    }
                    "Import" => {
                        acct.import();
                    }
                    "None" => {
                        continue;
                    }
                    _ => {
                        panic!("Invalid menu option!");
                    }
                }

                let more = Confirm::new("More actions?").prompt().unwrap();
                if !more {
                    continue;
                }
            }
            "Select Account" => {
                let mut account_map: HashMap<String, AccountRecord> = HashMap::new();
                let mut account_names: Vec<String> = Vec::new();
                for account in accounts.iter() {
                    account_names.push(account.info.name.clone());
                    account_map.insert(account.info.name.clone(), account.clone());
                }

                // add none clause
                account_names.push("None".to_string());
                let selected_account = Select::new("Select account:", account_names)
                    .prompt()
                    .unwrap()
                    .to_string();

                if selected_account == "None" {
                    continue;
                }

                let acctx = account_map
                    .get(&selected_account)
                    .expect("Account not found!");
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

        loop {
            let selected_menu_item = Select::new("Select action: ", ACCT_ACTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();
            match selected_menu_item.as_str() {
                "Import" => {
                    acct.import();
                }
                "Modify" => {
                    acct.modify();
                }
                "Record" => {
                    acct.record();
                }
                "Report" => {
                    acct.report();
                }
                "None" => {
                    break;
                }
                _ => {
                    panic!("Invalid menu option!");
                }
            }
            let more = Confirm::new("More actions?").prompt().unwrap();
            if !more {
                break;
            }
        }
    }
}

pub fn decode_and_create_account_type(
    uid: u32,
    db: &mut DbConn,
    account: &AccountRecord,
) -> Box<dyn AccountOperations> {
    match account.info.atype {
        AccountType::Bank => Box::new(BankAccount::new(uid, account.id, db)),
        AccountType::Investment => Box::new(InvestmentAccountManager::new(uid, account.id, db)),
        _ => {
            panic!("Invalid account type!");
        }
    }
}

pub fn query_user_for_analysis_period() -> (NaiveDate, NaiveDate) {
    const PERIOD_CHOICES: [&'static str; 10] = [
        "1 Day", "1 Week", "1 Month", "3 Months", "6 Months", "1 Year", "2 Year", "10 Year", "YTD",
        "Custom",
    ];
    let choice: String = Select::new(
        "What period would you like to analyze:",
        PERIOD_CHOICES.to_vec(),
    )
    .prompt()
    .unwrap()
    .to_string();

    let mut period_end = Local::now().date_naive();
    let mut period_start = period_end;

    match choice.as_str() {
        "1 Day" => {
            period_start = period_start.checked_sub_days(Days::new(1)).unwrap();
        }
        "1 Week" => {
            period_start = period_start.checked_sub_days(Days::new(7)).unwrap();
        }
        "1 Month" => {
            period_start = period_start.checked_sub_months(Months::new(1)).unwrap();
        }
        "3 Months" => {
            period_start = period_start.checked_sub_months(Months::new(3)).unwrap();
        }
        "6 Months" => {
            period_start = period_start.checked_sub_months(Months::new(6)).unwrap();
        }
        "1 Year" => {
            period_start = period_start.with_year(period_start.year() - 1).unwrap();
        }
        "2 Year" => {
            period_start = period_start.with_year(period_start.year() - 2).unwrap();
        }
        "5 Year" => {
            // plus 1 accounts for leap year
            period_start = period_start.with_year(period_start.year() - 5).unwrap();
        }
        "10 Year" => {
            period_start = period_start.with_year(period_start.year() - 10).unwrap();
        }
        "YTD" => {
            // set as January 1st
            period_start = period_start.with_day(1).unwrap();
            period_start = period_start.with_month(1).unwrap();
        }
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

pub fn create_new_account(
    uid: u32,
    db: &mut DbConn,
) -> (Option<(Box<dyn AccountOperations>, AccountRecord)>) {
    const ACCOUNT_TYPES: [&'static str; 3] = ["Bank Account", "Investment Account", "None"];
    let selected_account_type = Select::new(
        "What account type would you like to create:",
        ACCOUNT_TYPES.to_vec(),
    )
    .prompt()
    .unwrap()
    .to_string();

    let id;
    let new_account;
    let acct: Box<dyn AccountOperations>;

    if selected_account_type == "None" {
        return None;
    }

    let mut name: String;
    loop { 
        name = Text::new("Enter account name:")
        .prompt()
        .unwrap()
        .to_string();

        if name.len() == 0 { 
            println!("Invalid account name!");
        } else if db.account_with_name_exists(uid, name.clone()).unwrap() { 
            println!("Account with name {} already exists!", name);
        } else { 
            break;
        }

        let try_again = Confirm::new("Try again?")
            .prompt()
            .unwrap();
        if !try_again {
            return None; 
        }
    }

    match selected_account_type.as_str() {
        "Bank Account" => {
            new_account = BankAccount::create(name);
            id = db.add_account(uid, &new_account).unwrap();
            acct = Box::new(BankAccount::new(uid, id, db));
        }
        "Investment Account" => {
            new_account = InvestmentAccountManager::create(name);
            id = db.add_account(uid, &new_account).unwrap();
            acct = Box::new(InvestmentAccountManager::new(uid, id, db));
        }
        _ => {
            panic!("Unrecognized input!");
        }
    }

    return Some((
        acct,
        AccountRecord {
            id: id,
            info: new_account,
        },)
    );
}
