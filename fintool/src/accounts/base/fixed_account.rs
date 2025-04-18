use crate::database::DbConn;
use crate::tui::{create_new_account, decode_and_create_account_type};
use crate::types::accounts::AccountRecord;
use crate::types::categories::CategoryAutoCompleter;
use crate::types::ledger::{LedgerInfo, LedgerRecord};
use crate::types::participants::{ParticipantAutoCompleter, ParticipantType};
use chrono::{Datelike, NaiveDate};
use inquire::*;
use shared_lib::{LedgerEntry, TransferType};
use std::collections::HashMap;
use std::hash::Hash;

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
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
                ptype: ParticipantType::Payee,
            })
            .prompt()
            .unwrap();

        let pid =
            self.db
                .check_and_add_participant(self.uid, self.id, selected_payee, ParticipantType::Payee);

        let cid;
        let selected_category = Text::new("Enter category:")
            .with_autocomplete(CategoryAutoCompleter {
                uid : self.uid,
                aid: self.id,
                db: self.db.clone(),
            })
            .prompt()
            .unwrap()
            .to_ascii_uppercase();
        cid = self.db.check_and_add_category(self.uid, self.id, selected_category);

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
            ancillary_f32data : 0.0
        };

        let link = Confirm::new("Link transaction to another account?")
            .prompt()
            .unwrap();
        let id = self
            .db
            .add_ledger_entry(self.uid, self.id, withdrawal.clone())
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
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
                ptype: ParticipantType::Payer,
            })
            .prompt()
            .unwrap();

        let pid =
            self.db
                .check_and_add_participant(self.uid, self.id, selected_payee, ParticipantType::Payer);

        let cid;
        let selected_category = Text::new("Enter category:")
            .with_autocomplete(CategoryAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
            })
            .prompt()
            .unwrap()
            .to_ascii_uppercase();

        cid = self.db.check_and_add_category(self.uid, self.id, selected_category);

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
            ancillary_f32data : 0.0
        };

        let id = self.db.add_ledger_entry(self.uid, self.id, deposit.clone()).unwrap();
        let entry = LedgerRecord {
            id: id,
            info: deposit,
        };

        let link = Confirm::new("Link transaction to another account?")
            .prompt()
            .unwrap();

        if link {
            self.link_transaction(entry);
        }
    }

    pub fn modify(&mut self, selected_record: LedgerRecord) -> LedgerRecord {
        let updated_date: String = DateSelect::new("Enter date")
            .with_starting_date(
                NaiveDate::parse_from_str(selected_record.info.date.as_str(), "%Y-%m-%d").unwrap(),
            )
            .prompt()
            .unwrap()
            .to_string();

        let updated_amount: f32 = CustomType::<f32>::new("Enter deposit amount")
            .with_placeholder(selected_record.info.amount.to_string().as_str())
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        const TRANSFER_OPTIONS: [&'static str; 4] = [
            "Deposit from External Account",
            "Deposit from Internal Account",
            "Withdrawal to External Account",
            "Withdrawal to Internal Account",
        ];

        let starting_transfer_filter = match selected_record.info.transfer_type {
            TransferType::DepositFromExternalAccount => TRANSFER_OPTIONS[0],
            TransferType::DepositFromInternalAccount => TRANSFER_OPTIONS[1],
            TransferType::WithdrawalToExternalAccount => TRANSFER_OPTIONS[2],
            TransferType::WithdrawalToInternalAccount => TRANSFER_OPTIONS[3],
            TransferType::ZeroSumChange => TRANSFER_OPTIONS[1]
        };

        let updated_action = Select::new(
            "Enter the action of the transfer",
            TRANSFER_OPTIONS.to_vec(),
        )
        .with_starting_filter_input(starting_transfer_filter)
        .prompt()
        .unwrap();

        let (update_transfer_type, ptype) = match updated_action {
            "Deposit from External Account" => (
                TransferType::DepositFromExternalAccount,
                ParticipantType::Payer,
            ),
            "Deposit from Internal Account" => (
                TransferType::DepositFromInternalAccount,
                ParticipantType::Payer,
            ),
            "Withdrawal to External Account" => (
                TransferType::WithdrawalToExternalAccount,
                ParticipantType::Payee,
            ),
            "Withdrawal to Internal Account" => (
                TransferType::WithdrawalToInternalAccount,
                ParticipantType::Payee,
            ),
            _ => {
                panic!("Unrecognized pattern!")
            }
        };

        let updated_payee = Text::new("Enter payee:")
            .with_default(
                self.db
                    .get_participant(self.uid, self.id, selected_record.info.participant)
                    .unwrap()
                    .as_str(),
            )
            .with_autocomplete(ParticipantAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
                ptype: ptype.clone(),
            })
            .prompt()
            .unwrap();

        let updated_pid = self
            .db
            .check_and_add_participant(self.uid, self.id, updated_payee, ptype);

        let updated_category = Text::new("Enter category:")
            .with_default(
                self.db
                    .get_category_name(self.uid, self.id, selected_record.info.category_id)
                    .unwrap()
                    .as_str()
            )
            .with_autocomplete(CategoryAutoCompleter {
                uid :self.uid,
                aid: self.id,
                db: self.db.clone(),
            })
            .prompt()
            .unwrap()
            .to_ascii_uppercase();

        let updated_cid = self.db.check_and_add_category(self.uid, self.id, updated_category);

        let updated_description = Text::new("Enter description:")
            .with_default(&selected_record.info.description)
            .prompt()
            .unwrap();

        let updated_entry = LedgerRecord {
            id: selected_record.id,
            info: LedgerInfo {
                date: updated_date,
                amount: updated_amount,
                transfer_type: update_transfer_type,
                participant: updated_pid,
                category_id: updated_cid,
                description: updated_description,
                ancillary_f32data : 0.0
            },
        };

        self.db.update_ledger_item(self.uid, updated_entry.clone()).unwrap();

        // check if link was made
        let link_if_exists = self
            .db
            .check_and_get_account_transaction_record_matching_from_ledger_id(self.uid,selected_record.id)
            .unwrap();
        if link_if_exists.is_some() {
            let record = link_if_exists.unwrap();
            self.db.remove_account_transaction(self.uid, record.id).unwrap();
            self.db.remove_ledger_item(self.uid, record.info.to_ledger).unwrap();

            let link = Confirm::new("Link transaction to another account?")
                .prompt()
                .unwrap();

            if link {
                self.link_transaction(updated_entry.clone());
            }
        }
        return updated_entry;
    }

    // returns uid of selected ledger entry
    pub fn select_ledger_entry(&mut self) -> Option<LedgerRecord> {
        let records = self.db.get_ledger(self.uid, self.id).unwrap();
        let mut entries: HashMap<String, u32> = HashMap::new();
        let mut strings: Vec<String> = Vec::new();
        let mut mapped_records: HashMap<u32, LedgerInfo> = HashMap::new();
        for rcrd in records {
            let v: String = format!(
                "{} | {} | {} | {} | ",
                rcrd.info.date,
                self.db
                    .get_category_name(self.uid, self.id, rcrd.info.category_id)
                    .unwrap(),
                self.db
                    .get_participant(self.uid, self.id, rcrd.info.participant)
                    .unwrap(),
                rcrd.info.amount
            );
            strings.push(v.clone());
            entries.insert(v.clone(), rcrd.id);
            mapped_records.insert(rcrd.id, rcrd.info);
        }
        strings.push("None".to_string());
        let errant_record: String = Select::new("What item would you like to modify: ", strings)
            .prompt()
            .unwrap()
            .to_string();

        if errant_record == "None".to_string() {
            return None;
        }

        let id = *entries
            .get(&errant_record)
            .expect("Unable to find matching ID!");

        let selected_record = LedgerRecord {
            id: id.clone(),
            info: mapped_records
                .get(&id)
                .expect("Record not found!")
                .to_owned(),
        };
        Some(selected_record)
    }

    pub fn link_transaction(&mut self, entry: LedgerRecord) {
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

    pub fn get_current_value(&mut self) -> f32 {
        return self.db.get_current_value(self.uid, self.id).unwrap();
    }

    pub fn simple_rate_of_return(&mut self, start_date: NaiveDate, end_date: NaiveDate) -> f32 {
        let mut rate: f32 = 0.0;
        let starting_amount = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.uid, self.id, start_date)
            .unwrap();
        let ending_amount: f32 = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.uid, self.id, end_date)
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
            .get_cumulative_total_of_ledger_before_date(self.uid, self.id, start_date)
            .unwrap();
        let ending_amount: f32 = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.uid, self.id, end_date)
            .unwrap();
        let date_diff: i32 = end_date.num_days_from_ce() - start_date.num_days_from_ce();
        let year_diff: f32 = date_diff as f32 / 365.0;

        rate = (ending_amount / starting_amount).powf(1 as f32 / year_diff);
        return rate;
    }
}
