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
use crate::accounts::base::{BaseActions, BaseGrowth};
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

use super::{Account, AccountOperations};

pub struct FixedAccount {
    pub id: u32,
    pub uid: u32,
    pub db: DbConn,
    pub ledger: Vec<LedgerRecord>,
}

impl FixedAccount {
    pub fn new(uid: u32, id: u32, db: DbConn) -> Self {
        let acct = Self {
            uid: uid,
            id: id,
            db: db.clone(),
            ledger: db.get_ledger(uid, id).unwrap(),
        };
        acct
    }

    pub fn withdrawal(&self, initial_opt: Option<LedgerRecord>, overwrite: bool) -> LedgerRecord {
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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    cats: None,
                })
                .with_default(
                    self.db
                        .get_category_name(self.uid, self.id, initial.info.category_id)
                        .unwrap()
                        .as_str(),
                )
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
        } else {
            Text::new(category_prompt)
                .with_autocomplete(CategoryAutoCompleter {
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    cats: None,
                })
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
        };

        cid = self
            .db
            .check_and_add_category(self.uid, self.id, selected_category);

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
                        uid: self.uid,
                        aid: self.id,
                        db: self.db.clone(),
                        ptype: ParticipantType::Payee,
                        with_accounts: false,
                        stock_tickers_only: false,
                        manually_recorded_only: false,
                    })
                    .with_default(
                        self.db
                            .get_participant(self.uid, self.id, initial.info.participant)
                            .unwrap()
                            .as_str(),
                    )
                    .prompt()
                    .unwrap()
            } else {
                Text::new(payee_prompt)
                    .with_autocomplete(ParticipantAutoCompleter {
                        uid: self.uid,
                        aid: self.id,
                        db: self.db.clone(),
                        ptype: ParticipantType::Payee,
                        with_accounts: false,
                        stock_tickers_only: false,
                        manually_recorded_only: false,
                    })
                    .prompt()
                    .unwrap()
            };
            pid = self.db.check_and_add_participant(
                self.uid,
                self.id,
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
                self.db
                    .update_ledger_item(
                        self.uid,
                        self.id,
                        LedgerRecord {
                            id: initial.id,
                            info: withdrawal.clone(),
                        },
                    )
                    .unwrap()
            } else {
                self.db
                    .add_ledger_entry(self.uid, self.id, withdrawal.clone())
                    .unwrap()
            };

            if overwrite {
                let maintain_labels =
                    Confirm::new("Would you like to maintain all prior labels (y/n)?")
                        .prompt()
                        .unwrap();
                if !maintain_labels {
                    let mapped_labels = self
                        .db
                        .check_and_get_label_mapping_matching_ledger_id(self.uid, self.id, id)
                        .unwrap();
                    if !mapped_labels.is_empty() {
                        for label in mapped_labels {
                            self.db
                                .remove_label_mapping(self.uid, self.id, label.id)
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
                            uid: self.uid,
                            db: self.db.clone(),
                        })
                        .prompt()
                        .unwrap()
                        .to_ascii_uppercase();
                    let label_id = self.db.check_and_add_label(self.uid, label).unwrap();
                    self.db
                        .add_label_mapping(self.uid, self.id, label_id, id)
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
                self.db
                    .get_participant(self.uid, self.id, initial.info.participant)
            } else {
                None
            };

            let user_input = self.link_transaction(initial_account_opt);
            if user_input.is_none() {
                return initial;
            }
            (acct, selected_payee) = user_input.unwrap();
            pid = self.db.check_and_add_participant(
                self.uid,
                self.id,
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
                self.db
                    .update_ledger_item(
                        self.uid,
                        self.id,
                        LedgerRecord {
                            id: initial.id,
                            info: withdrawal.clone(),
                        },
                    )
                    .unwrap()
            } else {
                self.db
                    .add_ledger_entry(self.uid, self.id, withdrawal.clone())
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
                    let mapped_labels = self
                        .db
                        .check_and_get_label_mapping_matching_ledger_id(self.uid, self.id, id)
                        .unwrap();
                    if !mapped_labels.is_empty() {
                        for label in mapped_labels {
                            self.db
                                .remove_label_mapping(self.uid, self.id, label.id)
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
                            uid: self.uid,
                            db: self.db.clone(),
                        })
                        .prompt()
                        .unwrap()
                        .to_ascii_uppercase();
                    let label_id = self.db.check_and_add_label(self.uid, label).unwrap();
                    self.db
                        .add_label_mapping(self.uid, self.id, label_id, entry.id)
                        .unwrap();

                    let continue_prompt = Confirm::new("Add more labels (y/n)?").prompt().unwrap();
                    if !continue_prompt {
                        break;
                    }
                }
            }

            if link {
                acct.link(self.id, entry.clone());
            }

            return entry;
        }
    }

    pub fn fee(&self, initial_opt: Option<LedgerRecord>, overwrite: bool) -> LedgerRecord { 
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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    cats: None,
                })
                .with_default(
                    self.db
                        .get_category_name(self.uid, self.id, initial.info.category_id)
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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    cats: None,
                })
                .with_validator(category_validator)
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string()
        };

        cid = self
            .db
            .check_and_add_category(self.uid, self.id, selected_category);

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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    ptype: ParticipantType::Payer,
                    with_accounts: false,
                    stock_tickers_only: false,
                    manually_recorded_only: false,
                })
                .with_default(
                    self.db
                        .get_participant(self.uid, self.id, initial.info.participant)
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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
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
        pid = self.db.check_and_add_participant(
            self.uid,
            self.id,
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
            self.db
                .update_ledger_item(
                    self.uid,
                    self.id,
                    LedgerRecord {
                        id: initial.id,
                        info: deposit.clone(),
                    },
                )
                .unwrap()
        } else {
            self.db
                .add_ledger_entry(self.uid, self.id, deposit.clone())
                .unwrap()
        };

        if overwrite {
            let maintain_labels =
                Confirm::new("Would you like to maintain all prior labels (y/n)?")
                    .prompt()
                    .unwrap();
            if !maintain_labels {
                let mapped_labels = self
                    .db
                    .check_and_get_label_mapping_matching_ledger_id(self.uid, self.id, id)
                    .unwrap();
                if !mapped_labels.is_empty() {
                    for label in mapped_labels {
                        self.db
                            .remove_label_mapping(self.uid, self.id, label.id)
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
                        uid: self.uid,
                        db: self.db.clone(),
                    })
                    .prompt()
                    .unwrap()
                    .to_ascii_uppercase();
                let label_id = self.db.check_and_add_label(self.uid, label).unwrap();
                self.db
                    .add_label_mapping(self.uid, self.id, label_id, id)
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

    pub fn deposit(&self, initial_opt: Option<LedgerRecord>, overwrite: bool) -> LedgerRecord {
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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    cats: None,
                })
                .with_default(
                    self.db
                        .get_category_name(self.uid, self.id, initial.info.category_id)
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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    cats: None,
                })
                .with_validator(category_validator)
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string()
        };

        cid = self
            .db
            .check_and_add_category(self.uid, self.id, selected_category);

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
                        uid: self.uid,
                        aid: self.id,
                        db: self.db.clone(),
                        ptype: ParticipantType::Payer,
                        with_accounts: false,
                        stock_tickers_only: false,
                        manually_recorded_only: false,
                    })
                    .with_default(
                        self.db
                            .get_participant(self.uid, self.id, initial.info.participant)
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
                        uid: self.uid,
                        aid: self.id,
                        db: self.db.clone(),
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
            pid = self.db.check_and_add_participant(
                self.uid,
                self.id,
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
                self.db
                    .update_ledger_item(
                        self.uid,
                        self.id,
                        LedgerRecord {
                            id: initial.id,
                            info: deposit.clone(),
                        },
                    )
                    .unwrap()
            } else {
                self.db
                    .add_ledger_entry(self.uid, self.id, deposit.clone())
                    .unwrap()
            };

            if overwrite {
                let maintain_labels =
                    Confirm::new("Would you like to maintain all prior labels (y/n)?")
                        .prompt()
                        .unwrap();
                if !maintain_labels {
                    let mapped_labels = self
                        .db
                        .check_and_get_label_mapping_matching_ledger_id(self.uid, self.id, id)
                        .unwrap();
                    if !mapped_labels.is_empty() {
                        for label in mapped_labels {
                            self.db
                                .remove_label_mapping(self.uid, self.id, label.id)
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
                            uid: self.uid,
                            db: self.db.clone(),
                        })
                        .prompt()
                        .unwrap()
                        .to_ascii_uppercase();
                    let label_id = self.db.check_and_add_label(self.uid, label).unwrap();
                    self.db
                        .add_label_mapping(self.uid, self.id, label_id, id)
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
                self.db
                    .get_participant(self.uid, self.id, initial.info.participant)
            } else {
                None
            };

            let user_input = self.link_transaction(initial_account_opt);
            if user_input.is_none() {
                return initial;
            }
            (acct, selected_payer) = user_input.unwrap();
            pid = self.db.check_and_add_participant(
                self.uid,
                self.id,
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
                self.db
                    .update_ledger_item(
                        self.uid,
                        self.id,
                        LedgerRecord {
                            id: initial.id,
                            info: deposit.clone(),
                        },
                    )
                    .unwrap()
            } else {
                self.db
                    .add_ledger_entry(self.uid, self.id, deposit.clone())
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
                    let mapped_labels = self
                        .db
                        .check_and_get_label_mapping_matching_ledger_id(self.uid, self.id, id)
                        .unwrap();
                    if !mapped_labels.is_empty() {
                        for label in mapped_labels {
                            self.db
                                .remove_label_mapping(self.uid, self.id, label.id)
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
                            uid: self.uid,
                            db: self.db.clone(),
                        })
                        .prompt()
                        .unwrap()
                        .to_ascii_uppercase();
                    let label_id = self.db.check_and_add_label(self.uid, label).unwrap();
                    self.db
                        .add_label_mapping(self.uid, self.id, label_id, entry.id)
                        .unwrap();

                    let continue_prompt = Confirm::new("Add more labels (y/n)?").prompt().unwrap();
                    if !continue_prompt {
                        break;
                    }
                }
            }

            if link {
                acct.link(self.id, entry.clone());
            }

            return entry;
        }
    }

    pub fn accrual(&self, initial_opt: Option<LedgerRecord>, overwrite: bool) -> LedgerRecord { 
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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    cats: None,
                })
                .with_default(
                    self.db
                        .get_category_name(self.uid, self.id, initial.info.category_id)
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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    cats: None,
                })
                .with_validator(category_validator)
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string()
        };

        cid = self
            .db
            .check_and_add_category(self.uid, self.id, selected_category);

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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
                    ptype: ParticipantType::Payer,
                    with_accounts: false,
                    stock_tickers_only: false,
                    manually_recorded_only: false,
                })
                .with_default(
                    self.db
                        .get_participant(self.uid, self.id, initial.info.participant)
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
                    uid: self.uid,
                    aid: self.id,
                    db: self.db.clone(),
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
        pid = self.db.check_and_add_participant(
            self.uid,
            self.id,
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
            self.db
                .update_ledger_item(
                    self.uid,
                    self.id,
                    LedgerRecord {
                        id: initial.id,
                        info: deposit.clone(),
                    },
                )
                .unwrap()
        } else {
            self.db
                .add_ledger_entry(self.uid, self.id, deposit.clone())
                .unwrap()
        };

        if overwrite {
            let maintain_labels =
                Confirm::new("Would you like to maintain all prior labels (y/n)?")
                    .prompt()
                    .unwrap();
            if !maintain_labels {
                let mapped_labels = self
                    .db
                    .check_and_get_label_mapping_matching_ledger_id(self.uid, self.id, id)
                    .unwrap();
                if !mapped_labels.is_empty() {
                    for label in mapped_labels {
                        self.db
                            .remove_label_mapping(self.uid, self.id, label.id)
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
                        uid: self.uid,
                        db: self.db.clone(),
                    })
                    .prompt()
                    .unwrap()
                    .to_ascii_uppercase();
                let label_id = self.db.check_and_add_label(self.uid, label).unwrap();
                self.db
                    .add_label_mapping(self.uid, self.id, label_id, id)
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

impl FixedAccount {

    pub fn link_transaction(
        &self,
        initial_opt: Option<String>,
    ) -> Option<(Box<dyn Account>, String)> {
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
                    with_accounts: true,
                    stock_tickers_only: false,
                    manually_recorded_only: false,
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
                    with_accounts: true,
                    stock_tickers_only: false,
                    manually_recorded_only: false,
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
            let user_input = prompt_and_create_new_account(self.uid, &self.db);
            if user_input.is_none() {
                return None;
            }
            (acct, record) = user_input.unwrap();
            selected_account = record.info.name;
        } else {
            let acctx = account_map
                .get(&selected_account)
                .expect("Account not found!");
            acct = decode_and_init_account_type(self.uid, &self.db, acctx);
        }

        return Some((acct, selected_account.clone()));
    }

    pub fn get_external_transactions_between_timestamps(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Option<Vec<LedgerRecord>> {
        let mut transactions: Vec<LedgerRecord> = Vec::new();
        if self.ledger.is_empty() {
            return None;
        }
        transactions = self
            .ledger
            .iter()
            .filter(|rcrd| {
                (rcrd.info.transfer_type == TransferType::WithdrawalToExternalAccount
                    || rcrd.info.transfer_type == TransferType::DepositFromExternalAccount)
                    && (NaiveDate::parse_from_str(rcrd.info.date.as_str(), "%Y-%m-%d")
                        .expect("Unable to parse date")
                        >= start_date)
                    && (NaiveDate::parse_from_str(rcrd.info.date.as_str(), "%Y-%m-%d")
                        .expect("Unable to parse date")
                        <= end_date)
            })
            .into_iter()
            .map(|x| x.clone())
            .collect();

        Some(transactions)
    }

    pub fn get_ledger_entries_between_timestamps(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<LedgerRecord> {
        let mut transactions: Vec<LedgerRecord> = Vec::new();
        transactions = self
            .ledger
            .iter()
            .filter(|rcrd| {
                (NaiveDate::parse_from_str(rcrd.info.date.as_str(), "%Y-%m-%d")
                    .expect("Unable to parse date")
                    >= start_date)
                    && (NaiveDate::parse_from_str(rcrd.info.date.as_str(), "%Y-%m-%d")
                        .expect("Unable to parse date")
                        <= end_date)
            })
            .into_iter()
            .map(|x| x.clone())
            .collect();

        transactions
    }

    // returns uid of selected ledger entry
    pub fn select_ledger_entry(&self) -> Option<LedgerRecord> {
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
}

impl BaseActions for FixedAccount { 
    fn get_current_value(&self) -> Option<f32> {
        let value= self.db.get_current_value(self.uid, self.id).unwrap();
        Some(value)
    }

    fn get_account_value_on_day(&self, day: &NaiveDate) -> Option<f32> {
        let value_opt = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.uid, self.id, *day)
            .unwrap();
        return value_opt;
    }

    fn modify(&mut self, selected_record: LedgerRecord) -> Option<LedgerRecord> {

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
                        account_transaction_opt = self
                            .db
                            .check_and_get_account_transaction_record_matching_to_ledger_id(
                                self.uid,
                                self.id,
                                selected_record.id,
                            )
                            .unwrap();
                        if account_transaction_opt.is_some() {
                            let account_transaction = account_transaction_opt.unwrap();
                            self.db
                                .remove_account_transaction(self.uid, account_transaction.id)
                                .unwrap();
                            self.db
                                .remove_ledger_item(
                                    self.uid,
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
                        account_transaction_opt = self
                            .db
                            .check_and_get_account_transaction_record_matching_from_ledger_id(
                                self.uid,
                                self.id,
                                selected_record.id,
                            )
                            .unwrap();
                        if account_transaction_opt.is_some() {
                            let account_transaction = account_transaction_opt.unwrap();
                            self.db
                                .remove_ledger_item(
                                    self.uid,
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
                        account_transaction_opt = self
                            .db
                            .check_and_get_account_transaction_record_matching_to_ledger_id(
                                self.uid,
                                self.id,
                                selected_record.id,
                            )
                            .unwrap();
                        if account_transaction_opt.is_some() {
                            let account_transaction = account_transaction_opt.unwrap();
                            self.db
                                .remove_account_transaction(self.uid, account_transaction.id)
                                .unwrap();
                            self.db
                                .remove_ledger_item(
                                    self.uid,
                                    account_transaction.info.from_account,
                                    account_transaction.info.from_ledger,
                                )
                                .unwrap();
                        }
                    }
                    TransferType::WithdrawalToExternalAccount => {
                        account_transaction_opt = self
                            .db
                            .check_and_get_account_transaction_record_matching_from_ledger_id(
                                self.uid,
                                self.id,
                                selected_record.id,
                            )
                            .unwrap();
                        if account_transaction_opt.is_some() {
                            let account_transaction = account_transaction_opt.unwrap();
                            self.db
                                .remove_account_transaction(self.uid, account_transaction.id)
                                .unwrap();
                            self.db
                                .remove_ledger_item(
                                    self.uid,
                                    account_transaction.info.to_account,
                                    account_transaction.info.to_ledger,
                                )
                                .unwrap();
                        }
                    }
                    _ => {},
                }
                self.db
                    .remove_ledger_item(self.uid, self.id, selected_record.id.clone())
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

impl BaseGrowth for FixedAccount {
    fn money_weighted_return(&self, start_date : NaiveDate, end_date : NaiveDate) -> f32 {
        #[derive(Debug)]
        struct CashFlow {
            amount: f32,
            t: f32,
        };

        fn irr(flows: &[CashFlow]) -> Option<f32> {
            let mut low = -0.29999;
            let mut high = 1.; // allow very high return
            let tolerance = 1e-2;

            fn npv(rate: f32, flows: &[CashFlow]) -> f32 {
                flows
                    .iter()
                    .map(|x| x.amount / (1.0 + rate).powf(x.t))
                    .sum()
            }

            if npv(low, flows) * npv(high, flows) > 0.0 {
                return None; // no guaranteed root
            }

            while (high - low) > tolerance {
                let mid = (low + high) / 2.0;
                let value = npv(mid, flows);

                if value > 0.0 {
                    low = mid;
                } else {
                    high = mid;
                }
            }

            Some((low + high) / 2.0)
        }

        let mut cfs: Vec<CashFlow> = Vec::new();

        let day_before = start_date.checked_sub_days(Days::new(1)).unwrap();
        let initial_value = self.get_account_value_on_day(&day_before).unwrap();
        cfs.push(CashFlow {
            amount: -initial_value,
            t: 0.0,
        });

        let txns = self
            .db
            .get_ledger_entries_within_timestamps(self.uid, self.id, start_date, end_date)
            .unwrap();

        for txn in txns {
            let txn_date = NaiveDate::parse_from_str(&txn.info.date, "%Y-%m-%d").unwrap();
            let amount = match txn.info.transfer_type {
                TransferType::DepositFromExternalAccount => -txn.info.amount,
                TransferType::WithdrawalToExternalAccount => txn.info.amount,
                _ => {
                    continue;
                }
            };

            let t = (txn_date - start_date).num_days() as f32 / 365.25;
            let cf = CashFlow {
                amount: amount,
                t: t,
            };
            cfs.push(cf);
        }

        let final_value_opt = self.get_account_value_on_day(&end_date);
        if final_value_opt.is_none() {
            return f32::NAN;
        }
        let final_value = final_value_opt.unwrap();
        let final_t = (end_date - start_date).num_days() as f32 / 365.25;
        cfs.push(CashFlow {
            amount: final_value,
            t: final_t,
        });

        let irr_opt = irr(&cfs);
        if irr_opt.is_none() {
            f32::NAN
        } else {
            irr_opt.unwrap() * 100.
        }
    }
    fn time_weighted_return(&self, start_date : NaiveDate, end_date : NaiveDate) -> f32 {
        return f32::NAN;
    }
}


