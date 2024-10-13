use inquire::Select;
use inquire::DateSelect;
use inquire::Confirm;

use crate::tui::AccountOperations;
use crate::database::DbConn;
use crate::types::accounts;
use crate::types::participants::ParticipantType;
use crate::types::transfer_types;
use crate::types::transfer_types::TransferType;

use super::variable_account;
use super::variable_account::VariableAccount;

pub struct InvestmentAccountManager {
    id    : u32,
    db    : DbConn,
    variable : VariableAccount,
}


impl InvestmentAccountManager {
    pub fn new(id : u32, db : &mut DbConn) -> Self {
        Self { 
            id : id, 
            db : db.clone(),
            variable : VariableAccount::new(id, db)
        }
    }
}

impl AccountOperations for InvestmentAccountManager {
    fn create( account_id : u32, db : &mut DbConn ) {
        let mut acct = Self::new(account_id, db);
        // record several payees and payer types for use
        db.add_participant(account_id, ParticipantType::Payee, "Fixed".to_string());
        db.add_participant(account_id, ParticipantType::Payer, "Fixed".to_string());
        db.add_category(account_id, "Bought".to_string());
        db.add_category(account_id, "Cash Dividend".to_string());
        db.add_category(account_id, "Interest".to_string());
        db.add_category(account_id, "Dividend-Reinvest".to_string());
        db.add_category(account_id, "Sold".to_string());
        db.add_category(account_id, "Deposit".to_string());
        db.add_category(account_id, "Withdrawal".to_string());

        acct.record();
    }

    fn record( &mut self ) {
        loop { 
            let action = Select::new("\nWhat transaction would you like to record?", vec!["Deposit", "Withdrawal", "Purchase", "Sale"])
                .prompt().unwrap().to_string();
            match action.as_str() {
                "Deposit" => {
                    self.variable.fixed.deposit();
                }
                "Withdrawal" => {
                    self.variable.fixed.withdrawal();
                }
                "Purchase" =>  {
                    self.variable.purchase_stock();
                }
                "Sale" => {
                    self.variable.sell_stock();
                }
                _ => {
                    panic!("Unrecognized input!");
                }
            }

            let record_again = Confirm::new("Would you like to record another transaction?").prompt().unwrap();
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


