use inquire::Select;
use inquire::DateSelect;
use inquire::Confirm;
use inquire::InquireError;
use inquire::Text;

use crate::OffsetDateTime;
use chrono::{Datelike, Days, Local, Utc};
use chrono::{Date, NaiveDate, NaiveTime, NaiveDateTime, Weekday};
use time::{Duration};
use crate::tui::AccountOperations;
use crate::database::DbConn;
use crate::types::accounts;
use crate::types::participants::ParticipantType;
use crate::types::transfer_types;
use crate::types::transfer_types::TransferType;
use crate::types::accounts::AccountRecord;
use crate::tui::AccountCreation;
use crate::tui::query_user_for_analysis_period;
use crate::types::accounts::AccountType;

use super::variable_account;
use super::variable_account::VariableAccount;

pub struct InvestmentAccountManager {
    id    : u32,
    db    : DbConn,
    variable : VariableAccount,
}

impl AccountCreation for InvestmentAccountManager {
    fn create() -> AccountRecord {
        let mut name: String = String::new();
        loop {
            name = Text::new("Enter investment account name:")
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
        let mut has_stocks = true;
        let mut has_ledger = false;
        let mut has_budget = false;
    
        let account: AccountRecord = AccountRecord {
            atype: AccountType::Investment,
            name: name,
            has_stocks: has_stocks,
            has_bank: has_bank,
            has_ledger: has_ledger,
            has_budget: has_budget
        };

        return account;
    }
}


impl InvestmentAccountManager {
    pub fn new(id : u32, db : &mut DbConn) -> Self {
        let mut acct = Self { 
            id : id, 
            db : db.clone(),
            variable : VariableAccount::new(id, db)
        };

        // acct.db.add_participant(id, ParticipantType::Payee, "Fixed".to_string());
        // acct.db.add_participant(id, ParticipantType::Payer, "Fixed".to_string());
        // acct.db.add_category(id, "Bought".to_string());
        // acct.db.add_category(id, "Cash Dividend".to_string());
        // acct.db.add_category(id, "Interest".to_string());
        // acct.db.add_category(id, "Dividend-Reinvest".to_string());
        // acct.db.add_category(id, "Sold".to_string());
        // acct.db.add_category(id, "Deposit".to_string());
        // acct.db.add_category(id, "Withdrawal".to_string());

        acct
    }
}

impl AccountOperations for InvestmentAccountManager {
    // fn create( account_id : u32, db : &mut DbConn ) {
    //     let mut acct: InvestmentAccountManager = Self::new(account_id, db);
    //     // record several payees and payer types for use
    //     db.add_participant(account_id, ParticipantType::Payee, "Fixed".to_string());
    //     db.add_participant(account_id, ParticipantType::Payer, "Fixed".to_string());
    //     db.add_category(account_id, "Bought".to_string());
    //     db.add_category(account_id, "Cash Dividend".to_string());
    //     db.add_category(account_id, "Interest".to_string());
    //     db.add_category(account_id, "Dividend-Reinvest".to_string());
    //     db.add_category(account_id, "Sold".to_string());
    //     db.add_category(account_id, "Deposit".to_string());
    //     db.add_category(account_id, "Withdrawal".to_string());

    //     acct.record();
    // }

    fn record( &mut self ) {
        const RECORD_OPTIONS : [&'static str; 4] = ["Deposit", "Withdrawal", "Purchase", "Sale"];
        loop { 
            let action = Select::new("\nWhat transaction would you like to record?", RECORD_OPTIONS.to_vec())
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

        const REPORT_OPTIONS :[&'static str; 2] = ["Total Value", "Time-Weighted Rate of Return"];
        let choice = Select::new("What would you like to report: ", REPORT_OPTIONS.to_vec())
            .prompt().unwrap().to_string();
        match choice.as_str() { 
            "Total Value" => {
                let value = self.variable.get_current_value();
                println!("\tTotal Account Value: {}", value);
            }
            "Time-Weighted Rate of Return" => {
                let (period_start, period_end) = query_user_for_analysis_period();
                let twr = self.variable.time_weighted_return(period_start, period_end);
                println!("\tRate of return: {}%", twr);
            }
            _ => {
                panic!("Unrecognized input!");
            }
        }
    }

}


