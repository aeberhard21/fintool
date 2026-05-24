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
use chrono::format::Fixed;
use chrono::Local;
use chrono::{Days, NaiveDate, NaiveTime};
use core::f32;
use csv::ReaderBuilder;
use inquire::Confirm;
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
        Axis, Bar, BarChart, BarGroup, Block, Borders, Cell, Chart, Clear, Dataset, GraphType,
        HighlightSpacing, LegendPosition, List, ListItem, Padding, Paragraph, Row, Table, Tabs,
        Widget, Wrap,
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
use shared_lib::{FlatLedgerEntry, LedgerEntry, StockInfo};
use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;
use std::{option, rc};

use crate::accounts::base::budget::Budget;
use crate::accounts::base::fixed_account::{FixedAccountFileIO, FixedGrowth, FixedValuable};
use crate::accounts::base::interest_bearing_fixed_account::{
    InterestBearingFixedAccount, InterestBearingLedger,
};
use crate::accounts::base::liquid_account::LiquidAccount;
use crate::accounts::base::{AccountContext, AccountFileIO, Valuable};
use crate::accounts::base::{HasContext, LedgerOps};
use crate::accounts::growth::{report_growth, GrowthCalculable};
use crate::accounts::FilePathHelper;
#[cfg(feature = "ratatui_support")]
use crate::accounts::{render::*, render_table_tabs, KEY_BARCHART_BUDGET, KEY_CASHFLOW_CHART};
#[cfg(feature = "ratatui_support")]
use crate::accounts::{AnalysisPeriod, KEY_SIMPLE_RATE_OF_RETURN, KEY_TOTAL_VALUE};
#[cfg(feature = "ratatui_support")]
use crate::app::app::{App, DisplayValue, LineChart};
#[cfg(feature = "ratatui_support")]
use crate::app::screen::{ledger_table_constraint_len_calculator, CurrentlySelecting};
use crate::database::DbConn;
use crate::tui::get_analysis_period_dates;
use crate::tui::query_user_for_analysis_period;
use crate::types::accounts::AccountInfo;
use crate::types::accounts::AccountRecord;
use crate::types::accounts::AccountTransaction;
use crate::types::accounts::AccountType;
use crate::types::investments::StockRecord;
use crate::types::ledger::DisplayableLedgerRecord;
use crate::types::ledger::LedgerInfo;
use crate::types::ledger::LedgerRecord;
use crate::types::participants;
use crate::types::participants::ParticipantType;
#[cfg(feature = "ratatui_support")]
use crate::ui::{centered_rect, float_range};
use shared_lib::TransferType;

use super::base::fixed_account::FixedAccount;
use super::Account;
use super::AccountCreation;
use super::AccountData;
use super::AccountOperations;
#[cfg(feature = "ratatui_support")]
use super::AccountUI;

pub struct BankAccount {
    ctx: AccountContext,
}

impl HasContext for BankAccount {
    fn ctx(&self) -> &AccountContext {
        &self.ctx
    }
    fn ctx_mut(&mut self) -> &mut AccountContext {
        &mut self.ctx
    }
}

impl InterestBearingLedger for BankAccount {}

impl LedgerOps for BankAccount {
    fn modify(&mut self, selected_record: LedgerRecord) -> Option<LedgerRecord> {
        return self.modify_interest_bearing(selected_record);
    }
}

impl FixedAccount for BankAccount {}

impl InterestBearingFixedAccount for BankAccount {}

impl Valuable for BankAccount {
    fn account_value(&self) -> Option<f32> {
        self.fixed_value()
    }
    fn get_account_value_on_day(&self, day: &NaiveDate) -> Option<f32> {
        self.fixed_value_on_day(day)
    }
}

impl FixedValuable for BankAccount {}

