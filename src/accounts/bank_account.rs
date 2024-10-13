use inquire::Select;
use inquire::DateSelect;
use inquire::Confirm;
use std::sync::Arc;

use crate::tui::AccountOperations;
use crate::database::DbConn;
use crate::types::accounts;
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
        let acct: BankAccount = Self { 
            id : id, 
            db : db.clone(), 
            fixed : FixedAccount::new(id, db.clone())
        };
        acct
    }
}

impl AccountOperations for BankAccount {
    fn create( account_id : u32, db : &mut DbConn ) { 
        let mut acct = Self::new(account_id, db);
        db.add_participant(account_id, ParticipantType::Payee, "Fixed".to_string());
        acct.record();
    }

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
        
    }
}
