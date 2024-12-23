use chrono::NaiveDate;
use inquire::Select;
use inquire::DateSelect;
use inquire::Confirm;
use inquire::Text;
use std::sync::Arc;

use crate::tui::query_user_for_analysis_period;
use crate::types::accounts::AccountType;
use crate::tui::AccountCreation;
use crate::tui::AccountOperations;
use crate::database::DbConn;
use crate::types::accounts;
use crate::types::accounts::AccountRecord;
use crate::types::participants::ParticipantType;
use crate::types::transfer_types;
use crate::types::transfer_types::TransferType;

use super::fixed_account;
use super::fixed_account::FixedAccount;

pub struct BankAccount { 
    id : u32, 
    db : DbConn, 
    fixed : FixedAccount
}

impl BankAccount {
    pub fn new(id : u32, db : &mut DbConn) -> Self {
        let mut acct: BankAccount = Self { 
            id : id, 
            db : db.clone(), 
            fixed : FixedAccount::new(id, db.clone())
        };
        // acct.db.add_participant(id, ParticipantType::Payee, "Fixed".to_string());
        acct
    }
}

impl AccountCreation for BankAccount {
    fn create() -> AccountRecord {
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
        let mut has_bank = true;
        let mut has_stocks = false;
        let mut has_ledger = false;
        let mut has_budget = false;

        let account: AccountRecord = AccountRecord {
            atype: AccountType::Bank,
            name: name,
            has_stocks: has_stocks,
            has_bank: has_bank,
            has_ledger: has_ledger,
            has_budget: has_budget
        };

        return account;
    }
}

impl AccountOperations for BankAccount {
    // fn create( account_id : u32, db : &mut DbConn ) { 
    //     let mut acct = Self::new(account_id, db);
    //     db.add_participant(account_id, ParticipantType::Payee, "Fixed".to_string());
    //     acct.record();
    // }

    fn record( &mut self ) {
        loop { 
            let action = Select::new("\nWhat transaction would you like to record?", vec!["Deposit", "Withdrawal"])
                .prompt().unwrap().to_string();
            match action.as_str() {
                "Deposit" => {
                    self.fixed.deposit();
                }
                "Withdrawal" => {
                    self.fixed.withdrawal();
                }
                _ => {
                    panic!("Unrecognized input!");
                }
            }
            let record_again = Confirm::new("Would you like to record another transaction?").prompt().unwrap();;
            if !record_again { 
                return;
            }
        }
    }

    fn modify( &mut self ) {
        
    }

    fn export( &mut self ) {
        
    }

    fn report( &mut self ) {
        const REPORT_OPTIONS: [&'static str; 2] = ["Total Value", "Simple Growth Rate"];
        let choice: String = Select::new("What would you like to report: ", REPORT_OPTIONS.to_vec())
            .prompt().unwrap().to_string();
        match choice.as_str() {
            "Total Value" => {
                let value = self.fixed.get_current_value();
                println!("\tTotal Account Value: {}", value);
            },
            "Simple Growth Rate" => {
                let (period_start, period_end ) = query_user_for_analysis_period();
                let rate = self.fixed.simple_rate_of_return(period_start, period_end);
                println!("\tRate of return: {}%", rate);
            }
            _ => {
                panic!("Unrecognized input!");
            }
        }
    }
}
