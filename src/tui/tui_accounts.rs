use core::panic;
use std::collections::HashMap;

use crate::database::db_accounts::AccountRecord;
use crate::database::db_accounts::AccountType;
use crate::database::db_banks::BankRecord;
use crate::database::db_cd::CdRecord;
use crate::database::db_hsa::HsaRecord;
use crate::database::db_investments::StockRecord;
use crate::database::{self, *};
use crate::stocks;
use chrono::{Date, NaiveDate, NaiveDateTime, Weekday};
use inquire::*;
use tokio::time::MissedTickBehavior;
use yahoo_finance_api::{YahooConnector, YahooError};

pub fn create_account(_atype: AccountType, _uid: u32, _db: &mut DbConn) -> u32 {
    let mut name: String = String::new();
    loop {
        name = Text::new("Enter account name:")
            .prompt()
            .unwrap()
            .to_string();
        if name.len() == 0 {
            println!("Invalid account name!")
        } else {
            break;
        }
    }
    let mut has_bank = false;
    let mut has_stocks = false;
    let mut has_ledger = false;
    match _atype {
        AccountType::Bank => {
            has_bank = true;
            has_stocks = false;
            has_ledger = false;
        }
        AccountType::CD => {
            has_bank = true;
            has_stocks = false;
            has_ledger = false;
        }
        AccountType::Investment => {
            has_bank = true;
            has_stocks = true;
            has_ledger = false;
        }
        AccountType::Ledger => {
            has_bank = false;
            has_stocks = false;
            has_ledger = true;
        }
        AccountType::Retirement => {
            has_bank = true;
            has_stocks = true;
            has_ledger = false;
        }
        AccountType::Health => {
            has_bank = true;
            has_stocks = true;
            has_ledger = false;
        }
        AccountType::Custom => {
            has_bank =
                Confirm::new("Would you like to associate a bank account with this account?")
                    .with_default(false)
                    .prompt()
                    .unwrap();
            has_stocks =
                Confirm::new("Would you like to associate investments with this accoutnt?")
                    .with_default(false)
                    .prompt()
                    .unwrap();
            has_ledger = Confirm::new("Would you like to associate a ledger with this account?")
                .with_default(false)
                .prompt()
                .unwrap();
        }
    }
    let account = AccountRecord {
        aid: None,
        atype: _atype,
        name: name,
        has_stocks: has_stocks,
        has_bank: has_bank,
        has_ledger: has_ledger,
    };
    _db.add_account(_uid, account).unwrap()
}

pub fn select_account(_uid: u32, _db: &mut DbConn, atype: AccountType) -> u32 {
    let msg;
    match &atype {
        &AccountType::Bank => msg = "Select bank account: ",
        &AccountType::CD => msg = "Select CD account: ",
        &AccountType::Health => msg = "Select health account: ",
        &AccountType::Ledger => msg = "Select ledger: ",
        &AccountType::Investment => msg = "Select investment account: ",
        &AccountType::Retirement => msg = "Select retirement account: ",
        _ => panic!("Unrecognized account type!"),
    }
    let accounts: Vec<String> = _db.get_user_accounts(_uid, atype).unwrap();
    let account: String = Select::new(msg, accounts).prompt().unwrap().to_string();
    let aid = _db.get_account_id(_uid, account).unwrap();
    return aid;
}

pub fn select_stock(tickers: Vec<String>) -> String {
    let stock: String = Select::new("Select stock: ", tickers)
        .prompt()
        .unwrap()
        .to_string();
    return stock;
}

pub fn record_f32_amount(_uid: u32, _db: &mut DbConn) -> BankRecord {
    let amount: Result<f32, InquireError> = CustomType::<f32>::new("Enter amount in account: ")
        .with_placeholder("00000.00")
        .with_default(00000.00)
        .with_error_message("Please type a valid amount!")
        .prompt();

    let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date").prompt();
    let date = &date_input
        .unwrap()
        .and_hms_milli_opt(0, 0, 0, 0)
        .unwrap()
        .timestamp();
    println!("The date is: {} ", &date);
    let converted_time = NaiveDateTime::from_timestamp(*date, 0).to_string();
    println!("The date is: {} ", converted_time);

    return BankRecord {
        amount: amount.unwrap(),
        date: *date,
    };
}

