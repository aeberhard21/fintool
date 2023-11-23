// use core::panic;
use crate::ledger::Ledger;
use crate::tui::tui_ledger::*;
use chrono::{NaiveDate, Weekday};
use inquire::*;

mod tui_ledger;

pub fn menu(_ledger: &mut Ledger) {
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
