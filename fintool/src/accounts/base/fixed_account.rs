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
use crate::types::accounts::AccountRecord;
use crate::accounts::base::{AccountFileIO, LedgerOps, Valuable};
use crate::accounts::growth::{GrowthCalculable, GrowthMetric, compound_annual_growth_rate, money_weighted_return, simple_rate_of_return};
use crate::types::categories::CategoryAutoCompleter;
use crate::types::labels::LabelAutoCompleter;
use crate::types::ledger::{LedgerInfo, LedgerRecord};
use crate::types::participants::{ParticipantAutoCompleter, ParticipantType};
use chrono::{Datelike, Days, NaiveDate};
use core::{f32, panic};
use csv::ReaderBuilder;
use inquire::validator::MinLengthValidator;
use inquire::*;
use shared_lib::{FlatLedgerEntry, LedgerEntry, TransferType};
use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;

use rustyline::completion::FilenameCompleter;
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::Completer;
use rustyline::CompletionType;
use rustyline::Config;
use rustyline::EditMode;
use rustyline::Editor;
use rustyline::Helper;
use rustyline::Highlighter;
use rustyline::Hinter;
use rustyline::Validator;

use super::{Account, AccountContext, HasContext};

#[derive(Helper, Completer, Hinter, Highlighter, Validator)]
pub struct FilePathHelper {
    #[rustyline(Completer)]
    pub completer: FilenameCompleter,
    #[rustyline(Highlighter)]
    pub highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    pub validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    pub hinter: HistoryHinter,
    pub colored_prompt: String,
}