impl GrowthCalculable for BankAccount {
    fn calculate_growth(
        &self,
        metric: super::growth::GrowthMetric,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> f32 {
        self.fixed_growth(metric, start_date, end_date)
    }
}

impl FixedGrowth for BankAccount {}

impl Budget for BankAccount {}

impl AccountFileIO for BankAccount {
    fn import(&self) {
        self.import_fixed_account();
    }
    fn export(&self) {
        self.export_fixed_account();
    }
}

impl FixedAccountFileIO for BankAccount {}

impl BankAccount {
    pub fn new(uid: u32, id: u32, db: &DbConn) -> Self {
        let mut acct: BankAccount = Self {
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
}

impl AccountCreation for BankAccount {
    fn create(uid: u32, name: String, _db: &DbConn) -> AccountRecord {
        let has_bank = true;
        let has_stocks = false;
        let has_ledger = false;
        let has_budget = false;

        let account: AccountInfo = AccountInfo {
            atype: AccountType::Bank,
            name: name,
            has_stocks: has_stocks,
            has_bank: has_bank,
            has_ledger: has_ledger,
            has_budget: has_budget,
        };

        let aid = _db.add_account(uid, &account).unwrap();
        let acct = Self::new(uid, aid, _db);

        let initialize_account =
            Confirm::new("Would you like to open the account with an initial deposit?")
                .prompt()
                .unwrap();

        if initialize_account {
            acct.deposit(None, false);
        }

        let add_budget = Confirm::new("Would you like to associate a budget to this account?")
            .with_default(false)
            .prompt()
            .unwrap();
        if add_budget {
            acct.create_budget();
            acct.set_budget();
        }

        return AccountRecord {
            id: aid,
            info: account,
        };
    }
}

impl AccountOperations for BankAccount {
    fn record(&mut self) {
        let ctx = self.ctx();
        const RECORD_OPTIONS: [&'static str; 6] =
            ["Accrual", "Budget", "Deposit", "Fee", "Withdrawal", "None"];
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
                "Deposit" => {
                    self.deposit(None, false);
                }
                "Withdrawal" => {
                    self.withdrawal(None, false);
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
                        self.create_budget();
                    }
                    <BankAccount as Budget>::record(&self);
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
        const MODIFY_OPTIONS: [&'static str; 4] = ["Ledger", "Categories", "People", "None"];
        const MODIFY_OPTIONS_WITH_BUDGET: [&'static str; 5] =
            ["Ledger", "Categories", "People", "Budget", "None"];
        let options = match self.has_budget() {
            true => MODIFY_OPTIONS_WITH_BUDGET.to_vec(),
            false => MODIFY_OPTIONS.to_vec(),
        };

        loop {
            let modify_choice = Select::new("\nWhat would you like to modify:", options.clone())
                .prompt()
                .unwrap();
            match modify_choice {
                "Ledger" => loop {
                    let record_or_none = self.select_ledger_entry();
                    if record_or_none.is_none() {
                        break;
                    }
                    let selected_record = record_or_none.unwrap();
                    <BankAccount as LedgerOps>::modify(self, selected_record);
                    let go_again = Confirm::new("Modify additional records? (y/n)")
                        .prompt()
                        .unwrap();
                    if !go_again {
                        break;
                    }
                },
                "Categories" => {
                    let ctx = self.ctx().clone();
                    loop {
                        let records = ctx.db.get_categories(ctx.uid, ctx.aid).unwrap();
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
                                ctx.db.update_category_name(
                                    ctx.uid,
                                    ctx.aid,
                                    chosen_category,
                                    new_name,
                                );
                            }
                            "Remove" => {
                                // check if category is referenced by any current ledger
                                let is_referenced = ctx
                                    .db
                                    .check_if_ledger_references_category(
                                        ctx.uid,
                                        ctx.aid,
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
                                            ctx.db
                                                .get_participant(
                                                    ctx.uid,
                                                    ctx.aid,
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
                                    ctx.db.remove_category(
                                        ctx.uid,
                                        ctx.aid,
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
                    let ctx = self.ctx().clone();
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
                        let participants =
                            ctx.db.get_participants(ctx.uid, ctx.aid, ptype).unwrap();
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
                                ctx.db
                                    .update_participant_name(
                                        ctx.uid,
                                        ctx.aid,
                                        ptype,
                                        chosen_person.clone(),
                                        new_name,
                                    )
                                    .unwrap();
                            }
                            "Remove" => {
                                // check if participant is referenced by any current ledger
                                let is_referenced = ctx
                                    .db
                                    .check_if_ledger_references_participant(
                                        ctx.uid,
                                        ctx.aid,
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
                                            ctx.db
                                                .get_category_name(
                                                    ctx.uid,
                                                    ctx.aid,
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
                                            ctx.db
                                                .remove_participant(
                                                    ctx.uid,
                                                    ctx.aid,
                                                    ParticipantType::Payee,
                                                    chosen_person.clone(),
                                                )
                                                .unwrap();
                                        }
                                        ParticipantType::Payer => {
                                            ctx.db
                                                .remove_participant(
                                                    ctx.uid,
                                                    ctx.aid,
                                                    ParticipantType::Payer,
                                                    chosen_person.clone(),
                                                )
                                                .unwrap();
                                        }
                                        _ => {
                                            ctx.db
                                                .remove_participant(
                                                    ctx.uid,
                                                    ctx.aid,
                                                    ParticipantType::Payee,
                                                    chosen_person.clone(),
                                                )
                                                .unwrap();
                                            ctx.db
                                                .remove_participant(
                                                    ctx.uid,
                                                    ctx.aid,
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
                "Budget" => {
                    if self.has_budget() {
                        <BankAccount as Budget>::modify(&self);
                    } else {
                        let add_budget = Confirm::new("A budget for this account does not exist, would you like to create one (y/n)?")
                            .with_default(false)
                            .prompt()
                            .unwrap();
                        if add_budget {
                            self.create_budget();
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

            let go_again = Confirm::new("Modify other elements? (y/n)")
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
        const REPORT_OPTIONS: [&'static str; 3] = ["Total Value", "Growth", "None"];
        let choice: String =
            Select::new("What would you like to report: ", REPORT_OPTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();
        match choice.as_str() {
            "Total Value" => {
                let value = self.get_value();
                println!("\tTotal Account Value: {}", value);
            }
            "Growth" => {
                let rate = report_growth(self).unwrap_or(f32::NAN);
                println!("\tRate of return: {}%", rate);
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

impl BankAccount {
    fn get_growth(&self, start_period: NaiveDate, end_period: NaiveDate) -> f32 {
        return self.calculate_growth(
            super::growth::GrowthMetric::SimpleReturn,
            start_period,
            end_period,
        );
    }
}

impl AccountData for BankAccount {}

impl LiquidAccount for BankAccount {}

#[cfg(feature = "ratatui_support")]
impl AccountUI for BankAccount {
    fn populate_page_cache_f32(&self, app: &mut App) {
        let ctx = self.ctx();
        let mut kv: HashMap<String, DisplayValue> = HashMap::new();

        let start = if app.analysis_start < ctx.open_date {
            ctx.open_date
        } else {
            app.analysis_start
        };

        kv.insert(
            KEY_TOTAL_VALUE.into(),
            DisplayValue::Float(self.get_value()),
        );
        kv.insert(
            KEY_SIMPLE_RATE_OF_RETURN.into(),
            DisplayValue::Float(self.calculate_growth(
                super::growth::GrowthMetric::SimpleReturn,
                start,
                app.analysis_end,
            )),
        );

        app.page_cache_f32 = Some(kv);

        app.ledger_entries = Some(self.get_displayable_ledger());
        app.linechart_cache = get_account_value_linechart(self, app);
        app.barchart_cache.insert(
            KEY_BARCHART_BUDGET.into(),
            get_budget_barchart_data(self, app),
        );
        app.barchart_cache
            .insert(KEY_CASHFLOW_CHART.into(), get_cash_flow_chart(self, app));
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

        let reports_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(graphs_reports[0]);

        let table_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(chunk[1]);
        let table_tab_area = table_chunks[0];
        let ledger_area = table_chunks[1];

        // color according to current selection
        if let Some(current_selection) = app.currently_selected {
            match current_selection {
                CurrentlySelecting::Account => {
                    render_table_tabs(
                        frame,
                        table_tab_area,
                        self.renders_tables(),
                        app.selected_table_tab,
                        Color::Red,
                    );
                }
                CurrentlySelecting::Table => {
                    render_table_tabs(
                        frame,
                        table_tab_area,
                        self.renders_tables(),
                        app.selected_table_tab,
                        Color::Green,
                    );
                }
                _ => {
                    render_table_tabs(
                        frame,
                        table_tab_area,
                        self.renders_tables(),
                        app.selected_table_tab,
                        Color::Reset,
                    );
                }
            }
        }

        match app.selected_table_tab {
            0 => {
                render_ledger_table(frame, ledger_area, app);
            }
            1 => {
                render_cash_flow_chart(frame, ledger_area, app);
            }
            _ => {
                render_spend_chart(frame, ledger_area, app);
            }
        }

        render_current_value(frame, reports_chunks[0], app);
        render_simple_growth(frame, reports_chunks[1], app);
        render_account_value_linechart(frame, graphs_reports[1], app);
    }
}

impl Account for BankAccount {
    fn kind(&self) -> AccountType {
        return AccountType::Bank;
    }
    #[cfg(feature = "ratatui_support")]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    #[cfg(feature = "ratatui_support")]
    fn as_liquid_account(&self) -> Option<&dyn LiquidAccount> {
        return Some(self);
    }
    #[cfg(feature = "ratatui_support")]
    fn renders_tables(&self) -> Vec<String> {
        if self.has_budget() {
            return vec![
                "Transactions".to_string(),
                "Cash Flow".to_string(),
                "Spend Chart".to_string(),
            ];
        } else {
            return vec!["Transactions".to_string(), "Cash Flow".to_string()];
        }
    }
}
