/* ------------------------------------------------------------------------
  Copyright (C) 2025  Andrew J. Eberhard

  This program is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  This program is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <https://www.gnu.org/licenses/>.
-----------------------------------------------------------------------*/
use crate::database::DbConn;
use crate::tui::{decode_and_init_account_type, prompt_and_create_new_account};
use crate::types::accounts::AccountRecord;
use crate::types::categories::CategoryAutoCompleter;
use crate::types::labels::LabelAutoCompleter;
use crate::types::ledger::{LedgerInfo, LedgerRecord};
use crate::types::participants::{ParticipantAutoCompleter, ParticipantType};
use chrono::{Datelike, Days, NaiveDate};
use core::{f32, panic};
use inquire::validator::MinLengthValidator;
use inquire::*;
use shared_lib::{LedgerEntry, TransferType};
use std::collections::HashMap;
use std::hash::Hash;

use super::{Account, AccountContext, HasContext, LedgerOps};
use crate::accounts::base::fixed_account::FixedAccount;
use crate::accounts::base::{Valuable, ValueLimited};

pub trait InterestBearingFixedAccount: HasContext + LedgerOps + FixedAccount {
    fn fee(&self, initial_opt: Option<LedgerRecord>, overwrite: bool) -> LedgerRecord {
        let ctx = Self::ctx(&self);

        let default_to_use: bool;
        let mut initial = LedgerRecord {
            id: 0,
            info: LedgerInfo {
                date: "1970-01-01".to_string(),
                amount: 0.0,
                transfer_type: TransferType::WithdrawalToInternalAccount,
                participant: 0,
                category_id: 0,
                description: "".to_string(),
            },
        };

        if initial_opt.is_some() {
            default_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            default_to_use = false;
        }

        let date_prompt = "Enter date of fee:";
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

        let amount_prompt = "Enter fee charged:";
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
        let category_validator =
            MinLengthValidator::new(3).with_message("Category cannot be empty!");
        let category_prompt = "Enter category:";
        let selected_category = if default_to_use {
            Text::new(category_prompt)
                .with_autocomplete(CategoryAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    cats: None,
                })
                .with_default(
                    ctx.db
                        .get_category_name(ctx.uid, ctx.aid, initial.info.category_id)
                        .unwrap()
                        .as_str(),
                )
                .with_validator(category_validator)
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string()
        } else {
            Text::new(category_prompt)
                .with_autocomplete(CategoryAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    cats: None,
                })
                .with_validator(category_validator)
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string()
        };

        cid = ctx
            .db
            .check_and_add_category(ctx.uid, ctx.aid, selected_category);

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

        let selected_payer;
        let pid;
        let participant_validator =
            MinLengthValidator::new(1).with_message("Payer cannot be empty!");

        selected_payer = if default_to_use {
            Text::new("Enter payer:")
                .with_autocomplete(ParticipantAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    ptype: ParticipantType::Payer,
                    with_accounts: false,
                    stock_tickers_only: false,
                    manually_recorded_only: false,
                })
                .with_default(
                    ctx.db
                        .get_participant(ctx.uid, ctx.aid, initial.info.participant)
                        .unwrap()
                        .as_str(),
                )
                .with_validator(participant_validator)
                .prompt()
                .unwrap()
                .trim()
                .to_string()
        } else {
            Text::new("Enter payer:")
                .with_autocomplete(ParticipantAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    ptype: ParticipantType::Payer,
                    with_accounts: false,
                    stock_tickers_only: false,
                    manually_recorded_only: false,
                })
                .with_validator(participant_validator)
                .prompt()
                .unwrap()
                .trim()
                .to_string()
        };
        pid = ctx.db.check_and_add_participant(
            ctx.uid,
            ctx.aid,
            selected_payer,
            ParticipantType::Payer,
            false,
        );

        let deposit = LedgerInfo {
            date: date_input,
            amount: amount_input,
            transfer_type: TransferType::WithdrawalToInternalAccount,
            participant: pid,
            category_id: cid,
            description: description_input,
        };

        let id = if default_to_use && overwrite {
            ctx.db
                .update_ledger_item(
                    ctx.uid,
                    ctx.aid,
                    LedgerRecord {
                        id: initial.id,
                        info: deposit.clone(),
                    },
                )
                .unwrap()
        } else {
            ctx.db
                .add_ledger_entry(ctx.uid, ctx.aid, deposit.clone())
                .unwrap()
        };

        if overwrite {
            let maintain_labels =
                Confirm::new("Would you like to maintain all prior labels (y/n)?")
                    .prompt()
                    .unwrap();
            if !maintain_labels {
                let mapped_labels = ctx
                    .db
                    .check_and_get_label_mapping_matching_ledger_id(ctx.uid, ctx.aid, id)
                    .unwrap();
                if !mapped_labels.is_empty() {
                    for label in mapped_labels {
                        ctx.db
                            .remove_label_mapping(ctx.uid, ctx.aid, label.id)
                            .unwrap();
                    }
                }
            }
        }

        // add labels for transaction
        let add_label_prompt = Confirm::new("Add labels to deposit (y/n)?")
            .prompt()
            .unwrap();
        if add_label_prompt == true {
            loop {
                let label = Text::new("Enter label:")
                    .with_autocomplete(LabelAutoCompleter {
                        uid: ctx.uid,
                        db: ctx.db.clone(),
                    })
                    .prompt()
                    .unwrap()
                    .to_ascii_uppercase();
                let label_id = ctx.db.check_and_add_label(ctx.uid, label).unwrap();
                ctx.db
                    .add_label_mapping(ctx.uid, ctx.aid, label_id, id)
                    .unwrap();

                let continue_prompt = Confirm::new("Add more labels (y/n)?").prompt().unwrap();
                if !continue_prompt {
                    break;
                }
            }
        }

        return LedgerRecord {
            id: id,
            info: deposit,
        };
    }

    fn accrual(&self, initial_opt: Option<LedgerRecord>, overwrite: bool) -> LedgerRecord {
        let ctx = Self::ctx(&self);
        let default_to_use: bool;
        let mut initial = LedgerRecord {
            id: 0,
            info: LedgerInfo {
                date: "1970-01-01".to_string(),
                amount: 0.0,
                transfer_type: TransferType::DepositFromInternalAccount,
                participant: 0,
                category_id: 0,
                description: "".to_string(),
            },
        };

        if initial_opt.is_some() {
            default_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            default_to_use = false;
        }

        let date_prompt = "Enter date of accrual:";
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

        let amount_prompt = "Enter amount accrued:";
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
        let category_validator =
            MinLengthValidator::new(3).with_message("Category cannot be empty!");
        let category_prompt = "Enter category:";
        let selected_category = if default_to_use {
            Text::new(category_prompt)
                .with_autocomplete(CategoryAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    cats: None,
                })
                .with_default(
                    ctx.db
                        .get_category_name(ctx.uid, ctx.aid, initial.info.category_id)
                        .unwrap()
                        .as_str(),
                )
                .with_validator(category_validator)
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string()
        } else {
            Text::new(category_prompt)
                .with_autocomplete(CategoryAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    cats: None,
                })
                .with_validator(category_validator)
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string()
        };

        cid = ctx
            .db
            .check_and_add_category(ctx.uid, ctx.aid, selected_category);

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

        let selected_payer;
        let pid;
        let participant_validator =
            MinLengthValidator::new(1).with_message("Payer cannot be empty!");

        selected_payer = if default_to_use {
            Text::new("Enter payer:")
                .with_autocomplete(ParticipantAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    ptype: ParticipantType::Payer,
                    with_accounts: false,
                    stock_tickers_only: false,
                    manually_recorded_only: false,
                })
                .with_default(
                    ctx.db
                        .get_participant(ctx.uid, ctx.aid, initial.info.participant)
                        .unwrap()
                        .as_str(),
                )
                .with_validator(participant_validator)
                .prompt()
                .unwrap()
                .trim()
                .to_string()
        } else {
            Text::new("Enter payer:")
                .with_autocomplete(ParticipantAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    ptype: ParticipantType::Payer,
                    with_accounts: false,
                    stock_tickers_only: false,
                    manually_recorded_only: false,
                })
                .with_validator(participant_validator)
                .prompt()
                .unwrap()
                .trim()
                .to_string()
        };
        pid = ctx.db.check_and_add_participant(
            ctx.uid,
            ctx.aid,
            selected_payer,
            ParticipantType::Payer,
            false,
        );

        let deposit = LedgerInfo {
            date: date_input,
            amount: amount_input,
            transfer_type: TransferType::DepositFromInternalAccount,
            participant: pid,
            category_id: cid,
            description: description_input,
        };

        let id = if default_to_use && overwrite {
            ctx.db
                .update_ledger_item(
                    ctx.uid,
                    ctx.aid,
                    LedgerRecord {
                        id: initial.id,
                        info: deposit.clone(),
                    },
                )
                .unwrap()
        } else {
            ctx.db
                .add_ledger_entry(ctx.uid, ctx.aid, deposit.clone())
                .unwrap()
        };

        if overwrite {
            let maintain_labels =
                Confirm::new("Would you like to maintain all prior labels (y/n)?")
                    .prompt()
                    .unwrap();
            if !maintain_labels {
                let mapped_labels = ctx
                    .db
                    .check_and_get_label_mapping_matching_ledger_id(ctx.uid, ctx.aid, id)
                    .unwrap();
                if !mapped_labels.is_empty() {
                    for label in mapped_labels {
                        ctx.db
                            .remove_label_mapping(ctx.uid, ctx.aid, label.id)
                            .unwrap();
                    }
                }
            }
        }

        // add labels for transaction
        let add_label_prompt = Confirm::new("Add labels to deposit (y/n)?")
            .prompt()
            .unwrap();
        if add_label_prompt == true {
            loop {
                let label = Text::new("Enter label:")
                    .with_autocomplete(LabelAutoCompleter {
                        uid: ctx.uid,
                        db: ctx.db.clone(),
                    })
                    .prompt()
                    .unwrap()
                    .to_ascii_uppercase();
                let label_id = ctx.db.check_and_add_label(ctx.uid, label).unwrap();
                ctx.db
                    .add_label_mapping(ctx.uid, ctx.aid, label_id, id)
                    .unwrap();

                let continue_prompt = Confirm::new("Add more labels (y/n)?").prompt().unwrap();
                if !continue_prompt {
                    break;
                }
            }
        }

        return LedgerRecord {
            id: id,
            info: deposit,
        };
    }
}

