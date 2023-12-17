use crate::database;
use crate::database::*;
use crate::database::db_accounts::AccountType;
use crate::ledger::*;
use crate::user::*;
use chrono::{NaiveDate, Weekday};
use inquire::*;
use rusqlite::config::DbConfig;

pub fn create_ledger(_uid: u32, _db: &mut DbConn) {
    let name: String = Text::new("Enter ledger name:")
        .prompt()
        .unwrap()
        .to_string();
    // _user.create_ledger(_db, name);
    _db.add_account(_uid, name, AccountType::Ledger);
}

pub fn add_ledger(_user: &mut User, _db: &mut DbConn) {
    let ledger_options: Vec<String> = _user.get_ledgers();
    let _ledger: String = Select::new("Select which ledger to add to:", ledger_options)
        .prompt()
        .unwrap()
        .to_string();

    let deposit_options: Vec<&str> = vec!["Credit", "Debit"];

    // this function returns either "Ok" or "Err". "Ok" indicates that the type T in Result<T, E>
    // is okay to be used.
    let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date").prompt();
    let date: String = date_input.unwrap().to_string();

    println!("Entered date is {0}", date);

    let amount_input: Result<f32, InquireError> = CustomType::<f32>::new("Enter amount")
        .with_placeholder("00000.00")
        .with_default(00000.00)
        .with_error_message("Please type a valid amount!")
        .prompt();
    let amount: f32 = amount_input.unwrap();

    println!("Entered amount is {}", amount.to_string());

    let deposit_type: String = Select::new("Credit or debit:", deposit_options)
        .prompt()
        .unwrap()
        .to_string();
    let deposit: bool;

    let mut payee: String = "".to_string();

    // the match is equivalent to a switch statement
    match deposit_type.as_str() {
        "Credit" => {
            deposit = false;
            payee = Text::new("Enter payee:").prompt().unwrap().to_string();
        }
        "Debit" => {
            deposit = true;
        }
        _ => {
            panic!("Invalid entry.");
        }
    }

    let description_input: String = Text::new("Enter payment description:")
        .prompt()
        .unwrap()
        .to_string();

    let entry = LedgerEntry {
        date: date,
        amount: amount,
        deposit: deposit,
        payee: payee,
        description: description_input,
    };
    // _ledger.add(entry);
    _user.add_ledger_entry(_ledger, _db, entry);
}

pub fn print_ledger(_user: &mut User, _db: &mut DbConn) {
    let ledger_options: Vec<String> = _user.get_ledgers();
    let _ledger: String = Select::new("Select which ledger to view:", ledger_options)
        .prompt()
        .unwrap()
        .to_string();
    _user.print_ledger(_db, _ledger);
}
