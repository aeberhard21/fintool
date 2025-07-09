use core::panic;

use crate::database::{self, *};
// use crate::tui::tui_budgets::create_budget;
use crate::types::accounts::*;
use inquire::*;

pub fn select_account_by_type(
    _uid: u32,
    _db: &mut DbConn,
    atype: AccountType,
) -> Option<(u32, String)> {
    let msg;
    match &atype {
        &AccountType::Bank => msg = "Select bank account: ",
        &AccountType::CD => msg = "Select CD account: ",
        &AccountType::Wallet => msg = "Select wallet: ",
        &AccountType::Investment => msg = "Select investment account: ",
        _ => panic!("Unrecognized account type!"),
    }
    let accounts: Option<Vec<String>> = _db.get_user_accounts_by_type(_uid, atype).unwrap();
    if accounts.is_none() {
        return None;
    }
    let account: String = Select::new(msg, accounts.unwrap())
        .prompt()
        .unwrap()
        .to_string();
    let aid = _db.get_account_id(_uid, account.clone()).unwrap();
    return Some((aid, account));
}

pub fn select_account_by_filter(_uid: u32, _db: &mut DbConn, filter: AccountFilter) -> u32 {
    let msg;
    match &filter {
        &AccountFilter::Bank => msg = "Select bank account: ",
        &AccountFilter::Stocks => msg = "Select investment account: ",
        &AccountFilter::Wallet => msg = "Select wallet account: ",
        &AccountFilter::Budget => msg = "Select budget account: ",
        _ => panic!("Unrecognized account type!"),
    }
    let accounts: Vec<String> = _db.get_user_accounts_by_filter(_uid, filter).unwrap();
    let account: String = Select::new(msg, accounts).prompt().unwrap().to_string();
    let aid = _db.get_account_id(_uid, account).unwrap();
    return aid;
}
