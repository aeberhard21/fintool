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
use chrono::{Days, Local, Months, NaiveDate};
use csv::ReaderBuilder;
use inquire::Confirm;
use inquire::CustomType;
use inquire::Select;
use inquire::Text;
#[cfg(feature = "ratatui_support")]
use ratatui::{
    buffer::Buffer,
    layout::{self, Constraint, Direction, Layout, Rect},
    style::{palette, palette::tailwind, Color, Modifier, Style, Stylize},
    symbols::{self, Marker},
    text::{Line, Span, Text as ratatuiText},
    widgets::{
        canvas::{Canvas, Circle, Line as CanvasLine},
        Axis, Bar, BarChart, BarGroup, Block, Borders, Cell, Chart, Clear, Dataset, GraphType,
        HighlightSpacing, List, ListItem, Padding, Paragraph, Row, Table, Tabs, Widget, Wrap,
    },
    Frame,
};
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
use shared_lib::{FlatLedgerEntry, LedgerEntry};
use std::collections::HashMap;
use std::env::current_exe;
use std::hash::Hash;
use std::iter::zip;
use std::path::Path;
use std::rc;

use crate::accounts::base::budget::Budget;
use crate::accounts::base::charge_account::ChargeAccountLedger;
use crate::accounts::base::charge_account::ChargeAccountValuable;
use crate::accounts::base::charge_account::ChargedAccountExpiry;
use crate::accounts::base::fixed_account::FixedAccountFileIO;
use crate::accounts::base::fixed_account::{FixedAccount, FixedGrowth, FixedValuable};
use crate::accounts::base::interest_bearing_fixed_account::InterestBearingLedger;
use crate::accounts::base::AccountFileIO;
use crate::accounts::base::ValueLimited;
use crate::accounts::base::{AccountContext, Valuable};
use crate::accounts::base::{HasContext, LedgerOps};
use crate::accounts::growth::GrowthCalculable;
#[cfg(feature = "ratatui_support")]
use crate::accounts::render::*;
use crate::accounts::FilePathHelper;
use crate::accounts::{
    KEY_BARCHART_BUDGET, KEY_BARCHART_EXPENDITURES, KEY_CREDIT_LINE, KEY_DAYS_UNTIL_DUE,
    KEY_REMAINING_CONTRIBUTION, KEY_REMAINING_CREDIT, KEY_STATEMENT_DUE_DATE,
};
#[cfg(feature = "ratatui_support")]
use crate::app::app::{App, BarChartData, DisplayValue};
#[cfg(feature = "ratatui_support")]
use crate::app::screen::ledger_table_constraint_len_calculator;
use crate::database::DbConn;
use crate::tui::query_user_for_analysis_period;
use crate::types::accounts::AccountInfo;
use crate::types::accounts::AccountRecord;
use crate::types::accounts::AccountTransaction;
use crate::types::accounts::AccountType;
use crate::types::credit_card;
use crate::types::credit_card::CreditCardInfo;
use crate::types::ledger::LedgerInfo;
use crate::types::ledger::LedgerRecord;
use crate::types::participants;
use crate::types::participants::ParticipantType;
use crate::{tui::get_analysis_period_dates, types::ledger::Expenditure};
use shared_lib::TransferType;

use super::base::charge_account::ChargeAccount;
use super::Account;
use super::AccountCreation;
use super::AccountData;
use super::AccountOperations;
#[cfg(feature = "ratatui_support")]
use super::AccountUI;
use super::KEY_TOTAL_VALUE;

#[cfg(feature = "ratatui_support")]
use crate::ui::{centered_rect, float_range};

pub struct CreditCardAccount {
    ctx: AccountContext,
}

impl HasContext for CreditCardAccount {
    fn ctx(&self) -> &AccountContext {
        &self.ctx
    }
    fn ctx_mut(&mut self) -> &mut AccountContext {
        &mut self.ctx
    }
}

