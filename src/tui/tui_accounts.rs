use crate::database;
use crate::database::*;
use crate::database::db_accounts::AccountType;
use chrono::{NaiveDate, Weekday};
use inquire::*;

pub fn create_account(_atype: AccountType, _uid: u32, _db: &mut DbConn ) {
    let name: String = Text::new("Enter account name:").prompt().unwrap().to_string();
    _db.add_account(_uid, name, _atype);
}