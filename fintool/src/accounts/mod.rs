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
pub mod bank_account;
pub mod base;
pub mod certificate_of_deposit;
pub mod credit_card_account;
pub mod growth;
pub mod health_savings_account;
pub mod investment_account_manager;
#[cfg(feature = "ratatui_support")]
pub mod render;
pub mod retirement_401k_plan;
pub mod roth_ira;
pub mod wallet;

use crate::accounts::base::{DisplayablePositionStatistics, HasContext, LedgerOps, Valuable};
#[cfg(feature = "ratatui_support")]
use crate::app::app::{App, DisplayValue};
#[cfg(feature = "ratatui_support")]
use crate::app::screen::{
    ledger_table_constraint_len_calculator, positions_table_constraint_len_calculator,
};
use crate::database::DbConn;
use crate::types::accounts::AccountRecord;
use crate::types::accounts::AccountTransaction;
use crate::types::accounts::AccountType;
use crate::types::ledger::{DisplayableLedgerRecord, LedgerInfo, LedgerRecord};
use crate::types::participants::ParticipantType;
use shared_lib::TransferType;
use strum::{Display, EnumIter, EnumString, FromRepr};

use chrono::NaiveDate;
#[cfg(feature = "ratatui_support")]
use ratatui::symbols::block;
#[cfg(feature = "ratatui_support")]
use ratatui::{
    buffer::Buffer,
    layout::{self, Constraint, Direction, Layout, Rect},
    style::{palette, palette::tailwind, Color, Modifier, Style, Stylize},
    symbols::{self, Marker},
    text::{Line, Span, Text as ratatuiText},
    widgets::{
        Axis, Bar, BarChart, BarGroup, Block, Borders, Cell, Chart, Clear, Dataset, GraphType,
        HighlightSpacing, List, ListItem, Padding, Paragraph, Row, Table, Tabs, Widget, Wrap,
    },
    Frame,
};
use std::any::Any;

use crate::accounts::bank_account::BankAccount;
use crate::accounts::base::liquid_account::LiquidAccount;
#[cfg(feature = "ratatui_support")]
use crate::accounts::investment_account_manager::InvestmentAccountManager;
#[cfg(feature = "ratatui_support")]
use crate::accounts::retirement_401k_plan::Retirement401kPlan;
#[cfg(feature = "ratatui_support")]
use crate::accounts::roth_ira::RothIraAccount;
use crate::accounts::wallet::Wallet;

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

pub const KEY_TOTAL_VALUE: &str = "Current Value";
pub const KEY_SIMPLE_RATE_OF_RETURN: &str = "SRR";
pub const KEY_COMPOUNDED_ANNUAL_RATE_OF_RETURN: &str = "CAGR";
pub const KEY_MONEY_WEIGHTED_RATE_OF_RETURN: &str = "MWRR";
pub const KEY_TIME_WEIGHTED_RATE_OF_RETURN: &str = "TWRR";
pub const KEY_BARCHART_BUDGET: &str = "Budget";
pub const KEY_BARCHART_EXPENDITURES: &str = "Expenditures";
pub const KEY_REMAINING_CREDIT: &str = "Remaining Credit";
pub const KEY_DAYS_UNTIL_DUE: &str = "Days Until Due";
pub const KEY_STATEMENT_DUE_DATE: &str = "Statement Due Date";
pub const KEY_CREDIT_LINE: &str = "Credit Line";
pub const KEY_REMAINING_CONTRIBUTION: &str = "Remaining Contribution";
pub const KEY_CONTRIBUTION_LIMIT: &str = "Contribution Limit";
pub const KEY_MATURITY_DATE: &str = "Maturity Date";
pub const KEY_DAYS_TO_MATURITY: &str = "Days to Maturity";

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

#[derive(Clone, Display, Debug, FromRepr, EnumIter, EnumString)]
pub enum AnalysisPeriod {
    #[strum(to_string = "1 Day")]
    OneDay,
    #[strum(to_string = "1 Week")]
    OneWeek,
    #[strum(to_string = "1 Month")]
    OneMonth,
    #[strum(to_string = "3 Months")]
    ThreeMonths,
    #[strum(to_string = "6 Months")]
    SixMonths,
    #[strum(to_string = "1 Year")]
    OneYear,
    #[strum(to_string = "2 Years")]
    TwoYears,
    #[strum(to_string = "5 Years")]
    FiveYears,
    #[strum(to_string = "10 Years")]
    TenYears,
    #[strum(to_string = "YTD")]
    YTD,
    #[strum(to_string = "All Time")]
    AllTime,
    #[strum(to_string = "Custom")]
    Custom,
}

impl AnalysisPeriod {
    pub fn to_menu_selection(value: Self) -> String {
        format!("{value}")
    }
}

pub trait AccountCreation {
    fn create(uid: u32, name: String, _db: &DbConn) -> AccountRecord;
}

