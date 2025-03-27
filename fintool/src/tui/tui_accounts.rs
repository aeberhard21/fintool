use core::panic;

use crate::database::{self, *};
use crate::tui::tui_budgets::create_budget;
use crate::types::accounts::*;
use inquire::*;

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
