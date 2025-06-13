use crate::database::DbConn;
use crate::tui::{create_new_account, decode_and_create_account_type};
use crate::types::accounts::AccountRecord;
use crate::types::categories::CategoryAutoCompleter;
use crate::types::ledger::{LedgerInfo, LedgerRecord};
use crate::types::participants::{ParticipantAutoCompleter, ParticipantType};
use chrono::{Datelike, NaiveDate};
use inquire::validator::MinLengthValidator;
use inquire::*;
use shared_lib::{LedgerEntry, TransferType};
use core::panic;
use std::collections::HashMap;
use std::hash::Hash;

use super::{Account, AccountOperations};

pub struct ChargeAccount {
    pub id: u32,
    pub uid: u32,
    pub db: DbConn,
}

impl ChargeAccount {
    pub fn new(uid: u32, id: u32, db: DbConn) -> Self {
        let acct = Self {
            uid: uid,
            id: id,
            db: db,
        };
        acct
    }

    pub fn charge(&mut self, initial_opt : Option<LedgerRecord>, overwrite : bool) {
        let default_to_use : bool;
        let mut initial = LedgerRecord { id : 0, info : LedgerInfo { date: "1970-01-01".to_string(), amount: 0.0, transfer_type: TransferType::WithdrawalToExternalAccount, participant: 0, category_id: 0, description: "".to_string(), ancillary_f32data: 0.0 }};

        if initial_opt.is_some() { 
            default_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            default_to_use = false;
        }

        let date_prompt = "Enter date of charge:";
        let date_input = if default_to_use { 
            DateSelect::new(date_prompt)
            .with_default(NaiveDate::parse_from_str(&initial.info.date, "%Y-%m-%d").unwrap())
            .prompt()
            .unwrap()
            .format("%Y-%m-%d")
            .to_string()
        } else { 
            DateSelect::new(date_prompt)
            .prompt()
            .unwrap()
            .format("%Y-%m-%d")
            .to_string()
        };

        let amount_prompt = "Enter amount charged:";
        let amount_input: f32 = if default_to_use { 
            CustomType::<f32>::new(amount_prompt)
            .with_placeholder("00000.00")
            .with_default(initial.info.amount)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap()
        } else { 
            CustomType::<f32>::new(amount_prompt)
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap()  
        };

        let cid;
        let category_prompt = "Enter category:";
        let selected_category = if default_to_use { 
            Text::new(category_prompt)
            .with_autocomplete(CategoryAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
            })
            .with_default(self.db.get_category_name(self.uid, self.id, initial.info.category_id).unwrap().as_str())
            .prompt()
            .unwrap()
            .to_ascii_uppercase()
        } else { 
            Text::new(category_prompt)
            .with_autocomplete(CategoryAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
            })
            .prompt()
            .unwrap()
            .to_ascii_uppercase()
        };

        cid = self.db.check_and_add_category(self.uid, self.id, selected_category);

        let description_prompt = "Enter description:";
        let description_input = if default_to_use { 
            Text::new(description_prompt)
            .with_default(&initial.info.description)
            .prompt()
            .unwrap()
            .to_string()
        } else { 
            Text::new(description_prompt)
            .prompt()
            .unwrap()
            .to_string()
        };

        let selected_payee;
        let mut acct: Box<dyn Account>;
        let pid;
        let payee_prompt = "Enter payee:";

        selected_payee = if default_to_use { 
            Text::new(payee_prompt)
            .with_autocomplete(ParticipantAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
                ptype: ParticipantType::Payee,
                with_accounts : false
            })
            .with_default(self.db.get_participant(self.uid, self.id, initial.info.participant).unwrap().as_str())
            .prompt()
            .unwrap()
        } else {
            Text::new(payee_prompt)
            .with_autocomplete(ParticipantAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
                ptype: ParticipantType::Payee,
                with_accounts : false
            })
            .prompt()
            .unwrap()
        };
        pid = self.db.check_and_add_participant(self.uid, self.id, selected_payee, ParticipantType::Payee, false);

        let withdrawal = LedgerInfo {
            date: date_input,
            amount: amount_input,
            transfer_type: TransferType::WithdrawalToExternalAccount,
            participant: pid,
            category_id: cid,
            description: description_input,
            ancillary_f32data : 0.0
        };

        let id = if default_to_use && overwrite { 
            self.db.update_ledger_item(self.uid, self.id, LedgerRecord { id: initial.id, info: withdrawal.clone()} ).unwrap()
        } else { 
            self.db.add_ledger_entry(self.uid, self.id, withdrawal.clone()).unwrap()
        };
    }

    pub fn pay(&mut self, initial_opt : Option<LedgerRecord>, overwrite : bool) {

        let default_to_use : bool;
        let mut initial = LedgerRecord { id : 0, info : LedgerInfo { date: "1970-01-01".to_string(), amount: 0.0, transfer_type: TransferType::DepositFromExternalAccount, participant: 0, category_id: 0, description: "".to_string(), ancillary_f32data: 0.0 }};

        if initial_opt.is_some() { 
            default_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            default_to_use = false;
        }

        let date_prompt = "Enter date of payment:";
        let date_input = if default_to_use { 
            DateSelect::new(date_prompt)
            .with_default(NaiveDate::parse_from_str(&initial.info.date, "%Y-%m-%d").unwrap())
            .prompt()
            .unwrap()
            .format("%Y-%m-%d")
            .to_string()
        } else { 
            DateSelect::new(date_prompt)
            .prompt()
            .unwrap()
            .format("%Y-%m-%d")
            .to_string()
        };

        let amount_prompt = "Enter amount paid:";
        let amount_input: f32 = if default_to_use { 
            CustomType::<f32>::new(amount_prompt)
            .with_placeholder("00000.00")
            .with_default(initial.info.amount)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap()
        } else { 
            CustomType::<f32>::new(amount_prompt)
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap()  
        };

        let cid;
        let category_validator = MinLengthValidator::new(3).with_message("Category cannot be empty!");
        let category_prompt = "Enter category:";
        let selected_category = if default_to_use { 
            Text::new(category_prompt)
            .with_autocomplete(CategoryAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
            })
            .with_default(self.db.get_category_name(self.uid, self.id, initial.info.category_id).unwrap().as_str())
            .with_validator(category_validator)
            .prompt()
            .unwrap()
            .to_ascii_uppercase()
            .trim()
            .to_string()
        } else { 
            Text::new(category_prompt)
            .with_autocomplete(CategoryAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
            })
            .with_validator(category_validator)
            .prompt()
            .unwrap()
            .to_ascii_uppercase()
            .trim()
            .to_string()
        };

        cid = self.db.check_and_add_category(self.uid, self.id, selected_category);

        let description_prompt = "Enter description:";
        let description_input = if default_to_use { 
            Text::new(description_prompt)
            .with_default(&initial.info.description)
            .prompt()
            .unwrap()
            .to_string()
            .trim()
            .to_string()
        } else { 
            Text::new(description_prompt)
            .prompt()
            .unwrap()
            .to_string()
            .trim()
            .to_string()
        };
        
        let link = Confirm::new("Link payment to another account?")
            .prompt()
            .unwrap();

        let selected_payer;
        let mut acct: Box<dyn Account>;
        let pid;
        let participant_validator = MinLengthValidator::new(1).with_message("Payer cannot be empty!");
        if !link { 
            selected_payer = if default_to_use { 
                Text::new("Enter payer:")
                .with_autocomplete(ParticipantAutoCompleter {
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    ptype: ParticipantType::Payer,
                    with_accounts : false
                })
                .with_default(self.db.get_participant(self.uid, self.id, initial.info.participant).unwrap().as_str())
                .with_validator(participant_validator)
                .prompt()
                .unwrap()
                .trim()
                .to_string()
            } else {
                Text::new("Enter payer:")
                .with_autocomplete(ParticipantAutoCompleter {
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    ptype: ParticipantType::Payer,
                    with_accounts : false
                })
                .with_validator(participant_validator)
                .prompt()
                .unwrap()
                .trim()
                .to_string()
            };
            pid = self.db
                .check_and_add_participant(self.uid, self.id, selected_payer, ParticipantType::Payer, false);

            let deposit = LedgerInfo {
                date: date_input,
                amount: amount_input,
                transfer_type: TransferType::DepositFromExternalAccount,
                participant: pid,
                category_id: cid,
                description: description_input,
                ancillary_f32data : 0.0
            };
    
            let id = if default_to_use && overwrite { 
                self.db.update_ledger_item(self.uid, self.id, LedgerRecord { id: initial.id, info: deposit.clone() } ).unwrap()
            } else {  
                self.db.add_ledger_entry(self.uid, self.id, deposit.clone()).unwrap()
            };
        } else {
            
            let initial_account_opt = if default_to_use {
                self.db.get_participant(self.uid, self.id, initial.info.participant)
            } else { 
                None
            };

            let user_input = self.link_transaction(initial_account_opt);
            if user_input.is_none() { 
                return;
            }
            (acct, selected_payer) = user_input.unwrap();
            pid = self.db
                .check_and_add_participant(self.uid, self.id, selected_payer, ParticipantType::Both, true);

            let deposit = LedgerInfo {
                date: date_input,
                amount: amount_input,
                transfer_type: TransferType::DepositFromExternalAccount,
                participant: pid,
                category_id: cid,
                description: description_input,
                ancillary_f32data : 0.0
            };
    
            let id = if default_to_use && overwrite  {
                self.db.update_ledger_item(self.uid, self.id, LedgerRecord { id:initial.id, info: deposit.clone() }).unwrap()
            } else {
                self.db.add_ledger_entry(self.uid, self.id, deposit.clone()).unwrap()
            };
            
            let entry = LedgerRecord {
                id: id,
                info: deposit,
            };
    
            if link {
                acct.link(self.id, entry);
            }
        }

    }

    pub fn modify(&mut self, selected_record: LedgerRecord) -> LedgerRecord {

        let was_payment = match selected_record.info.transfer_type.clone() { 
            TransferType::DepositFromExternalAccount|TransferType::DepositFromInternalAccount => { true }
            TransferType::WithdrawalToInternalAccount|TransferType::WithdrawalToExternalAccount => { false }
            TransferType::ZeroSumChange => { println!("Unable to modify a zero-sum change!"); return selected_record }
        };

        const OPTIONS: [&'static str; 3] = ["Update", "Remove", "None"];
        let modify_choice = Select::new("What would you like to do:", OPTIONS.to_vec()).prompt().unwrap();
        match modify_choice { 
            "Update" => {
                let account_transaction_opt: Option<crate::types::accounts::AccountTransactionRecord>;
                if was_payment { 
                    account_transaction_opt = self.db.check_and_get_account_transaction_record_matching_to_ledger_id(self.uid, self.id, selected_record.id).unwrap();
                    if account_transaction_opt.is_some() { 
                        let account_transaction = account_transaction_opt.unwrap();
                        self.db.remove_account_transaction(self.uid, account_transaction.id).unwrap();
                        self.db.remove_ledger_item(self.uid, account_transaction.info.from_account, account_transaction.info.from_ledger).unwrap();
                    }
                    self.pay(Some(selected_record.clone()), true);
                } else { 
                    account_transaction_opt = self.db.check_and_get_account_transaction_record_matching_from_ledger_id(self.uid, self.id, selected_record.id).unwrap();
                    if account_transaction_opt.is_some() { 
                        let account_transaction = account_transaction_opt.unwrap();
                        self.db.remove_ledger_item(self.uid, account_transaction.info.to_account, account_transaction.info.to_ledger).unwrap();
                    }                    
                    self.charge(Some(selected_record.clone()), true);
                }
            }
            "Remove" => { 
                let account_transaction_opt: Option<crate::types::accounts::AccountTransactionRecord>;
                if was_payment { 
                    account_transaction_opt = self.db.check_and_get_account_transaction_record_matching_to_ledger_id(self.uid, self.id, selected_record.id).unwrap();
                    if account_transaction_opt.is_some() { 
                        let account_transaction = account_transaction_opt.unwrap();
                        self.db.remove_account_transaction(self.uid, account_transaction.id).unwrap();
                        self.db.remove_ledger_item(self.uid, account_transaction.info.from_account, account_transaction.info.from_ledger).unwrap();
                    }
                } else { 
                    account_transaction_opt = self.db.check_and_get_account_transaction_record_matching_from_ledger_id(self.uid, self.id, selected_record.id).unwrap();
                    if account_transaction_opt.is_some() { 
                        let account_transaction = account_transaction_opt.unwrap();
                        self.db.remove_account_transaction(self.uid, account_transaction.id).unwrap();
                        self.db.remove_ledger_item(self.uid, account_transaction.info.to_account, account_transaction.info.to_ledger).unwrap();
                    }                    
                }
                self.db.remove_ledger_item(self.uid, self.id, selected_record.id.clone()).unwrap();
            }
            "None" => { 
                return selected_record.clone();
            }
            _ =>  { 
                panic!("Unrecognized input!");
            }
        }

        return selected_record;
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

    pub fn link_transaction(&mut self, initial_opt : Option<String>) -> Option<(Box<dyn Account>, String)> {

        let default_to_use;
        let mut initial_account = String::new();
        if initial_opt.is_some() { 
            default_to_use = true;
            initial_account = initial_opt.unwrap();
        } else { 
            default_to_use = false;
        }

        let accounts = self.db.get_user_accounts(self.uid).unwrap();
        let mut account_map: HashMap<String, AccountRecord> = HashMap::new();
        let mut account_names: Vec<String> = Vec::new();
        for account in accounts.iter() {
            account_names.push(account.info.name.clone());
            account_map.insert(account.info.name.clone(), account.clone());
        }

        let select_account_prompt = "Select account:";
        let mut selected_account = if default_to_use {
            Text::new(select_account_prompt)
            .with_autocomplete(ParticipantAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
                ptype: ParticipantType::Both,
                with_accounts : true
            })         
            .with_default(initial_account.as_str())       
            .prompt()
            .unwrap()
        } else { 
            Text::new(select_account_prompt)
            .with_autocomplete(ParticipantAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
                ptype: ParticipantType::Both,
                with_accounts : true
            })                
            .prompt()
            .unwrap()
        };

        if selected_account.clone() == "None" {
            return None;
        }

        let acct: Box<dyn Account>;
        let record: AccountRecord;
        if selected_account.clone() == "New Account".to_ascii_uppercase().to_string() {
            let user_input = create_new_account(self.uid, &mut self.db);
            if user_input.is_none() { 
                return None;
            }
            (acct, record) = user_input.unwrap();
            selected_account = record.info.name;
        } else {
            let acctx = account_map
                .get(&selected_account)
                .expect("Account not found!");
            acct = decode_and_create_account_type(self.uid, &mut self.db, acctx);
        }

        return Some((acct, selected_account.clone()));
    }

    pub fn get_current_balance(&mut self) -> f32 {
        // because a credit card is debt, return it as a positive balance. 
        return -self.db.get_current_value(self.uid, self.id).unwrap();
    }
 
}