pub trait AccountOperations: HasContext {
    fn import(&mut self);
    fn record(&mut self);
    fn modify(&mut self);
    fn export(&self);
    fn report(&self);
    fn link(&self, transacting_account: u32, entry: LedgerRecord) -> Option<u32> {
        let ctx = self.ctx();
        let from_account;
        let to_account;

        let cid;
        let pid;
        let transacting_account_name: String;
        let (new_ttype, description) = match entry.info.transfer_type {
            TransferType::DepositFromExternalAccount => {
                // if the transacting account received a deposit, then self must be the "from" account
                from_account = ctx.aid;
                to_account = transacting_account;
                cid = ctx.db.check_and_add_category(
                    ctx.uid,
                    ctx.aid,
                    "Withdrawal".to_ascii_uppercase(),
                );
                transacting_account_name = ctx
                    .db
                    .get_account_name(ctx.uid, transacting_account)
                    .unwrap();
                pid = ctx.db.check_and_add_participant(
                    ctx.uid,
                    ctx.aid,
                    transacting_account_name.clone(),
                    ParticipantType::Payee,
                    true,
                );
                (
                    TransferType::WithdrawalToExternalAccount,
                    format!(
                        "[Link]: Withdrawal of ${} to account {} on {}.",
                        entry.info.amount, transacting_account_name, entry.info.date
                    ),
                )
            }
            TransferType::WithdrawalToExternalAccount => {
                // if the transacting account had an amount withdrawn, then self must be the "to" account
                from_account = transacting_account;
                to_account = ctx.aid;
                cid =
                    ctx.db
                        .check_and_add_category(ctx.uid, ctx.aid, "Deposit".to_ascii_uppercase());
                transacting_account_name = ctx
                    .db
                    .get_account_name(ctx.uid, transacting_account)
                    .unwrap();
                pid = ctx.db.check_and_add_participant(
                    ctx.uid,
                    ctx.aid,
                    transacting_account_name.clone(),
                    ParticipantType::Payer,
                    true,
                );
                (
                    TransferType::DepositFromExternalAccount,
                    format!(
                        "[Link]: Deposit of ${} from account {} on {}.",
                        entry.info.amount, transacting_account_name, entry.info.date
                    ),
                )
            }
            _ => {
                return None;
            }
        };

        let linked_entry = LedgerInfo {
            date: entry.info.date,
            amount: entry.info.amount,
            transfer_type: new_ttype.clone(),
            participant: pid,
            category_id: cid,
            description: description,
        };

        let (from_ledger_id, to_ledger_id) = match new_ttype {
            TransferType::WithdrawalToExternalAccount => (
                ctx.db
                    .add_ledger_entry(ctx.uid, ctx.aid, linked_entry)
                    .unwrap(),
                entry.id,
            ),
            TransferType::DepositFromExternalAccount => (
                entry.id,
                ctx.db
                    .add_ledger_entry(ctx.uid, ctx.aid, linked_entry)
                    .unwrap(),
            ),
            _ => {
                panic!("Unrecognized input!")
            }
        };

        let transaction_record = AccountTransaction {
            from_account: from_account,
            to_account: to_account,
            from_ledger: from_ledger_id,
            to_ledger: to_ledger_id,
        };

        return Some(
            ctx.db
                .add_account_transaction(ctx.uid, transaction_record)
                .unwrap(),
        );
    }
}

pub trait AccountData: HasContext + Valuable + LedgerOps {
    fn get_id(&self) -> u32 {
        self.ctx().aid
    }
    fn get_name(&self) -> String {
        self.ctx()
            .db
            .get_account_name(self.ctx().uid, self.ctx().aid)
            .unwrap()
    }
    fn get_value(&self) -> f32 {
        return self.account_value().unwrap_or(f32::NAN);
    }
    fn get_value_on_day(&self, day: NaiveDate) -> f32 {
        return self.get_account_value_on_day(&day).unwrap_or(f32::NAN);
    }
    fn get_open_date(&self) -> NaiveDate {
        let ctx = self.ctx();
        return ctx.open_date;
    }
}

#[cfg(feature = "ratatui_support")]
pub trait AccountUI: AccountData {
    fn populate_page_cache_f32(&self, app: &mut App);

    fn render(&self, frame: &mut Frame, area: Rect, app: &mut App);
}

#[cfg(not(feature = "ratatui_support"))]
pub trait Account: AccountData + AccountOperations + Any + HasContext {
    fn kind(&self) -> AccountType;
    fn has_budget(&self) -> bool {
        let ctx = self.ctx();
        let acct = ctx.db.get_account(ctx.uid, ctx.aid).unwrap();
        acct.info.has_budget
    }
    fn set_budget(&self) {
        let ctx = self.ctx();
        let mut acct = ctx.db.get_account(ctx.uid, ctx.aid).unwrap();
        acct.info.has_budget = true;
        let _ = ctx.db.update_account(ctx.uid, ctx.aid, &acct.info).unwrap();
    }
}

#[cfg(feature = "ratatui_support")]
pub trait Account: AccountData + AccountOperations + AccountUI + Any + HasContext {
    fn kind(&self) -> AccountType;
    fn as_any(&self) -> &dyn std::any::Any;
    fn has_budget(&self) -> bool {
        let ctx = self.ctx();
        let acct = ctx.db.get_account(ctx.uid, ctx.aid).unwrap();
        acct.info.has_budget
    }
    fn set_budget(&self) {
        let ctx = self.ctx();
        let mut acct = ctx.db.get_account(ctx.uid, ctx.aid).unwrap();
        acct.info.has_budget = true;
        let _ = ctx.db.update_account(ctx.uid, ctx.aid, &acct.info).unwrap();
    }
    fn as_liquid_account(&self) -> Option<&dyn LiquidAccount> {
        return None;
    }
    fn renders_tables(&self) -> Vec<String> {
        return vec!["Transactions".to_string()];
    }
}

#[cfg(feature = "ratatui_support")]
pub fn render_table_tabs(
    frame: &mut Frame,
    area: Rect,
    tab_names: Vec<String>,
    selected_tab: usize,
    highlight_color: Color,
) {
    let atype_tabs = Tabs::new(tab_names.into_iter())
        .highlight_style(highlight_color)
        .select(selected_tab)
        .block(
            Block::bordered()
                .title(" Tables ")
                .style(Style::new().bg(tailwind::SLATE.c900)),
        )
        .padding("", "")
        .divider(" | ");
    frame.render_widget(atype_tabs, area);
}
