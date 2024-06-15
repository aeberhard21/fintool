use crate::database;
use crate::database::db_accounts::AccountFilter;
use crate::database::db_accounts::AccountType;
use crate::database::DbConn;
use crate::stocks;
use crate::tui::tui_accounts::*;
use crate::tui::tui_ledger::*;
use crate::tui::tui_user::*;
use inquire::*;

use self::tui_budgets::amend_budget;

mod tui_accounts;
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
            "bank",
            "CD",
            "health",
            "investment",
            "ledger",
            "retirement",
            "none",
        ];
    } else {
        commands = vec![
            "bank",
            "CD",
            "health",
            "investment",
            "ledger",
            "retirement",
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
        "bank" => {
            aid = create_account(AccountType::Bank, _uid, _db);
            let record = record_f32_amount(_uid, _db);
            _db.record_bank_account(aid, record)
                .expect("Unable to record bank account!");
        }
        "CD" => {
            aid = create_account(AccountType::CD, _uid, _db);
            let record = record_cd_account(_uid);
            _db.add_cd(aid, record).expect("Unable to add CD account!");
        }
        "health" => {
            aid = create_account(AccountType::Health, _uid, _db);
            let record = record_health_account(_uid, _db);
            _db.record_hsa_account(aid, record)
                .expect("Unable to record HSA account!");
        }
        "investment" => {
            aid = create_account(AccountType::Investment, _uid, _db);
            let cash = record_f32_amount(_uid, _db);
            _db.record_bank_account(aid, cash);
            loop {
                match record_stock_purchase(_uid) {
                    Some(record) => {
                        _db.add_stock(aid, record).expect("Unable to add stock!");
                    }
                    None => return,
                }
                let another: bool = Confirm::new("Add another stock to investment?")
                    .with_default(false)
                    .prompt()
                    .unwrap();
                if false == another {
                    break;
                }
            }
        }
        "ledger" => {
            aid = create_account(AccountType::Ledger, _uid, _db);
        }
        "retirement" => {
            create_account(AccountType::Retirement, _uid, _db);
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
    let commands: Vec<&str> = vec![
        "bank",
        "CD",
        "health",
        "investment",
        "ledger",
        "retirement",
        "none",
    ];
    let command: String = Select::new("\nWhat would you like to add:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "bank" => {
            let aid = select_account_by_type(_uid, _db, AccountType::Bank);
            let record = record_f32_amount(_uid, _db);
            _db.record_bank_account(aid, record);
        }
        "health" => {
            let aid = select_account_by_type(_uid, _db, AccountType::Health);
            let record = record_health_account(_uid, _db);
            _db.record_hsa_account(aid, record);
        }
        "investment" => {
            let aid = select_account_by_type(_uid, _db, AccountType::Investment);
            let report_bank: bool = Confirm::new("Record fixed cash account?")
                .with_default(false)
                .prompt()
                .unwrap();
            if report_bank == true {
                let cash = record_f32_amount(_uid, _db);
                _db.record_bank_account(aid, cash);
            }
            loop {
                match record_stock_purchase(_uid) {
                    Some(record) => {
                        _db.add_stock(aid, record);
                    }
                    None => return,
                }
                let another: bool = Confirm::new("Add another stock to investment?")
                    .with_default(false)
                    .prompt()
                    .unwrap();
                if false == another {
                    break;
                }
            }
            let insured_account = record_f32_amount(_uid, _db);
            _db.record_bank_account(aid, insured_account);
        }
        "ledger" => {
            let aid = select_account_by_type(_uid, _db, AccountType::Ledger);
            loop {
                let entry = add_ledger(_uid, _db);
                _db.add_ledger_entry(aid, entry);
                let another: bool = Confirm::new("Add another entry?")
                    .with_default(false)
                    .prompt()
                    .unwrap();
                if false == another {
                    break;
                }
            }
        }
        "retirement" => {
            let aid = select_account_by_type(_uid, _db, AccountType::Retirement);
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
    let commands: Vec<&str> = vec!["bank", "growth", "health", "investment", "ledger", "wealth", "none"];
    let command: String = Select::new("What would you like to report:", commands)
        .prompt()
        .unwrap()
        .to_string();
    let mut aid= 0;

    match command.as_str() {
        "bank" => {
            aid = select_account_by_type(_uid, _db, AccountType::Bank);
            let account = _db.get_account_name(_uid, aid).unwrap();
            let value = _db.get_bank_value(aid).unwrap().amount;
            println!("The value of account {} is: {}", &account, value)
        }
        "health" => {
            aid = select_account_by_type(_uid, _db, AccountType::Health);
            let account = _db.get_account_name(_uid, aid).unwrap();
            let acct = _db.get_hsa_value(aid).expect("Unable to get HSA account!");
            let mut total_investment_value = 0.0;
            for stock in acct.investments {
                total_investment_value += stocks::get_stock_at_close(stock.ticker)
                    .expect("Unable to retrieve stock value!")
                    * (stock.shares as f64);
            }
            let value = acct.fixed.amount as f64 + total_investment_value;
            println!("The value of account {} is: {}", &account, value);
        }
        "growth" => {
            let account_types = vec![
                "Bank",
                "CD",
                "Health",
                "Investment",
                "Ledger",
                "Retirement",
                "none",
            ];
            let selected_type: String = Select::new("What would you like to analyze:", account_types)
                .prompt()
                .unwrap()
                .to_string();

            if selected_type != "none" {
                let aid = select_account_by_type(_uid, _db, AccountType::from(selected_type));
                get_growth(aid,_db);
            }
        }
        "investment" => {
            let aid = select_account_by_type(_uid, _db, AccountType::Investment);
            let report_all = Confirm::new(
                "Report total of entire account (y) or an individual stock ticker (n)",
            )
            .with_default(false)
            .prompt()
            .unwrap();
            let mut ticker = database::SQLITE_WILDCARD.to_string();
            if !report_all {
                ticker = select_stock(
                    _db.get_stock_tickers(aid)
                        .expect("Unable to retrieve stock tickers for this account!"),
                );
            }
            println!(
                "Value at last closing: {}",
                get_total_of_stocks(aid, _db, ticker)
            );
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
