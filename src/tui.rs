use core::panic;
use crate::ledger::*;
use chrono::{NaiveDate, Weekday};
use inquire::*;

pub fn tui(_ledger: &mut Ledger) {
    println!("Welcome to FinTool!");
    loop {
        let commands: Vec<&str>= vec!["create", "add", "report", "view", "exit"];
        let command : String = Select::new("What would you like to do:", commands)
            .prompt()
            .unwrap()
            .to_string();

        match command.as_str() {
            "create" => {
                tui_create();
            }
            "add" => {
                tui_add(_ledger);
            }
            "view" => {
                tui_view(_ledger);
            }
            "report" => {
                tui_report(_ledger);
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

fn tui_create() {
    println!("Not implemented!");
}

fn tui_add(_ledger: &mut Ledger) {
    let commands: Vec<&str> = vec!["ledger", "investment", "none"];
    let command : String = Select::new("\nWhat would you like to add:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "ledger" => {
            println!("Adding ledger...\n");
            loop {
                add_ledger(_ledger);

                let another : bool = Confirm::new("Add another entry?")
                .with_default(false)
                .prompt()
                .unwrap();
                if !another {
                    break;
                }
            }
        }
        "investment" => {
            println!("Not implemented!");
        }
        "none" => {
            return;
        }
        _ => {
            panic!("Invalid command!");
        }
    }
}

fn tui_report(_ledger: &mut Ledger) {
    let commands: Vec<&str> = vec!["ledger"];
    let command : String = Select::new("What would you like to report:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "ledger" => {
            println!("Balance of account: {}", _ledger.sum());
        }
        _ => {
            panic!("Invalid command!");
        }
    }
}

fn tui_view(_ledger: &mut Ledger) {
    let commands: Vec<&str> = vec!["ledger", "portfolio", "none"];
    let command : String = Select::new("What would you like to view:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "ledger" => {
            _ledger.print();
        }
        _ => {
            panic!("Invalid command!");
        }
    }
}

fn add_ledger(_ledger: &mut Ledger) {
    let deposit_options: Vec<&str>= vec!["Credit", "Debit"];

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

    let deposit_type : String = Select::new("Credit or debit:", deposit_options).prompt().unwrap().to_string();
    let deposit : bool;

    let mut payee : String = "".to_string();

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

    let description_input : String = Text::new("Enter payment description:").prompt().unwrap().to_string();

    let entry = LedgerEntry {
        date: date, 
        amount: amount,
        deposit: deposit,
        payee: payee,
        description: description_input
    };
    _ledger.add(entry);

}