impl ChargeAccountLedger for CreditCardAccount {}

impl LedgerOps for CreditCardAccount {
    fn modify(&mut self, selected_record: LedgerRecord) -> Option<LedgerRecord> {
        self.modify_charge_account(selected_record)
    }
}

impl ChargeAccount for CreditCardAccount {}

impl Valuable for CreditCardAccount {
    fn account_value(&self) -> Option<f32> {
        self.balance()
    }
    fn get_account_value_on_day(&self, day: &NaiveDate) -> Option<f32> {
        self.balance_on_day(day)
    }
}

impl ChargeAccountValuable for CreditCardAccount {}

impl Budget for CreditCardAccount {}

impl ValueLimited for CreditCardAccount {
    fn account_limit(&self) -> f32 {
        return self.get_credit_line();
    }
    fn remaining(&self) -> f32 {
        return self.get_remaining_in_credit_line();
    }
    fn value_reset_date(&self) -> NaiveDate {
        self.get_statement_due_date()
    }
}

impl ChargedAccountExpiry for CreditCardAccount {}

impl AccountFileIO for CreditCardAccount {
    fn import(&self) {
        self.import_fixed_account();
    }
    fn export(&self) {
        self.export_fixed_account();
    }
}

impl FixedAccountFileIO for CreditCardAccount {}

impl CreditCardAccount {
    pub fn new(uid: u32, id: u32, db: &DbConn) -> Self {
        let mut acct: CreditCardAccount = Self {
            ctx: AccountContext {
                aid: id,
                uid: uid,
                db: db.clone(),
                open_date: Local::now().date_naive(),
            },
        };

        let mut ledger = acct.get_ledger();
        if !ledger.is_empty() {
            ledger.sort_by(|l1, l2| (&l1.info.date).cmp(&l2.info.date));
            acct.ctx.open_date =
                NaiveDate::parse_from_str(&ledger[0].info.date, "%Y-%m-%d").unwrap();
        }

        acct
    }

    fn get_statement_due_date(&self) -> NaiveDate {
        use chrono::Datelike;

        let credit_card = self
            .ctx
            .db
            .get_credit_card(self.ctx.uid, self.ctx.aid)
            .unwrap();
        let due_date = credit_card.info.statement_due_date;
        let local = Local::now().date_naive();
        let day = local.day();
        let diff: i32 = due_date as i32 - day as i32;
        let mut statement_due_date = local;
        if diff >= 0 {
            return statement_due_date
                .checked_add_days(Days::new(diff as u64))
                .unwrap();
        } else {
            statement_due_date = statement_due_date
                .checked_add_months(Months::new(1))
                .unwrap();
            statement_due_date = statement_due_date.with_day(due_date).unwrap();
            return statement_due_date;
        }
    }
}

