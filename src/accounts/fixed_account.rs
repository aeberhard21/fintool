use autocompletion::Replacement;
use chrono::format::Fixed;
use inquire::*;
use chrono::{Datelike, NaiveDate};
use type_aliases::Suggester;
use yahoo_finance_api::PeriodInfo;
use crate::database::DbConn;
use crate::types::ledger::{LedgerEntry, ParticipantAutoCompleter};
use crate::types::participants::{self, ParticipantType};
use crate::types::transfer_types::TransferType;
use crate::types::categories::CategoryAutoCompleter;

pub struct FixedAccount { 
    pub id : u32,
    pub db : DbConn,        
}

impl FixedAccount { 

    pub fn new( id : u32, db : DbConn ) -> Self {
        let acct = Self { 
            id : id, 
            db : db
        };
        acct
    }

    pub fn withdrawal( &mut self  ) {

        let date_input: String = DateSelect::new("Enter date")
            .prompt().unwrap().to_string();

        let amount_input: f32 = CustomType::<f32>::new("Enter withdrawal amount")
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let selected_payee = Text::new("Enter payee:")
            .with_autocomplete(ParticipantAutoCompleter { aid : self.id, db : self.db.clone(), ptype : ParticipantType::Payee })
            .prompt().unwrap();

        let pid = self.db.check_and_add_participant(self.id, selected_payee, ParticipantType::Payee);

        let cid;
        let selected_category = Text::new("Enter category:")
            .with_autocomplete( CategoryAutoCompleter {
                aid : self.id, 
                db : self.db.clone()
            })
            .prompt()
            .unwrap();
        cid = self.db.check_and_add_category(self.id, selected_category);

        let description_input: String = Text::new("Enter payment description:")
            .prompt()
            .unwrap()
            .to_string();

        let withdrawal = LedgerEntry {
            date : date_input,
            amount : amount_input,
            transfer_type : TransferType::WidthdrawalToExternalAccount,
            participant : pid, 
            category_id : cid, 
            description : description_input
        };

        self.db.add_ledger_entry(self.id, withdrawal).unwrap();

    }
    
    pub fn deposit ( &mut self ) {

        let date_input: String = DateSelect::new("Enter date")
            .prompt().unwrap().to_string();

        let amount_input: f32 = CustomType::<f32>::new("Enter deposit amount")
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let selected_payee = Text::new("Enter payer:")
            .with_autocomplete(ParticipantAutoCompleter { aid : self.id, db : self.db.clone(), ptype : ParticipantType::Payer })
            .prompt().unwrap();

        let pid = self.db.check_and_add_participant(self.id, selected_payee, ParticipantType::Payer);


        let cid;
        let selected_category = Text::new("Enter category:")
            .with_autocomplete( CategoryAutoCompleter {
                aid : self.id, 
                db : self.db.clone()
            })
            .prompt()
            .unwrap();

        cid = self.db.check_and_add_category(self.id, selected_category);

        let description_input: String = Text::new("Enter payment description:")
            .prompt()
            .unwrap()
            .to_string();

        let withdrawal = LedgerEntry {
            date : date_input,
            amount : amount_input,
            transfer_type : TransferType::DepositFromExternalAccount,
            participant: pid, 
            category_id : cid, 
            description : description_input
        };

        self.db.add_ledger_entry(self.id, withdrawal).unwrap();
    }

    pub fn get_current_value(&mut self) -> f32 { 
        return self.db.get_current_value(self.id).unwrap();
    }

    pub fn simple_rate_of_return(&mut self, start_date : NaiveDate, end_date : NaiveDate) -> f32 {
        let mut rate: f32 = 0.0;
        let starting_amount = self.db.get_cumulative_total_of_ledger_before_date(self.id, start_date).unwrap();
        let ending_amount: f32 = self.db.get_cumulative_total_of_ledger_before_date(self.id, end_date).unwrap();
        rate = (ending_amount-starting_amount)/(starting_amount);
        return rate;
    }

    pub fn compound_annual_growth_rate(&mut self, start_date : NaiveDate, end_date : NaiveDate) -> f32 {
        let mut rate: f32 = 0.0;
        let starting_amount: f32 = self.db.get_cumulative_total_of_ledger_before_date(self.id, start_date).unwrap();
        let ending_amount: f32 = self.db.get_cumulative_total_of_ledger_before_date(self.id, end_date).unwrap();
        let date_diff: i32 = end_date.num_days_from_ce() - start_date.num_days_from_ce();
        let year_diff: f32  = date_diff as f32 / 365.0;

        rate = (ending_amount/starting_amount).powf(1 as f32 /year_diff);
        return rate;
    }

}

