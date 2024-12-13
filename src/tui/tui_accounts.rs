use core::panic;
use std::collections::HashMap;

// use crate::database::db_accounts::AccountRecord;
// use crate::database::db_accounts::AccountType;
use crate::types::accounts::*;
use crate::database::db_banks::BankRecord;
use crate::database::db_cd::CdRecord;
// use crate::database::db_hsa::HsaRecord;
use crate::types::investments::StockRecord;
use crate::database::{self, *};
use crate::tui::tui_budgets::create_budget;
use crate::stocks;
use chrono::Month;
use chrono::Offset;
use chrono::TimeZone;
use chrono::{Date, NaiveDate, NaiveTime, NaiveDateTime, Weekday};
use inquire::*;
use tokio::time::MissedTickBehavior;
use yahoo_finance_api::{YahooConnector, YahooError};
use chrono::{Datelike, Days, Local, Utc};
use time::{Duration, OffsetDateTime};

// use self::db_accounts::AccountFilter;

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
    let mut has_budget = false;
    match _atype {
        AccountType::Bank => {
            has_bank = true;
        }
        AccountType::CD => {
            has_bank = true;
        }
        AccountType::Investment => {
            has_bank = true;
            has_stocks = true;
        }
        AccountType::Ledger => {
            has_ledger = true;
            has_budget = 
                Confirm::new("Would you like to associate a budget with this ledger?" )
                    .with_default(false)
                    .prompt()
                    .unwrap();
        }
        AccountType::Retirement => {
            has_bank = true;
            has_stocks = true;
        }
        AccountType::Health => {
            has_bank = true;
            has_stocks = true;
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
            has_budget = Confirm::new("Would you like to associate a budget with this account?")
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
        has_budget: has_budget
    };
    let aid = _db.add_account(_uid, account).unwrap();

    if has_budget {
        println!("Creating budget...");
        create_budget(aid, _db);
    }

    return aid;
}

pub fn select_account_by_type(_uid: u32, _db: &mut DbConn, atype: AccountType) -> (u32, String) {
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
    let accounts: Vec<String> = _db.get_user_accounts_by_type(_uid, atype).unwrap();
    for account in accounts.clone() { 
        println!("Here are the accounts {}", account);
    }
    let account: String = Select::new(msg, accounts).prompt().unwrap().to_string();
    let aid = _db.get_account_id(_uid, account.clone()).unwrap();
    return (aid, account);
}