impl AccountCreation for CreditCardAccount {
    fn create(uid: u32, name: String, _db: &DbConn) -> AccountRecord {
        let has_bank = false;
        let has_stocks = false;
        let has_ledger = false;
        let has_budget = false;

        let account: AccountInfo = AccountInfo {
            atype: AccountType::CreditCard,
            name: name,
            has_stocks: has_stocks,
            has_bank: has_bank,
            has_ledger: has_ledger,
            has_budget: has_budget,
        };

        let aid = _db.add_account(uid, &account).unwrap();

        let credit_limit = CustomType::<f32>::new("Enter credit limit:")
            .with_placeholder("3000.00")
            .with_default(3000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();
        let statement_due_date =
            CustomType::<u32>::new("Enter day of month that statement is due:")
                .with_placeholder("1")
                .with_default(1)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap();
        let cc = CreditCardInfo {
            credit_line: credit_limit,
            statement_due_date: statement_due_date,
        };

        _db.add_credit_card(uid, aid, cc).unwrap();

        let add_budget = Confirm::new("Would you like to associate a budget to this account?")
            .with_default(false)
            .prompt()
            .unwrap();
        if add_budget {
            let x = Self::new(uid, aid, _db);
            x.create_budget();
            x.set_budget();
        }

        return AccountRecord {
            id: aid,
            info: account,
        };
    }
}

impl AccountOperations for CreditCardAccount {
    fn record(&mut self) {
        const RECORD_OPTIONS: [&'static str; 6] =
            ["Accrual", "Budget", "Charge", "Fee", "Payment", "None"];
        loop {
            let action = Select::new(
                "\nWhat transaction would you like to record?",
                RECORD_OPTIONS.to_vec(),
            )
            .prompt()
            .unwrap()
            .to_string();
            match action.as_str() {
                "Accrual" => {
                    self.accrual(None, false);
                }
                "Fee" => {
                    self.fee(None, false);
                }
                "Payment" => {
                    self.pay(None, false);
                }
                "Charge" => {
                    self.charge(None, false);
                }
                "Budget" => {
                    if !self.has_budget() {
                        let add_budget = Confirm::new("A budget for this account does not exist, would you like to create one (y/n)?")
                            .with_default(false)
                            .prompt()
                            .unwrap();
                        if !add_budget {
                            continue;
                        }
                        self.set_budget();
                    }
                    <CreditCardAccount as Budget>::record(&self);
                }
                "None" => {
                    return;
                }
                _ => {
                    panic!("Unrecognized input!");
                }
            }
            let record_again = Confirm::new("Would you like to record another transaction?")
                .prompt()
                .unwrap();
            if !record_again {
                return;
            }
        }
    }

    fn import(&mut self) {
        <Self as AccountFileIO>::import(&self);
    }

    fn modify(&mut self) {
        const MODIFY_OPTIONS: [&'static str; 6] = [
            "Ledger",
            "Credit Line",
            "Statement Due Date",
            "Categories",
            "People",
            "None",
        ];
        const MODIFY_OPTIONS_WITH_BUDGET: [&'static str; 7] = [
            "Ledger",
            "Credit Line",
            "Statement Due Date",
            "Budget",
            "Categories",
            "People",
            "None",
        ];
        loop {
            let options = MODIFY_OPTIONS_WITH_BUDGET.to_vec();
            let modify_choice = Select::new("\nWhat would you like to modify:", options)
                .prompt()
                .unwrap();
            match modify_choice {
                "Budget" => {
                    if self.has_budget() {
                        <CreditCardAccount as Budget>::modify(&self);
                    } else {
                        let add_budget = Confirm::new("A budget for this account does not exist, would you like to create one (y/n)?")
                            .with_default(false)
                            .prompt()
                            .unwrap();
                        if add_budget {
                            self.create_budget();
                            self.set_budget();
                        }
                    }
                }
                "Ledger" => loop {
                    let record_or_none = self.select_ledger_entry();
                    if record_or_none.is_none() {
                        break;
                    }
                    let selected_record = record_or_none.unwrap();
                    <CreditCardAccount as LedgerOps>::modify(self, selected_record);
                    let go_again = Confirm::new("Modify additional records? (y/n)")
                        .prompt()
                        .unwrap();
                    if !go_again {
                        break;
                    }
                },
                "Credit Line" => {
                    let credit_card = self
                        .ctx
                        .db
                        .get_credit_card(self.ctx.uid, self.ctx.aid)
                        .unwrap();
                    let updated_credit_line = CustomType::<f32>::new("Enter updated credit line:")
                        .with_default(credit_card.info.credit_line)
                        .with_placeholder("1000.00")
                        .with_error_message("Enter a valid credit line!")
                        .prompt()
                        .unwrap();
                    self.ctx
                        .db
                        .update_credit_line(self.ctx.uid, self.ctx.aid, updated_credit_line)
                        .unwrap();
                }
                "Statement Due Date" => {
                    let credit_card = self
                        .ctx
                        .db
                        .get_credit_card(self.ctx.uid, self.ctx.aid)
                        .unwrap();
                    let updated_statement_due_date =
                        CustomType::<u32>::new("Enter updated statement due date:")
                            .with_default(credit_card.info.statement_due_date)
                            .with_placeholder("1")
                            .with_error_message("Enter a statement due date!")
                            .prompt()
                            .unwrap();
                    self.ctx
                        .db
                        .update_statement_due_date(
                            self.ctx.uid,
                            self.ctx.aid,
                            updated_statement_due_date,
                        )
                        .unwrap();
                }
                "Categories" => {
                    loop {
                        let records = self
                            .ctx
                            .db
                            .get_categories(self.ctx.uid, self.ctx.aid)
                            .unwrap();
                        let mut choices: Vec<String> = records
                            .iter()
                            .map(|x| x.category.name.clone())
                            .collect::<Vec<String>>();
                        choices.push("None".to_string());
                        let chosen_category = Select::new("Select category to modify:", choices)
                            .prompt()
                            .unwrap();

                        if chosen_category == "None" {
                            break;
                        }

                        const MODIFY_ACTIONS: [&'static str; 3] = ["Update", "Remove", "None"];
                        let update_or_remove =
                            Select::new("What would you like to do:", MODIFY_ACTIONS.to_vec())
                                .prompt()
                                .unwrap();
                        match update_or_remove {
                            "Update" => {
                                let new_name = Text::new("Enter category name:")
                                    .prompt()
                                    .unwrap()
                                    .to_string();
                                self.ctx.db.update_category_name(
                                    self.ctx.uid,
                                    self.ctx.aid,
                                    chosen_category,
                                    new_name,
                                );
                            }
                            "Remove" => {
                                // check if category is referenced by any current ledger
                                let is_referenced = self
                                    .ctx
                                    .db
                                    .check_if_ledger_references_category(
                                        self.ctx.uid,
                                        self.ctx.aid,
                                        chosen_category.clone(),
                                    )
                                    .unwrap();
                                if is_referenced.is_some() {
                                    let matched_records = is_referenced.unwrap();
                                    println!("The following records were found:");
                                    for record in matched_records {
                                        let v = format!(
                                            "{} | {} | {} | {} ",
                                            record.info.date,
                                            chosen_category.clone(),
                                            self.ctx
                                                .db
                                                .get_participant(
                                                    self.ctx.uid,
                                                    self.ctx.aid,
                                                    record.info.participant
                                                )
                                                .unwrap(),
                                            record.info.amount
                                        );
                                        print!("\t{}", v);
                                        println!("")
                                    }
                                }

                                // confirm they want to remove
                                let rm_msg = format!("Are you sure you want to delete the category {} (this will also delete found records)?", chosen_category);
                                let delete = Confirm::new(&rm_msg).prompt().unwrap();
                                if delete {
                                    self.ctx.db.remove_category(
                                        self.ctx.uid,
                                        self.ctx.aid,
                                        chosen_category.clone(),
                                    );
                                }
                            }
                            "None" => {
                                break;
                            }
                            _ => {
                                panic!("Unrecognized input!");
                            }
                        }
                        let go_again = Confirm::new("Modify additional categories? (y/n)")
                            .prompt()
                            .unwrap();
                        if !go_again {
                            break;
                        }
                    }
                }
                "People" => {
                    const PTYPE_OPTIONS: [&'static str; 3] = ["Payer", "Payee", "Both"];
                    loop {
                        let selected_ptype =
                            Select::new("What type of person:", PTYPE_OPTIONS.to_vec())
                                .prompt()
                                .unwrap();
                        let ptype = match selected_ptype {
                            "Payer" => ParticipantType::Payer,
                            "Payee" => ParticipantType::Payee,
                            "Both" => ParticipantType::Both,
                            _ => {
                                panic!("Unrecognized input: {}", selected_ptype);
                            }
                        };
                        let participants = self
                            .ctx
                            .db
                            .get_participants(self.ctx.uid, self.ctx.aid, ptype)
                            .unwrap();
                        let mut people = participants
                            .iter()
                            .map(|x| x.participant.name.clone())
                            .collect::<Vec<String>>();
                        // i think this is needed when "both" is selected, because an entry will be provided for each participant
                        people.sort();
                        people.dedup();
                        people.push("None".to_string());

                        let chosen_person = Select::new("Select person to modify:", people)
                            .prompt()
                            .unwrap();

                        if chosen_person == "None".to_string() {
                            break;
                        }

                        const MODIFY_ACTIONS: [&'static str; 3] = ["Update", "Remove", "None"];
                        let update_or_remove =
                            Select::new("What would you like to do:", MODIFY_ACTIONS.to_vec())
                                .prompt()
                                .unwrap();

                        match update_or_remove {
                            "Update" => {
                                let new_name = Text::new("Enter person's name:")
                                    .prompt()
                                    .unwrap()
                                    .to_string();
                                self.ctx
                                    .db
                                    .update_participant_name(
                                        self.ctx.uid,
                                        self.ctx.aid,
                                        ptype,
                                        chosen_person.clone(),
                                        new_name,
                                    )
                                    .unwrap();
                            }
                            "Remove" => {
                                // check if participant is referenced by any current ledger
                                let is_referenced = self
                                    .ctx
                                    .db
                                    .check_if_ledger_references_participant(
                                        self.ctx.uid,
                                        self.ctx.aid,
                                        ptype,
                                        chosen_person.clone(),
                                    )
                                    .unwrap();
                                if is_referenced.is_some() {
                                    let matched_records = is_referenced.unwrap();
                                    println!("The following records were found:");
                                    for record in matched_records {
                                        let v = format!(
                                            "{} | {} | {} | {} ",
                                            record.info.date,
                                            self.ctx
                                                .db
                                                .get_category_name(
                                                    self.ctx.uid,
                                                    self.ctx.aid,
                                                    record.info.category_id
                                                )
                                                .unwrap(),
                                            chosen_person.clone(),
                                            record.info.amount
                                        );
                                        print!("\t{}", v);
                                        println!("")
                                    }
                                }
                                // confirm they want to remove
                                let rm_msg = format!("Are you sure you want to delete the participant {} (this will also delete found records)?", chosen_person);
                                let delete = Confirm::new(&rm_msg).prompt().unwrap();
                                if delete {
                                    match ptype {
                                        ParticipantType::Payee => {
                                            self.ctx
                                                .db
                                                .remove_participant(
                                                    self.ctx.uid,
                                                    self.ctx.aid,
                                                    ParticipantType::Payee,
                                                    chosen_person.clone(),
                                                )
                                                .unwrap();
                                        }
                                        ParticipantType::Payer => {
                                            self.ctx
                                                .db
                                                .remove_participant(
                                                    self.ctx.uid,
                                                    self.ctx.aid,
                                                    ParticipantType::Payer,
                                                    chosen_person.clone(),
                                                )
                                                .unwrap();
                                        }
                                        _ => {
                                            self.ctx
                                                .db
                                                .remove_participant(
                                                    self.ctx.uid,
                                                    self.ctx.aid,
                                                    ParticipantType::Payee,
                                                    chosen_person.clone(),
                                                )
                                                .unwrap();
                                            self.ctx
                                                .db
                                                .remove_participant(
                                                    self.ctx.uid,
                                                    self.ctx.aid,
                                                    ParticipantType::Payer,
                                                    chosen_person.clone(),
                                                )
                                                .unwrap();
                                        }
                                    }
                                }
                            }
                            "None" => {
                                break;
                            }
                            _ => {
                                panic!("Unrecognized input: {}", update_or_remove);
                            }
                        }
                        let go_again = Confirm::new("Modify additional people? (y/n)")
                            .prompt()
                            .unwrap();
                        if !go_again {
                            break;
                        }
                    }
                }
                "None" => {
                    return;
                }
                _ => {
                    panic!("Unrecognized input!")
                }
            }
            let go_again = Confirm::new("Modify additional elements? (y/n)")
                .prompt()
                .unwrap();
            if !go_again {
                break;
            }
        }
    }

    fn export(&self) {
        <Self as AccountFileIO>::export(&self);
    }

    fn report(&self) {
        const REPORT_OPTIONS: [&'static str; 5] = [
            "Current Balance",
            "Credit Line",
            "Remaining Credit",
            "Spend Analyzer",
            "None",
        ];
        let choice: String =
            Select::new("What would you like to report: ", REPORT_OPTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();
        match choice.as_str() {
            "Current Balance" => {
                let value = self.account_value();
                println!("\tCurrent Balance: {}", value.unwrap());
            }
            "Credit Line" => {
                println!("\tCredit Line: {}", self.account_limit());
            }
            "Remaining Credit" => {
                println!("\tRemaining credit: {}", self.remaining());
            }
            "Spend Analyzer" => {
                let (start, end, _) = query_user_for_analysis_period(self.get_open_date());
                let expenses_wrapped = self
                    .ctx
                    .db
                    .get_expenditures_between_dates(self.ctx.uid, self.ctx.aid, start, end)
                    .unwrap();
                if expenses_wrapped.is_some() {
                    let mut expenses = expenses_wrapped.unwrap();
                    expenses.sort_by(|x, y| {
                        (x.amount)
                            .partial_cmp(&y.amount)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    println!("\nPositions:");
                    for expense in expenses {
                        println!("\t{} | {}", expense.category, expense.amount);
                    }
                } else {
                    println!("\nNo positions found!");
                }
            }
            "None" => {
                return;
            }
            _ => {
                panic!("Unrecognized input!");
            }
        }
    }
}

impl AccountData for CreditCardAccount {}

#[cfg(feature = "ratatui_support")]
impl AccountUI for CreditCardAccount {
    fn populate_page_cache_f32(&self, app: &mut App) {
        let mut kv: HashMap<String, DisplayValue> = HashMap::new();

        kv.insert(
            KEY_TOTAL_VALUE.into(),
            DisplayValue::Float(self.get_value()),
        );
        kv.insert(
            KEY_REMAINING_CREDIT.into(),
            DisplayValue::Float(self.remaining()),
        );
        kv.insert(
            KEY_CREDIT_LINE.into(),
            DisplayValue::Float(self.account_limit()),
        );
        kv.insert(
            KEY_DAYS_UNTIL_DUE.into(),
            DisplayValue::UInt(self.days_until_limit_reset()),
        );
        kv.insert(
            KEY_STATEMENT_DUE_DATE.into(),
            DisplayValue::Text(self.value_reset_date().to_string()),
        );

        app.page_cache_f32 = Some(kv);
        app.ledger_entries = Some(self.get_displayable_ledger());
        app.linechart_cache = None;
        app.barchart_cache = get_budget_barchart_data(self, app);
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        let chunk = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let graphs_reports = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
            .split(chunk[0]);

        let report_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(graphs_reports[0]);

        let value_area = report_chunks[0];
        let due_date = report_chunks[1];

        render_ledger_table(frame, chunk[1], app);
        render_current_value(frame, report_chunks[0], app);
        render_remaining_credit(frame, report_chunks[1], app);
        render_days_until_due_date(frame, report_chunks[2], app);
        render_spend_chart(frame, graphs_reports[1], app);
    }
}

impl Account for CreditCardAccount {
    fn kind(&self) -> AccountType {
        return AccountType::CreditCard;
    }
    #[cfg(feature = "ratatui_support")]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
