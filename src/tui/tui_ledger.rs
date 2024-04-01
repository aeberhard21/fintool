use crate::database::db_accounts::AccountType;
use crate::database::db_ledger::LedgerEntry;
use crate::database::db_people::PeopleType;
use crate::database::*;
use crate::ledger::*;
use crate::user::*;
use chrono::{NaiveDate, Weekday};
use inquire::*;

use self::db_accounts::AccountRecord;

pub fn add_ledger(_aid: u32, _db: &mut DbConn) -> LedgerEntry {
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

    let mut payee;
    let mut pid = 0;

    // the match is equivalent to a switch statement
    match deposit_type.as_str() {
        "Credit" => {
            deposit = false;
            let mut payees = _db.get_people(_aid, PeopleType::Payee).unwrap();
            if payees.len() > 0 {
                payees.push("None".to_string());
                payees.push("New Payee".to_string());
                payee = Select::new("Select payee:", payees)
                    .prompt()
                    .unwrap()
                    .to_string();
                if payee == "New Payee" {
                    payee = Text::new("Enter payee:").prompt().unwrap().to_string();
                    pid = _db.add_person(_aid, PeopleType::Payee, payee).unwrap();
                } else if payee == "None" {
                    pid = _db.add_person(_aid, PeopleType::Payee, payee).unwrap();
                } else {
                    pid = _db.get_person_id(_aid, payee).unwrap();
                }
            } else {
                payee = Text::new("Enter payee:").prompt().unwrap().to_string();
                pid = _db.add_person(_aid, PeopleType::Payee, payee).unwrap();
            }
        }
        "Debit" => {
            deposit = true;
            let mut payees = _db.get_people(_aid, PeopleType::Payer).unwrap();
            if payees.len() > 0 {
                payees.push("None".to_string());
                payees.push("New Payer".to_string());
                payee = Select::new("Select payer:", payees)
                    .prompt()
                    .unwrap()
                    .to_string();
                if payee == "New Payer" {
                    payee = Text::new("Enter payer:").prompt().unwrap().to_string();
                    pid = _db.add_person(_aid, PeopleType::Payer, payee).unwrap();
                } else if payee == "None" {
                    pid = _db.add_person(_aid, PeopleType::Payer, payee).unwrap();
                } else {
                    pid = _db.get_person_id(_aid, payee).unwrap();
                }
            } else {
                payee = Text::new("Enter payer:").prompt().unwrap().to_string();
                pid = _db.add_person(_aid, PeopleType::Payer, payee).unwrap();
            }
        }
        _ => {
            panic!("Invalid entry.");
        }
    }

    let mut categories = _db.get_categories(_aid).unwrap();
    let mut category;
    let mut cid = 0;
    if categories.len() > 0 {
        categories.push("None".to_string());
        categories.push("New Category".to_string());
        category = Select::new("Select category:", categories)
            .prompt()
            .unwrap()
            .to_string();

        if category == "New Category" {
            category = Text::new("Enter payment category:")
                .prompt()
                .unwrap()
                .to_string();
            cid = _db.add_category(_aid, category).unwrap();
        } else if category == "None" {
            cid = _db.add_category(_aid, category).unwrap();
        } else {
            cid = _db.get_category_id(_aid, &category).unwrap();
        }
    } else {
        category = Text::new("Enter payment category:")
            .prompt()
            .unwrap()
            .to_string();
        cid = _db.add_category(_aid, category).unwrap();
    }

    let description_input: String = Text::new("Enter payment description:")
        .prompt()
        .unwrap()
        .to_string();

    let entry = LedgerEntry {
        date: date,
        amount: amount,
        deposit: deposit,
        payee_id: pid,
        category_id: cid,
        description: description_input,
    };

    return entry;
}

// pub fn print_ledger(_user: &mut User, _db: &mut DbConn) {
//     let ledger_options: Vec<String> = _user.get_ledgers();
//     let _ledger: String = Select::new("Select which ledger to view:", ledger_options)
//         .prompt()
//         .unwrap()
//         .to_string();
//     _user.print_ledger(_db, _ledger);
// }