pub trait InterestBearingLedger: LedgerOps + HasContext + InterestBearingFixedAccount {
    fn modify_interest_bearing(&mut self, selected_record: LedgerRecord) -> Option<LedgerRecord> {
        let ctx = self.ctx();

        if selected_record.info.transfer_type == TransferType::ZeroSumChange {
            println!("Unable to modify a zero-sum change!");
            return None;
        }

        const OPTIONS: [&'static str; 3] = ["Update", "Remove", "None"];
        let modify_choice = Select::new("What would you like to do:", OPTIONS.to_vec())
            .prompt()
            .unwrap();
        match modify_choice {
            "Update" => {
                let account_transaction_opt: Option<
                    crate::types::accounts::AccountTransactionRecord,
                >;
                let updated_record = match selected_record.info.transfer_type {
                    TransferType::DepositFromExternalAccount => {
                        account_transaction_opt = ctx
                            .db
                            .check_and_get_account_transaction_record_matching_to_ledger_id(
                                ctx.uid,
                                ctx.aid,
                                selected_record.id,
                            )
                            .unwrap();
                        if account_transaction_opt.is_some() {
                            let account_transaction = account_transaction_opt.unwrap();
                            ctx.db
                                .remove_account_transaction(ctx.uid, account_transaction.id)
                                .unwrap();
                            ctx.db
                                .remove_ledger_item(
                                    ctx.uid,
                                    account_transaction.info.from_account,
                                    account_transaction.info.from_ledger,
                                )
                                .unwrap();
                        }
                        self.deposit(Some(selected_record.clone()), true)
                    }
                    TransferType::DepositFromInternalAccount => {
                        self.accrual(Some(selected_record.clone()), true)
                    }
                    TransferType::WithdrawalToExternalAccount => {
                        account_transaction_opt = ctx
                            .db
                            .check_and_get_account_transaction_record_matching_from_ledger_id(
                                ctx.uid,
                                ctx.aid,
                                selected_record.id,
                            )
                            .unwrap();
                        if account_transaction_opt.is_some() {
                            let account_transaction = account_transaction_opt.unwrap();
                            ctx.db
                                .remove_ledger_item(
                                    ctx.uid,
                                    account_transaction.info.to_account,
                                    account_transaction.info.to_ledger,
                                )
                                .unwrap();
                        }
                        self.withdrawal(Some(selected_record.clone()), true)
                    }
                    TransferType::WithdrawalToInternalAccount => {
                        self.fee(Some(selected_record.clone()), true)
                    }
                    _ => selected_record,
                };
                return Some(updated_record);
            }
            "Remove" => {
                let account_transaction_opt: Option<
                    crate::types::accounts::AccountTransactionRecord,
                >;
                match selected_record.info.transfer_type {
                    TransferType::DepositFromExternalAccount => {
                        account_transaction_opt = ctx
                            .db
                            .check_and_get_account_transaction_record_matching_to_ledger_id(
                                ctx.uid,
                                ctx.aid,
                                selected_record.id,
                            )
                            .unwrap();
                        if account_transaction_opt.is_some() {
                            let account_transaction = account_transaction_opt.unwrap();
                            ctx.db
                                .remove_account_transaction(ctx.uid, account_transaction.id)
                                .unwrap();
                            ctx.db
                                .remove_ledger_item(
                                    ctx.uid,
                                    account_transaction.info.from_account,
                                    account_transaction.info.from_ledger,
                                )
                                .unwrap();
                        }
                    }
                    TransferType::WithdrawalToExternalAccount => {
                        account_transaction_opt = ctx
                            .db
                            .check_and_get_account_transaction_record_matching_from_ledger_id(
                                ctx.uid,
                                ctx.aid,
                                selected_record.id,
                            )
                            .unwrap();
                        if account_transaction_opt.is_some() {
                            let account_transaction = account_transaction_opt.unwrap();
                            ctx.db
                                .remove_account_transaction(ctx.uid, account_transaction.id)
                                .unwrap();
                            ctx.db
                                .remove_ledger_item(
                                    ctx.uid,
                                    account_transaction.info.to_account,
                                    account_transaction.info.to_ledger,
                                )
                                .unwrap();
                        }
                    }
                    _ => {}
                }
                ctx.db
                    .remove_ledger_item(ctx.uid, ctx.aid, selected_record.id.clone())
                    .unwrap();
            }
            "None" => {
                return None;
            }
            _ => {
                panic!("Unrecognized input!");
            }
        }

        return Some(selected_record);
    }
}
