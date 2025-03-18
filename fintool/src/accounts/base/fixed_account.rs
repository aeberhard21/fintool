use crate::database::DbConn;
use crate::tui::{create_new_account, decode_and_create_account_type};
use crate::types::accounts::AccountRecord;
use crate::types::categories::CategoryAutoCompleter;
use crate::types::ledger::{LedgerInfo, LedgerRecord, ParticipantAutoCompleter};
use crate::types::participants::ParticipantType;
use chrono::{Datelike, NaiveDate};
use inquire::*;
use shared_lib::TransferType;
use std::collections::HashMap;

use super::AccountOperations;

pub struct FixedAccount {
    pub id: u32,
    pub uid: u32,
    pub db: DbConn,
}

impl FixedAccount {
    pub fn new(uid: u32, id: u32, db: DbConn) -> Self {
        let acct = Self {
            uid: uid,
            id: id,
            db: db,
        };
        acct
    }

    pub fn withdrawal(&mut self) {
        let date_input: String = DateSelect::new("Enter date").prompt().unwrap().to_string();

        let amount_input: f32 = CustomType::<f32>::new("Enter withdrawal amount")
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let selected_payee = Text::new("Enter payee:")
            .with_autocomplete(ParticipantAutoCompleter {
                aid: self.id,
                db: self.db.clone(),
                ptype: ParticipantType::Payee,
            })
            .prompt()
            .unwrap();

        let pid =
            self.db
                .check_and_add_participant(self.id, selected_payee, ParticipantType::Payee);

        let cid;
        let selected_category = Text::new("Enter category:")
            .with_autocomplete(CategoryAutoCompleter {
                aid: self.id,
                db: self.db.clone(),
            })
            .prompt()
            .unwrap();
        cid = self.db.check_and_add_category(self.id, selected_category);

        let description_input: String = Text::new("Enter payment description:")
            .prompt()
            .unwrap()
            .to_string();

        let withdrawal = LedgerInfo {
            date: date_input,
            amount: amount_input,
            transfer_type: TransferType::WithdrawalToExternalAccount,
            participant: pid,
            category_id: cid,
            description: description_input,
        };

        let link = Confirm::new("Link transaction to another account?")
            .prompt()
            .unwrap();
        let id = self
            .db
            .add_ledger_entry(self.id, withdrawal.clone())
            .unwrap();
        let entry = LedgerRecord {
            id: id,
            info: withdrawal,
        };

        if link {
            let accounts = self.db.get_user_account_info(self.uid).unwrap();
            let mut account_map: HashMap<String, AccountRecord> = HashMap::new();
            let mut account_names: Vec<String> = Vec::new();
            for account in accounts.iter() {
                account_names.push(account.info.name.clone());
                account_map.insert(account.info.name.clone(), account.clone());
            }

            // add new account
            account_names.push("New Account".to_string());

            // add none clause
            account_names.push("None".to_string());
            let selected_account = Select::new("Select account:", account_names)
                .prompt()
                .unwrap()
                .to_string();

            if selected_account.clone() == "None" {
                return;
            }

            let mut acct: Box<dyn AccountOperations>;
            if selected_account.clone() == "New Account" {
                (acct, _) = create_new_account(self.uid, &mut self.db);
            } else {
                let acctx = account_map
                    .get(&selected_account)
                    .expect("Account not found!");
                acct = decode_and_create_account_type(self.uid, &mut self.db, acctx);
            }

            if acct.link(self.id, entry).is_none() {
                println!("Unable to link transactions. Please review!");
            };
        }
    }

    pub fn deposit(&mut self) {
        let date_input: String = DateSelect::new("Enter date").prompt().unwrap().to_string();

        let amount_input: f32 = CustomType::<f32>::new("Enter deposit amount")
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let selected_payee = Text::new("Enter payer:")
            .with_autocomplete(ParticipantAutoCompleter {
                aid: self.id,
                db: self.db.clone(),
                ptype: ParticipantType::Payer,
            })
            .prompt()
            .unwrap();

        let pid =
            self.db
                .check_and_add_participant(self.id, selected_payee, ParticipantType::Payer);

        let cid;
        let selected_category = Text::new("Enter category:")
            .with_autocomplete(CategoryAutoCompleter {
                aid: self.id,
                db: self.db.clone(),
            })
            .prompt()
            .unwrap();

        cid = self.db.check_and_add_category(self.id, selected_category);

        let description_input: String = Text::new("Enter payment description:")
            .prompt()
            .unwrap()
            .to_string();

        let deposit = LedgerInfo {
            date: date_input,
            amount: amount_input,
            transfer_type: TransferType::DepositFromExternalAccount,
            participant: pid,
            category_id: cid,
            description: description_input,
        };

        let link = Confirm::new("Link transaction to another account?")
            .prompt()
            .unwrap();
        let id = self.db.add_ledger_entry(self.id, deposit.clone()).unwrap();
        let entry = LedgerRecord {
            id: id,
            info: deposit,
        };

        if link {
            let accounts = self.db.get_user_account_info(self.uid).unwrap();
            let mut account_map: HashMap<String, AccountRecord> = HashMap::new();
            let mut account_names: Vec<String> = Vec::new();
            for account in accounts.iter() {
                account_names.push(account.info.name.clone());
                account_map.insert(account.info.name.clone(), account.clone());
            }

            // add none clause
            account_names.push("None".to_string());
            let selected_account = Select::new("Select account:", account_names)
                .prompt()
                .unwrap()
                .to_string();
            let acctx = account_map
                .get(&selected_account)
                .expect("Account not found!");
            let mut acct = decode_and_create_account_type(self.uid, &mut self.db, acctx);

            acct.link(self.id, entry);
        }
    }

    pub fn get_current_value(&mut self) -> f32 {
        return self.db.get_current_value(self.id).unwrap();
    }

    pub fn simple_rate_of_return(&mut self, start_date: NaiveDate, end_date: NaiveDate) -> f32 {
        let mut rate: f32 = 0.0;
        let starting_amount = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.id, start_date)
            .unwrap();
        let ending_amount: f32 = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.id, end_date)
            .unwrap();
        rate = (ending_amount - starting_amount) / (starting_amount);
        return rate;
    }

    pub fn compound_annual_growth_rate(
        &mut self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> f32 {
        let mut rate: f32 = 0.0;
        let starting_amount: f32 = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.id, start_date)
            .unwrap();
        let ending_amount: f32 = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.id, end_date)
            .unwrap();
        let date_diff: i32 = end_date.num_days_from_ce() - start_date.num_days_from_ce();
        let year_diff: f32 = date_diff as f32 / 365.0;

        rate = (ending_amount / starting_amount).powf(1 as f32 / year_diff);
        return rate;
    }
}