pub fn record_health_account(_uid: u32, _db: &mut DbConn) -> HsaRecord {
    let bank = record_f32_amount(_uid, _db);
    let add_stocks: bool = Confirm::new("Record stock purchase?")
        .with_default(false)
        .prompt()
        .unwrap();
    let mut stocks: Vec<StockRecord> = Vec::new();
    loop {
        if add_stocks {
            match record_stock_purchase(_uid) {
                Some(stock) => stocks.push(stock),
                None => {}
            }
        }
        let add_more: bool = Confirm::new("Add additional stock purchases?")
            .with_default(false)
            .prompt()
            .unwrap();
        if !add_more {
            break;
        }
    }
    return HsaRecord {
        fixed: bank,
        investments: stocks,
    };
}

pub fn record_stock_purchase(_uid: u32) -> Option<StockRecord> {
    let another = false;
    let mut ticker: String = String::new();
    loop {
        ticker = Text::new("Enter stock ticker: ")
            .prompt()
            .unwrap()
            .to_string();
        let rs = stocks::get_stock_at_close(ticker.clone());
        match rs {
            Ok(price) => {
                break;
            }
            Err(YahooError::FetchFailed(str)) => {
                if str == String::from("404 Not Found") {
                    let another: bool =
                        Confirm::new("Ticker not found. Would you like to try again?")
                            .with_default(false)
                            .prompt()
                            .unwrap();
                } else {
                    panic!("Fetch failed for ticker '{}': {}!", ticker.clone(), str);
                }
            }
            Err(error) => {
                panic!("Fetch failed for ticker '{}': {}!", ticker.clone(), error);
            }
        }

        if !another {
            return None;
        }
    }

    let date_input: Result<NaiveDate, InquireError> =
        DateSelect::new("Enter date of purchase").prompt();
    let date = &date_input
        .unwrap()
        .and_hms_milli_opt(0, 0, 0, 0)
        .unwrap()
        .timestamp();

    let shares: f32 = CustomType::<f32>::new("Enter number of shares purchased: ")
        .with_placeholder("0.00")
        .with_default(0.00)
        .with_error_message("Please enter a valid amount!")
        .prompt()
        .unwrap();

    let costbasis: f32 = CustomType::<f32>::new("Enter cost basis of shares purchased: ")
        .with_placeholder("0.00")
        .with_default(0.00)
        .with_error_message("Please enter a valid amount!")
        .prompt()
        .unwrap();

    return Some(StockRecord {
        date: Some(*date),
        ticker: ticker,
        shares: shares,
        costbasis: Some(costbasis),
    });
}

pub fn record_cd_account(_uid: u32) -> CdRecord {
    let principal = CustomType::<f32>::new("Enter principal amount: ")
        .with_placeholder("00000.00")
        .with_default(00000.00)
        .with_error_message("Please type a valid amount!")
        .prompt()
        .unwrap();

    let apy = CustomType::<f32>::new("Enter APY (%): ")
        .with_placeholder("0.00")
        .with_default(00000.00)
        .with_error_message("Please type a valid amount!")
        .prompt()
        .unwrap();

    let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date opened").prompt();
    let date = &date_input
        .unwrap()
        .and_hms_milli_opt(0, 0, 0, 0)
        .unwrap()
        .timestamp();

    let months = CustomType::<u32>::new("Enter term length (months): ")
        .with_placeholder("0")
        .with_default(0000000)
        .with_error_message("Please type a valid amount!")
        .prompt()
        .unwrap();

    return CdRecord {
        principal: principal,
        apy: apy,
        open_date: *date,
        length: months,
    };
}

pub fn get_total_of_stocks(aid: u32, _db: &mut DbConn, ticker: String) -> f64 {
    let cum = _db.cumulate_stocks(aid, ticker);
    return stocks::return_stock_values(cum);
}

pub fn get_net_wealth(uid: u32, _db: &mut DbConn) -> f64 {
    let mut nw: f64 = 0.0;
    let accounts = _db
        .get_user_account_info(uid)
        .expect("Unable to retrieve user accounts!");
    for account in accounts {
        if account.has_bank {
            nw += _db.get_bank_value(account.aid.expect("Account ID required!")).expect("Unable to retrieve bank account!").amount as f64
        }
        if account.has_stocks {
            nw += get_total_of_stocks(account.aid.expect("Account ID required!"), _db, database::SQLITE_WILDCARD.to_string());
        }
    }
    return nw;
}
