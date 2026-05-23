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
#[cfg(feature = "ratatui_support")]
use crate::app::app::{App, DisplayValue};
#[cfg(feature = "ratatui_support")]
use crate::app::screen::{ledger_table_constraint_len_calculator, positions_table_constraint_len_calculator};
use crate::database::DbConn;
use crate::types::participants::{ParticipantAutoCompleter, ParticipantType};
use crate::types::accounts::AccountRecord;
use crate::types::accounts::AccountType;
use crate::types::ledger::{DisplayableLedgerRecord, LedgerInfo, LedgerRecord};
use crate::tui::{decode_and_init_account_type, prompt_and_create_new_account};
#[cfg(feature = "ratatui_support")]
use crate::ui::centered_rect;
use chrono::{Datelike, Local, Month, NaiveDate, NaiveDateTime, naive};
use inquire::*;
use rusqlite::config::DbConfig;
use shared_lib::{LedgerEntry, TransferType};
use core::f32;
use std::any::Any;
use std::collections::HashMap;
use yahoo_finance_api::Quote;
use crate::accounts::Account;

pub mod budget;
pub mod charge_account;
pub mod fixed_account;
pub mod interest_bearing_fixed_account;
pub mod liquid_account;
pub mod variable_account;

pub struct AccountContext {
    pub aid: u32,
    pub uid: u32,
    pub db: DbConn,
    pub open_date : NaiveDate,
}

pub trait HasContext { 
    fn ctx(&self) -> &AccountContext;
    fn ctx_mut(&mut self) -> &mut AccountContext;
}

pub trait LedgerOps : HasContext {
    fn modify(&mut self, selected_record: LedgerRecord) -> Option<LedgerRecord>;

    fn link_transaction(
        &self,
        initial_opt: Option<String>,
    ) -> Option<(Box<dyn Account>, String)> {

        let ctx = Self::ctx(&self);

        let default_to_use;
        let mut initial_account = String::new();
        if initial_opt.is_some() {
            default_to_use = true;
            initial_account = initial_opt.unwrap();
        } else {
            default_to_use = false;
        }

        let accounts = ctx.db.get_user_accounts(ctx.uid).unwrap();
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
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
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
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
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
            let user_input = prompt_and_create_new_account(ctx.uid, &ctx.db);
            if user_input.is_none() {
                return None;
            }
            (acct, record) = user_input.unwrap();
            selected_account = record.info.name;
        } else {
            let acctx = account_map
                .get(&selected_account)
                .expect("Account not found!");
            acct = decode_and_init_account_type(ctx.uid, &ctx.db, acctx);
        }

        return Some((acct, selected_account.clone()));
    }

    fn get_ledger(&self) -> Vec<LedgerRecord> {
        let ctx = self.ctx();
        return ctx.db.get_ledger(ctx.uid, ctx.aid).unwrap();
    }
    fn get_ledger_within_dates(&self, start: NaiveDate, end: NaiveDate) -> Vec<LedgerRecord> {
        self.get_ledger_entries_between_timestamps(start, end)
    }
    fn get_displayable_ledger(&self) -> Vec<crate::types::ledger::DisplayableLedgerRecord> {
        let ctx = self.ctx();
        return ctx.db.get_displayable_ledger(ctx.uid, ctx.aid).unwrap();
    }
    
    fn get_external_transactions_between_timestamps(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Option<Vec<LedgerRecord>> {
        let ctx: &AccountContext = Self::ctx(&self);
        let ledger = ctx.db.get_external_transactions_between_timestamps(ctx.uid, ctx.aid, start_date, end_date).unwrap();
        ledger
    }

    fn get_ledger_entries_between_timestamps(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<LedgerRecord> {
        let ctx: &AccountContext = Self::ctx(&self);    
        let ledger = ctx.db.get_ledger_entries_within_timestamps(ctx.uid, ctx.aid,start_date, end_date).unwrap();
        ledger
    }

    // returns uid of selected ledger entry
    fn select_ledger_entry(&self) -> Option<LedgerRecord> {
        let ctx: &AccountContext = Self::ctx(&self);
        let records = ctx.db.get_ledger(ctx.uid, ctx.aid).unwrap();
        let mut entries: HashMap<String, u32> = HashMap::new();
        let mut strings: Vec<String> = Vec::new();
        let mut mapped_records: HashMap<u32, LedgerInfo> = HashMap::new();
        for rcrd in records {
            let v: String = format!(
                "{} | {} | {} | {} | ",
                rcrd.info.date,
                ctx.db
                    .get_category_name(ctx.uid, ctx.aid, rcrd.info.category_id)
                    .unwrap(),
                ctx.db
                    .get_participant(ctx.uid, ctx.aid, rcrd.info.participant)
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

pub struct VariableAccountContext { 
    pub buffer: Option<Vec<StockData>>,
}

pub trait HasVariableAccountContext { 
    fn variable_ctx(&self) -> &VariableAccountContext;
    fn variable_ctx_mut(&mut self) -> &mut VariableAccountContext;
}

pub trait Valuable : HasContext {
    fn account_value(&self) -> Option<f32>;
    fn get_account_value_on_day(&self, day: &NaiveDate) -> Option<f32>;
}

pub trait ValueLimited: HasContext {
    fn account_limit(&self) -> f32;
    fn remaining(&self) -> f32;
    fn value_reset_date(&self) -> NaiveDate {
        let today = Local::now().date_naive();
        let reset = NaiveDate::from_ymd_opt(today.year(), Month::December.number_from_month(), 31).expect("Unable to formulate date!");
        reset
    }
    fn days_until_limit_reset(&self) -> u32 {
        let local = Local::now().date_naive();
        return (local-self.value_reset_date()).num_days() as u32;
    }
}

pub trait AccountFileIO : HasContext + LedgerOps {
    fn import(&self);
    fn export(&self);
}

#[derive(Debug, Clone)]
pub struct StockData {
    ticker: String,
    quotes: Vec<Quote>,
    history: Vec<SharesOwned>,
}

#[derive(Debug, Clone)]
pub struct SharesOwned {
    date: NaiveDate,
    shares: f32,
}

#[derive(Debug, Clone)]
pub struct DisplayablePositionStatistics { 
    pub ticker : String,
    pub quantity : String, 
    pub value : String, 
    pub price : String,
    pub total_cost_basis : String, 
    pub unit_cost : String, 
    pub unrealized_gl : String,
    pub unrealized_gl_per : String,
}

impl DisplayablePositionStatistics {
    pub fn get_ticker_str() -> String { 
        "Ticker".to_string()
    }
    pub fn get_quantity_str() -> String { 
        "Quantity (UoM)".to_string()
    }
    pub fn get_value_str() -> String { 
        "Value ($)".to_string()
    }    
    pub fn get_price_str() -> String { 
        "Price ($)".to_string()
    }    
    pub fn get_total_cost_basis_str() -> String { 
        "Total Cost Basis ($)".to_string()
    }
    pub fn get_unit_cost_str() -> String { 
        "Unit Cost ($)".to_string()
    }
    pub fn get_unrealized_gl_str() -> String { 
        "Unrealized G/L ($)".to_string()
    }
    pub fn get_unrealized_gl_per_str() -> String { 
        "Unrealized G/L (%)".to_string()
    }   
}
