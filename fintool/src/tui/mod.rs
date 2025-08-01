use chrono::Datelike;
use chrono::Days;
use chrono::Local;
use chrono::Months;
use std::collections::HashMap;
use std::vec;
use strum::IntoEnumIterator;

use crate::accounts::bank_account::BankAccount;
use crate::accounts::base::Account;
use crate::accounts::base::AccountCreation;
use crate::accounts::base::AccountOperations;
use crate::accounts::base::AnalysisPeriod;
use crate::accounts::certificate_of_deposit::CertificateOfDepositAccount;
use crate::accounts::credit_card_account::CreditCardAccount;
use crate::accounts::investment_account_manager::InvestmentAccountManager;
use crate::accounts::roth_ira::RothIraAccount;
use crate::accounts::wallet::Wallet;
use crate::database::DbConn;
use crate::tui::tui_user::*;
use crate::types::accounts::AccountType;
use crate::types::accounts::*;
use chrono::NaiveDate;
use inquire::*;

pub mod tui_accounts;
// pub mod tui_budgets;
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
    const ACCOUNT_OPTIONS: [&'static str; 4] =
        ["Create Account", "Select Account", "Edit Account", "Exit"];
    let mut accounts: Vec<AccountRecord> = db.get_user_accounts(uid).unwrap();
    let mut acct: Box<dyn Account>;
    let mut choice;
    let mut new_account;
    const ACCT_ACTIONS: [&'static str; 6] = ["Import", "Export", "Modify", "Record", "Report", "None"];

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
                let user_input = prompt_and_create_new_account(uid, db);
                if user_input.is_none() && accounts_is_empty {
                    break;
                } else if user_input.is_none() {
                    continue;
                }
                (acct, new_account) = user_input.unwrap();
                accounts.push(new_account);
                accounts_is_empty = false;
                // acct.record();
                const ACCT_ACTIONS: [&'static str; 3] = ["Record", "Import", "None"];
                let selected_menu_item = Select::new("Select action:", ACCT_ACTIONS.to_vec())
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
                acct = decode_and_init_account_type(uid, db, acctx);
                // acct.info();
            }
            "Edit Account" => {
                const MODIFY_ACCT_ACTIONS: [&'static str; 3] = ["Rename", "Remove", "None"];
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
                acct = decode_and_init_account_type(uid, db, acctx);

                let selected_action =
                    Select::new("What would you like to do:", MODIFY_ACCT_ACTIONS.to_vec())
                        .prompt()
                        .unwrap();

                let id = acct.get_id();

                match selected_action {
                    "Rename" => {
                        // acct
                        rename_account(db, uid, id);
                        return;
                    }
                    "Remove" => {
                        db.remove_account(uid, id).unwrap();
                        return;
                    }
                    "None" => {
                        continue;
                    }
                    _ => {
                        panic!("Unrecognized input: {}", selected_action);
                    }
                }
            }
            "Exit" => {
                return;
            }
            _ => {
                panic!("Invalid option!");
            }
        }

        loop {
            let selected_menu_item = Select::new("Select action:", ACCT_ACTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();
            match selected_menu_item.as_str() {
                "Import" => {
                    acct.import();
                }
                "Export" => { 
                    acct.export();
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

        accounts = db.get_user_accounts(uid).unwrap();
    }
}

pub fn decode_and_init_account_type(
    uid: u32,
    db: &DbConn,
    account: &AccountRecord,
) -> Box<dyn Account> {
    match account.info.atype {
        AccountType::Bank => Box::new(BankAccount::new(uid, account.id, db)),
        AccountType::Investment => Box::new(InvestmentAccountManager::new(uid, account.id, db)),
        AccountType::CreditCard => Box::new(CreditCardAccount::new(uid, account.id, db)),
        AccountType::CD => Box::new(CertificateOfDepositAccount::new(uid, account.id, db)),
        AccountType::Wallet => Box::new(Wallet::new(uid, account.id, db)),
        AccountType::RetirementRothIra => {Box::new(RothIraAccount::new(uid, account.id, db))}
    }
}

pub fn select_analysis_period() -> AnalysisPeriod {
    let period_choices = AnalysisPeriod::iter()
        .map(AnalysisPeriod::to_menu_selection)
        .collect::<Vec<String>>();
    let choice: String = Select::new("What period would you like to analyze:", period_choices)
        .prompt()
        .unwrap()
        .to_string();

    let duration = choice.parse().expect("Unrecognized analysis period!");
    return duration;
}

pub fn query_user_for_analysis_period(open_date : NaiveDate) -> (NaiveDate, NaiveDate, AnalysisPeriod) {
    let duration = select_analysis_period();
    let period_start;
    let period_end;
    match &duration {
        AnalysisPeriod::Custom => {
            period_start = DateSelect::new("Enter starting date").prompt().unwrap();
            period_end = DateSelect::new("Enter ending date").prompt().unwrap();
            // return (period_start, period_end, duration);
        }
        _ => {
            (period_start, period_end) = get_analysis_period_dates(open_date, &duration);
        }
    }
    return (period_start, period_end, duration);
}

pub fn get_analysis_period_dates(
    open_date : NaiveDate,
    duration: &AnalysisPeriod,
) -> (NaiveDate, NaiveDate) {
    let period_end = Local::now().date_naive();
    let mut period_start = period_end;

    match duration {
        AnalysisPeriod::OneDay => {
            period_start = period_start.checked_sub_days(Days::new(1)).unwrap();
        }
        AnalysisPeriod::OneWeek => {
            period_start = period_start.checked_sub_days(Days::new(7)).unwrap();
        }
        AnalysisPeriod::OneMonth => {
            period_start = period_start.checked_sub_months(Months::new(1)).unwrap();
        }
        AnalysisPeriod::ThreeMonths => {
            period_start = period_start.checked_sub_months(Months::new(3)).unwrap();
        }
        AnalysisPeriod::SixMonths => {
            period_start = period_start.checked_sub_months(Months::new(6)).unwrap();
        }
        AnalysisPeriod::OneYear => {
            period_start = period_start.with_year(period_start.year() - 1).unwrap();
        }
        AnalysisPeriod::TwoYears => {
            period_start = period_start.with_year(period_start.year() - 2).unwrap();
        }
        AnalysisPeriod::FiveYears => {
            // plus 1 accounts for leap year
            period_start = period_start.with_year(period_start.year() - 5).unwrap();
        }
        AnalysisPeriod::TenYears => {
            period_start = period_start.with_year(period_start.year() - 10).unwrap();
        }
        AnalysisPeriod::YTD => {
            // set as January 1st
            period_start = period_start.with_day(1).unwrap();
            period_start = period_start.with_month(1).unwrap();
        }
        AnalysisPeriod::AllTime => {
            period_start = open_date
        }
        _ => {
            panic!("Not found!");
        }
    }
    return (period_start, period_end);
}

pub fn prompt_and_create_new_account(
    uid: u32,
    db: &DbConn,
) -> (Option<(Box<dyn Account>, AccountRecord)>) {
    let mut account_types = AccountType::iter()
        .map(AccountType::to_menu_selection)
        .collect::<Vec<String>>();
    account_types.push("None".to_string());
    let selected_account_type = Select::new(
        "What account type would you like to create:",
        account_types.to_vec(),
    )
    .prompt()
    .unwrap()
    .to_string();

    if selected_account_type == "None".to_string() {
        return None;
    }

    let account_type: AccountType = selected_account_type
        .parse()
        .expect("Unrecognized account type! Must match the Account Type enumeration!");
    return create_account(uid, account_type, db);
}

pub fn create_account(
    uid: u32,
    atype: AccountType,
    db: &DbConn,
) -> (Option<(Box<dyn Account>, AccountRecord)>) {
    let name: String = name_account(uid, db);
    let new_account: AccountRecord;
    let acct: Box<dyn Account>;

    match atype {
        AccountType::Bank => {
            new_account = BankAccount::create(uid, name, db);
        }
        AccountType::Investment => {
            new_account = InvestmentAccountManager::create(uid, name, db);
        }
        AccountType::CreditCard => {
            new_account = CreditCardAccount::create(uid, name, db);
        }
        AccountType::CD => {
            new_account = CertificateOfDepositAccount::create(uid, name, db);
        }
        AccountType::Wallet => {
            new_account = Wallet::create(uid, name, db);
        }
        AccountType::RetirementRothIra => { 
            new_account = RothIraAccount::create(uid, name, db);
        }
    }

    acct = decode_and_init_account_type(uid, db, &new_account);

    return Some((acct, new_account));
}

pub fn name_account(uid: u32, db: &DbConn) -> String {
    let mut name;
    loop {
        name = Text::new("Enter account name:")
            .prompt()
            .unwrap()
            .trim()
            .to_string();

        if name.len() == 0 {
            println!("Invalid account name!");
        } else if db.account_with_name_exists(uid, name.clone()).unwrap() {
            println!("Account with name {} already exists!", name);
        } else {
            break;
        }
    }

    return name;
}

pub fn rename_account(db: &DbConn, uid: u32, id: u32) {
    let new_name = name_account(uid, db);
    db.rename_account(uid, id, new_name).unwrap();
}