pub trait FixedAccount : HasContext + LedgerOps {
    fn withdrawal(&self, initial_opt: Option<LedgerRecord>, overwrite: bool) -> LedgerRecord {

        let ctx = Self::ctx(&self);

        let default_to_use: bool;
        let mut initial = LedgerRecord {
            id: 0,
            info: LedgerInfo {
                date: "1970-01-01".to_string(),
                amount: 0.0,
                transfer_type: TransferType::WithdrawalToExternalAccount,
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

        let date_prompt = "Enter date of withdrawal:";
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

        let amount_prompt = "Enter amount withdrew:";
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
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
        } else {
            Text::new(category_prompt)
                .with_autocomplete(CategoryAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    cats: None,
                })
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
        };

        cid = ctx.db
            .check_and_add_category(ctx.uid, ctx.aid, selected_category);

        let description_prompt = "Enter description:";
        let description_input = if default_to_use {
            Text::new(description_prompt)
                .with_default(&initial.info.description)
                .prompt()
                .unwrap()
                .to_string()
        } else {
            Text::new(description_prompt).prompt().unwrap().to_string()
        };

        let link = Confirm::new("Link transaction to another account?")
            .prompt()
            .unwrap();

        let selected_payee;
        let mut acct: Box<dyn Account>;
        let pid;
        let payee_prompt = "Enter payee:";
        if !link {
            selected_payee = if default_to_use {
                Text::new(payee_prompt)
                    .with_autocomplete(ParticipantAutoCompleter {
                        uid: ctx.uid,
                        aid: ctx.aid,
                        db: ctx.db.clone(),
                        ptype: ParticipantType::Payee,
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
                    .prompt()
                    .unwrap()
            } else {
                Text::new(payee_prompt)
                    .with_autocomplete(ParticipantAutoCompleter {
                        uid: ctx.uid,
                        aid: ctx.aid,
                        db: ctx.db.clone(),
                        ptype: ParticipantType::Payee,
                        with_accounts: false,
                        stock_tickers_only: false,
                        manually_recorded_only: false,
                    })
                    .prompt()
                    .unwrap()
            };
            pid = ctx.db.check_and_add_participant(
                ctx.uid,
                ctx.aid,
                selected_payee,
                ParticipantType::Payee,
                false,
            );

            let withdrawal = LedgerInfo {
                date: date_input,
                amount: amount_input,
                transfer_type: TransferType::WithdrawalToExternalAccount,
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
                            info: withdrawal.clone(),
                        },
                    )
                    .unwrap()
            } else {
                ctx.db
                    .add_ledger_entry(ctx.uid, ctx.aid, withdrawal.clone())
                    .unwrap()
            };

            if overwrite {
                let maintain_labels =
                    Confirm::new("Would you like to maintain all prior labels (y/n)?")
                        .prompt()
                        .unwrap();
                if !maintain_labels {
                    let mapped_labels = ctx.db
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
            let add_label_prompt = Confirm::new("Add labels to withdrawal (y/n)?")
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
                info: withdrawal,
            };
        } else {
            let initial_account_opt = if default_to_use {
                ctx.db
                    .get_participant(ctx.uid, ctx.aid, initial.info.participant)
            } else {
                None
            };

            let user_input = self.link_transaction(initial_account_opt);
            if user_input.is_none() {
                return initial;
            }
            (acct, selected_payee) = user_input.unwrap();
            pid = ctx.db.check_and_add_participant(
                ctx.uid,
                ctx.aid,
                selected_payee,
                ParticipantType::Both,
                true,
            );

            let withdrawal = LedgerInfo {
                date: date_input,
                amount: amount_input,
                transfer_type: TransferType::WithdrawalToExternalAccount,
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
                            info: withdrawal.clone(),
                        },
                    )
                    .unwrap()
            } else {
                ctx.db
                    .add_ledger_entry(ctx.uid, ctx.aid, withdrawal.clone())
                    .unwrap()
            };

            let entry = LedgerRecord {
                id: id,
                info: withdrawal,
            };

            if overwrite {
                let maintain_labels =
                    Confirm::new("Would you like to maintain all prior labels (y/n)?")
                        .prompt()
                        .unwrap();
                if !maintain_labels {
                    let mapped_labels = ctx.db
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
            let add_label_prompt = Confirm::new("Add labels to withdrawal (y/n)?")
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
                        .add_label_mapping(ctx.uid, ctx.aid, label_id, entry.id)
                        .unwrap();

                    let continue_prompt = Confirm::new("Add more labels (y/n)?").prompt().unwrap();
                    if !continue_prompt {
                        break;
                    }
                }
            }

            if link {
                acct.link(ctx.aid, entry.clone());
            }

            return entry;
        }
    }

    fn deposit(&self, initial_opt: Option<LedgerRecord>, overwrite: bool) -> LedgerRecord {
        let ctx = Self::ctx(&self);

        let default_to_use: bool;
        let mut initial = LedgerRecord {
            id: 0,
            info: LedgerInfo {
                date: "1970-01-01".to_string(),
                amount: 0.0,
                transfer_type: TransferType::DepositFromExternalAccount,
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

        let date_prompt = "Enter date of deposit:";
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

        let amount_prompt = "Enter amount deposited:";
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

        cid = ctx.db
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

        let link = Confirm::new("Link transaction to another account?")
            .prompt()
            .unwrap();

        let selected_payer;
        let mut acct: Box<dyn Account>;
        let pid;
        let participant_validator =
            MinLengthValidator::new(1).with_message("Payer cannot be empty!");
        if !link {
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
                transfer_type: TransferType::DepositFromExternalAccount,
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
                    let mapped_labels = ctx.db
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
        } else {
            let initial_account_opt = if default_to_use {
                ctx.db
                    .get_participant(ctx.uid, ctx.aid, initial.info.participant)
            } else {
                None
            };

            let user_input = self.link_transaction(initial_account_opt);
            if user_input.is_none() {
                return initial;
            }
            (acct, selected_payer) = user_input.unwrap();
            pid = ctx.db.check_and_add_participant(
                ctx.uid,
                ctx.aid,
                selected_payer,
                ParticipantType::Both,
                true,
            );

            let deposit = LedgerInfo {
                date: date_input,
                amount: amount_input,
                transfer_type: TransferType::DepositFromExternalAccount,
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

            let entry = LedgerRecord {
                id: id,
                info: deposit,
            };

            if overwrite {
                let maintain_labels =
                    Confirm::new("Would you like to maintain all prior labels (y/n)?")
                        .prompt()
                        .unwrap();
                if !maintain_labels {
                    let mapped_labels = ctx.db
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
                        .add_label_mapping(ctx.uid, ctx.aid, label_id, entry.id)
                        .unwrap();

                    let continue_prompt = Confirm::new("Add more labels (y/n)?").prompt().unwrap();
                    if !continue_prompt {
                        break;
                    }
                }
            }

            if link {
                acct.link(ctx.aid, entry.clone());
            }

            return entry;
        }
    }
}

pub fn fixed_account_value(ctx : &AccountContext) -> Option<f32> {
    let value= ctx.db.get_current_value(ctx.uid, ctx.aid).unwrap();
    Some(value)
}

pub fn fixed_account_value_on_day(ctx : &AccountContext, day: &NaiveDate) -> Option<f32> {
    let value_opt = ctx.db
        .get_cumulative_total_of_ledger_before_date(ctx.uid, ctx.aid, *day)
        .unwrap();
    return value_opt;
}

pub trait FixedValuable: Valuable {
    fn fixed_value(&self) -> Option<f32> {
        fixed_account_value(self.ctx())
    }
    fn fixed_value_on_day(&self, day: &NaiveDate  ) -> Option<f32> {
        fixed_account_value_on_day(self.ctx(), day)
    }
}

pub trait FixedGrowth: GrowthCalculable {
    fn fixed_growth(&self, metric: GrowthMetric, start_date: NaiveDate, end_date : NaiveDate) -> f32 {
        match metric {
            GrowthMetric::CAGR => {
                compound_annual_growth_rate(self, start_date, end_date)
            }
            GrowthMetric::MWRR => {
                money_weighted_return(self, start_date, end_date)
            }
            GrowthMetric::SimpleReturn => {
                simple_rate_of_return(self, start_date, end_date)
            }
            GrowthMetric::TWRR => {
                f32::NAN
            }
        }
    }
}

pub trait FixedLedger: LedgerOps + HasContext + FixedAccount { 
    fn modify_fixed(&mut self, selected_record: LedgerRecord) -> Option<LedgerRecord> {

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
                        account_transaction_opt = ctx.db
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
                    TransferType::WithdrawalToExternalAccount => {
                        account_transaction_opt = ctx.db
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
                    _ => {
                        selected_record
                    }
                };
                return Some(updated_record);
            }
            "Remove" => {
                let account_transaction_opt: Option<
                    crate::types::accounts::AccountTransactionRecord,
                >;
                match selected_record.info.transfer_type {
                    TransferType::DepositFromExternalAccount => {
                        account_transaction_opt = ctx.db
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
                        account_transaction_opt = ctx.db
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
                    _ => {},
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

pub trait FixedAccountFileIO : AccountFileIO {

    fn import_fixed_account(&self) {
        let ctx = self.ctx();
        let g = FilePathHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter::new(),
            validator: MatchingBracketValidator::new(),
            colored_prompt: "".to_owned(),
        };
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Vi)
            .build();
        let mut rl = Editor::with_config(config).unwrap();
        rl.set_helper(Some(g));

        let mut fp = Path::new("~");
        let mut bad_path;
        let mut csv: String = String::new();
        loop {
            csv = rl
                .readline("Enter path to CSV file (or none to quit): ")
                .unwrap();
            if csv.to_string() == "none" {
                return;
            }
            bad_path = match Path::new(&csv).try_exists() {
                Ok(true) => false,
                Ok(false) => {
                    println!("File {} cannot be found!", Path::new(&csv).display());
                    true
                }
                Err(e) => {
                    println!("File {} cannot be found: {}!", e, Path::new(&csv).display());
                    true
                }
            };
            if !bad_path {
                break;
            } else {
                let try_again = Confirm::new("Continue import?").prompt().unwrap();
                if !try_again {
                    return;
                }
            }
        }
        fp = Path::new(&csv);

        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_path(fp)
            .unwrap();

        let mut ledger_entries = Vec::new();
        for result in rdr.deserialize::<LedgerEntry>() {
            ledger_entries.push(result.unwrap());
        }
        ledger_entries.sort_by(|x, y| {
            (NaiveDate::parse_from_str(&x.date, "%Y-%m-%d").unwrap())
                .cmp(&NaiveDate::parse_from_str(&y.date, "%Y-%m-%d").unwrap())
        });
        for rcrd in ledger_entries {
            let ptype = if rcrd.transfer_type == TransferType::WithdrawalToExternalAccount {
                ParticipantType::Payee
            } else if rcrd.transfer_type == TransferType::WithdrawalToInternalAccount {
                ParticipantType::Payee
            } else if rcrd.transfer_type == TransferType::DepositFromExternalAccount {
                ParticipantType::Payer
            } else {
                ParticipantType::Payer
            };
            let entry: LedgerInfo = LedgerInfo {
                date: NaiveDate::parse_from_str(rcrd.date.as_str(), "%Y-%m-%d")
                    .unwrap()
                    .format("%Y-%m-%d")
                    .to_string(),
                amount: rcrd.amount,
                transfer_type: rcrd.transfer_type as TransferType,
                participant: ctx.db.check_and_add_participant(
                    ctx.uid,
                    ctx.aid,
                    rcrd.participant,
                    ptype,
                    false,
                ),
                category_id: ctx.db.check_and_add_category(
                    ctx.uid,
                    ctx.aid,
                    rcrd.category.to_ascii_uppercase(),
                ),
                description: rcrd.description,
            };
            let _lid: u32 = ctx.db.add_ledger_entry(ctx.uid, ctx.aid, entry).unwrap();
        }
    }

    fn export_fixed_account(&self) {
        let ctx = self.ctx();
        let g = FilePathHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter::new(),
            validator: MatchingBracketValidator::new(),
            colored_prompt: "".to_owned(),
        };
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Vi)
            .build();
        let mut rl = Editor::with_config(config).unwrap();
        rl.set_helper(Some(g));

        let mut wtr =
            csv::Writer::from_path(rl.readline("Enter path to CSV file: ").unwrap()).unwrap();
        let ledger = self.get_ledger();
        if !ledger.is_empty() {
            for record in ledger {
                let csv_ledger_record: shared_lib::LedgerEntry = LedgerEntry {
                    date: record.info.date,
                    amount: record.info.amount,
                    transfer_type: record.info.transfer_type,
                    participant: ctx.db
                        .get_participant(ctx.uid, ctx.aid, record.info.participant)
                        .unwrap(),
                    category: ctx.db
                        .get_category_name(ctx.uid, ctx.aid, record.info.category_id)
                        .unwrap(),
                    description: record.info.description,
                    stock_info: None,
                };
                let flattened = FlatLedgerEntry::from(csv_ledger_record);
                wtr.serialize(flattened).unwrap();
            }
        }
    }
}

