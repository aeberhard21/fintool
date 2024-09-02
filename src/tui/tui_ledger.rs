use crate::database::db_accounts::AccountType;
use crate::database::db_ledger::LedgerEntry;
use crate::database::db_people::PeopleType;
use crate::database::*;
use crate::ledger::*;
use crate::user::*;
use chrono::{NaiveDate, Weekday};
use db_ledger::TransferType;
use inquire::*;

use self::db_accounts::AccountRecord;

pub fn record_ledger_entry(_aid: u32, _db: &mut DbConn, action : Option<TransferType> ) -> LedgerEntry {
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

    let transfer_type: TransferType;
    let mut pid = 0;
    let mut deposit_type : String;
    let mut payee;

    if action.is_none() {
        let deposit_options: Vec<&str> = vec!["Widthdrawal", "Deposit"];
        deposit_type = Select::new("Widthdrawal or deposit:", deposit_options)
            .prompt()
            .unwrap()
            .to_string();
        
        if deposit_type == "Widthdrawal" {
            transfer_type = TransferType::WidthdrawalToExternalAccount;
        } else {
            transfer_type = TransferType::DepositFromExternalAccount;
        }
    } else {
       transfer_type = action.unwrap();
    }

    // the match is equivalent to a switch statement
    match transfer_type {
        TransferType::WidthdrawalToExternalAccount => {
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
        TransferType::DepositFromExternalAccount => {
            let mut payees = _db.get_people(_aid, PeopleType::Payee).unwrap();
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
        TransferType::WidthdrawalToInternalAccount => {
            let mut payees = _db.get_people(_aid, PeopleType::Payee).unwrap();
            if payees.len() > 0 {
                payees.push("None".to_string());
                payees.push("New Beneficiary".to_string());
                payee = Select::new("Select withdrawal beneficiary:", payees)
                    .prompt()
                    .unwrap()
                    .to_string();
                if payee == "New Beneficiary" {
                    payee = Text::new("Enter withdrawal beneficiary:").prompt().unwrap().to_string();
                    pid = _db.add_person(_aid, PeopleType::Payee, payee).unwrap();
                } else if payee == "None" {
                    pid = _db.add_person(_aid, PeopleType::Payee, payee).unwrap();
                } else {
                    pid = _db.get_person_id(_aid, payee).unwrap();
                }
            } else {
                payee = Text::new("Enter withdrawal beneficiary:").prompt().unwrap().to_string();
                pid = _db.add_person(_aid, PeopleType::Payee, payee).unwrap();
            } 
        }
        TransferType::WidthdrawalToInternalAccount => {
            let mut payees = _db.get_people(_aid, PeopleType::Payee).unwrap();
            if payees.len() > 0 {
                payees.push("None".to_string());
                payees.push("New Source".to_string());
                payee = Select::new("Select deposit source:", payees)
                    .prompt()
                    .unwrap()
                    .to_string();
                if payee == "New Source" {
                    payee = Text::new("Enter deposit source:").prompt().unwrap().to_string();
                    pid = _db.add_person(_aid, PeopleType::Payee, payee).unwrap();
                } else if payee == "None" {
                    pid = _db.add_person(_aid, PeopleType::Payee, payee).unwrap();
                } else {
                    pid = _db.get_person_id(_aid, payee).unwrap();
                }
            } else {
                payee = Text::new("Enter deposit source:").prompt().unwrap().to_string();
                pid = _db.add_person(_aid, PeopleType::Payee, payee).unwrap();
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
            cid = _db.get_category_id(_aid, category).unwrap();
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
        transfer_type: transfer_type,
        payee_id: pid,
        category_id: cid,
        description: description_input,
    };

    return entry;
}