pub fn select_account_by_filter(_uid: u32, _db: &mut DbConn, filter: AccountFilter) -> u32 {
    let msg;
    match &filter {
        &AccountFilter::Bank => msg = "Select bank account: ",
        &AccountFilter::Stocks => msg = "Select investment account: ",
        &AccountFilter::Ledger => msg = "Select ledger account: ",
        &AccountFilter::Budget => msg = "Select budget account: ",
        _ => panic!("Unrecognized account type!"),
    }
    let accounts: Vec<String> = _db.get_user_accounts_by_filter(_uid, filter).unwrap();
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

pub fn record_f32_amount() -> BankRecord {
    let amount: Result<f32, InquireError> = CustomType::<f32>::new("Enter amount in account: ")
        .with_placeholder("00000.00")
        .with_default(00000.00)
        .with_error_message("Please type a valid amount!")
        .prompt();

    let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date").prompt();

    return BankRecord {
        amount: amount.unwrap(),
        // date: *date,
        date: date_input.unwrap().to_string()
    };
}

// pub fn record_health_account(_uid: u32, _db: &mut DbConn) -> HsaRecord {
//     let bank = record_f32_amount(_uid, _db);
//     let add_stocks: bool = Confirm::new("Record stock purchase?")
//         .with_default(false)
//         .prompt()
//         .unwrap();
//     let mut stocks: Vec<StockRecord> = Vec::new();
//     loop {
//         if add_stocks {
//             match record_stock_purchase(_uid) {
//                 Some(stock) => stocks.push(stock),
//                 None => {}
//             }
//         }
//         let add_more: bool = Confirm::new("Add additional stock purchases?")
//             .with_default(false)
//             .prompt()
//             .unwrap();
//         if !add_more {
//             break;
//         }
//     }
//     return HsaRecord {
//         fixed: bank,
//         investments: stocks,
//     };
// }

pub fn record_stock_purchase() -> Option<StockRecord> {
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

    let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date of purchase").prompt();

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
        date: date_input.unwrap().to_string(),
        ticker: ticker,
        shares: shares,
        costbasis: costbasis,
        remaining: shares
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
    // let date = &date_input
    //     .unwrap()
    //     .and_hms_milli_opt(0, 0, 0, 0)
    //     .unwrap()
    //     .timestamp();

    let months = CustomType::<u32>::new("Enter term length (months): ")
        .with_placeholder("0")
        .with_default(0000000)
        .with_error_message("Please type a valid amount!")
        .prompt()
        .unwrap();

    return CdRecord {
        principal: principal,
        apy: apy,
        open_date: date_input.unwrap().to_string(),
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

pub fn get_growth(aid: u32, _db: &mut DbConn) {
    let periods: Vec<&str> = vec!["1 day", "1 Week", "1 Month", "3 Months", "6 Months", "1 Year", "2 Year", "10 Year", "YTD", "Custom" ];
    let command: String = Select::new("What period would you like to analyze:", periods)
        .prompt()
        .unwrap()
        .to_string();

    let period_end = OffsetDateTime::from_unix_timestamp(Utc::now().timestamp()).unwrap();
    let mut period_start = period_end;

    match command.as_str() {
        "1 Day" => {
            period_start = period_start.checked_sub(Duration::days(1)).unwrap();
        }
        "1 Week" => {
            period_start = period_start.checked_sub(Duration::days(7)).unwrap();
        }
        "1 Month" => {
            let mut year = period_end.year();
            let mut month = period_end.month() as i32 - 1;
            let mut day = period_end.day();
            month -= 1;
            while month < 0 {
                year -= 1;
                month += 12;
            }
            let month_as_enum = time::Month::try_from((month+1) as u8).ok().unwrap();
            let last_day_of_month = time::util::days_in_year_month(year, month_as_enum) as i32;
            day = if day > last_day_of_month as u8 {
                last_day_of_month as u8
            } else {
                day
            };

            period_start = period_start.replace_year(year).unwrap();
            period_start = period_start.replace_month(month_as_enum).unwrap();
            period_start = period_start.replace_day(day).unwrap();
        }
        "3 Months" => {
            let mut year = period_end.year();
            let mut month = period_end.month() as i32 - 1;
            let mut day = period_end.day();
            month -= 3;
            while month < 0 {
                year -= 1;
                month += 12;
            }
            let month_as_enum = time::Month::try_from((month+1) as u8).ok().unwrap();
            let last_day_of_month = time::util::days_in_year_month(year, month_as_enum) as i32;
            day = if day > last_day_of_month as u8 {
                last_day_of_month as u8
            } else {
                day
            };

            period_start = period_start.replace_year(year).unwrap();
            period_start = period_start.replace_month(month_as_enum).unwrap();
            period_start = period_start.replace_day(day).unwrap();
        }
        "6 Months" => {
            let mut year = period_end.year();
            let mut month = period_end.month() as i32 - 1;
            let mut day = period_end.day();
            month -= 6;
            while month < 0 {
                year -= 1;
                month += 12;
            }
            let month_as_enum = time::Month::try_from((month+1) as u8).ok().unwrap();
            let last_day_of_month = time::util::days_in_year_month(year, month_as_enum) as i32;
            day = if day > last_day_of_month as u8 {
                last_day_of_month as u8
            } else {
                day
            };

            period_start = period_start.replace_year(year).unwrap();
            period_start = period_start.replace_month(month_as_enum).unwrap();
            period_start = period_start.replace_day(day).unwrap();        
        }
        "1 Year" => {
            let month = period_start.month();
            let year = period_start.year();
            let day = period_start.day();
            if month == time::Month::February && time::util::is_leap_year(year) && day == 29 {
                // this handles the case of leap day
                period_start = period_start.replace_month(time::Month::March).unwrap();
                period_start = period_start.replace_day(1).unwrap();
            }
            period_start = period_start.replace_year(period_start.year()-1).unwrap();
        }
        "2 Year" => {
            let month = period_start.month();
            let year = period_start.year();
            let day = period_start.day();
            if month == time::Month::February && time::util::is_leap_year(year) && day == 29 {
                // this handles the case of leap day
                period_start = period_start.replace_month(time::Month::March).unwrap();
                period_start = period_start.replace_day(1).unwrap();
            }
            period_start = period_start.replace_year(period_start.year()-2).unwrap();
        }
        "5 Year" => {
            let month = period_start.month();
            let year = period_start.year();
            let day = period_start.day();
            if month == time::Month::February && time::util::is_leap_year(year) && day == 29 {
                // this handles the case of leap day
                period_start = period_start.replace_month(time::Month::March).unwrap();
                period_start = period_start.replace_day(1).unwrap();
            }
            period_start = period_start.replace_year(period_start.year()-5).unwrap();
        }
        "10 Year" => {
            let month = period_start.month();
            let year = period_start.year();
            let day = period_start.day();
            if month == time::Month::February && time::util::is_leap_year(year) && day == 29 {
                // this handles the case of leap day
                period_start = period_start.replace_month(time::Month::March).unwrap();
                period_start = period_start.replace_day(1).unwrap();
            }
            period_start = period_start.replace_year(period_start.year()-10).unwrap();
        }
        "YTD" => {
            period_start = period_start.replace_month(time::Month::January).unwrap().replace_day(1).unwrap();
        }
        "Custom" | _ => {
            let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date").prompt();
            let time = NaiveTime::from_hms_opt(0,0,0).unwrap();
            let date_time = NaiveDateTime::new(date_input.unwrap(), time);
            period_start = OffsetDateTime::from_unix_timestamp(Utc.from_utc_datetime(&date_time).timestamp()).unwrap();
        }
    }

    println!("aid is {}", aid.clone());
    let account = _db.get_account(aid).unwrap();

    let (mut bank_history, bank_initial) : (Vec<BankRecord>, BankRecord);
    let bank_growth : f32;

    if account.has_bank {
        (bank_history, bank_initial) = _db.get_bank_history(aid, period_start.date().to_string(), period_end.date().to_string()).unwrap();
        bank_history.reverse();
        let bank_end_amount = bank_history.get(0).unwrap().amount;
        bank_growth = (bank_end_amount - bank_initial.amount) / ( bank_initial.amount ) * 100 as f32;
        println!("Initial: {} End: {} Growth: {}", bank_initial.amount, bank_end_amount, bank_growth)
    }

    if account.has_stocks {
        let tickers = _db.get_stock_tickers(aid).unwrap();
        let ticker = Select::new("Select ticker:", tickers).prompt().unwrap().to_string();
        let stock_growth = _db.get_stock_growth(aid, ticker, 
            NaiveDate::parse_from_str(period_start.date().to_string().as_str(), "%Y-%m-%d").unwrap(), 
            NaiveDate::parse_from_str(period_end.date().to_string().as_str(), "%Y-%m-%d").unwrap());
        println!("Growth {}", stock_growth);
    }


}
