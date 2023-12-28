use std::borrow::Borrow;

use crate::database;
use crate::database::*;
use crate::database::db_accounts::AccountType;
use crate::database::db_banks::BankRecord;
use crate::database::db_hsa::HsaRecord;
use chrono::{NaiveDate, Weekday, Date, NaiveDateTime};
use inquire::*;
use savefile::Variant;


pub fn create_account(_atype: AccountType, _uid: u32, _db: &mut DbConn ) -> u32 {
    let name: String = Text::new("Enter account name:").prompt().unwrap().to_string();
    _db.add_account(_uid, name, _atype).unwrap()
}

pub fn select_account(_uid : u32, _db: &mut DbConn, atype: AccountType) -> u32 {
    let msg;
    match &atype {
        &AccountType::Bank => {msg = "Select bank account: ";}
        &AccountType::CD => {msg = "Select CD account: ";}
        &AccountType::Health => {msg = "Select health account: ";}
        &AccountType::Ledger => {msg = "Select ledger: ";}
        &AccountType::Investment => {msg = "Select investment account: ";}
        &AccountType::Retirement => {msg = "Select retirement account: ";}
    }
    let accounts: Vec<String> = _db.get_user_accounts(_uid, &atype).unwrap();
    let account: String = Select::new(msg, accounts).prompt().unwrap().to_string();
    let aid = _db.get_account_id(_uid, account).unwrap();
    return aid
}

pub fn record_f32_amount(_uid: u32, _db: &mut DbConn) -> BankRecord {
    let amount: Result<f32, InquireError> = CustomType::<f32>::new("Enter amount in account: ")
        .with_placeholder("00000.00")
        .with_default(00000.00)
        .with_error_message("Please type a valid amount!")
        .prompt();

    let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date").prompt();
    let date = &date_input.unwrap().and_hms_milli_opt(0, 0, 0, 0).unwrap().timestamp();
    println!("The date is: {} ", &date);
    let converted_time = NaiveDateTime::from_timestamp(*date, 0).to_string();
    println!("The date is: {} ", converted_time);

    return BankRecord { amount: amount.unwrap(), date: *date };
}

pub fn record_health_account(_uid: u32, _db: &mut DbConn) -> HsaRecord {
    let fixed: Result<f32, InquireError> = CustomType::<f32>::new("Enter fixed amount in account: ")
        .with_placeholder("00000.00")
        .with_default(00000.00)
        .with_error_message("Please type a valid amount!")
        .prompt();

    let variable: Result<f32, InquireError> = CustomType::<f32>::new("Enter variable amount in account: ")
        .with_placeholder("00000.00")
        .with_default(00000.00)
        .with_error_message("Please type a valid amount!")
        .prompt();

    let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date").prompt();
    let date = &date_input.unwrap().and_hms_milli_opt(0, 0, 0, 0).unwrap().timestamp();
    println!("The date is: {} ", &date);
    let converted_time = NaiveDateTime::from_timestamp(*date, 0).to_string();
    println!("The date is: {} ", converted_time);

    return HsaRecord { date: *date, fixed : fixed.unwrap(), variable : variable.unwrap() };
}

