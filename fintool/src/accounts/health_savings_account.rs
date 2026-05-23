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
use chrono::{Datelike, Days, Local, NaiveDate, NaiveTime};
use core::f32;
use core::f64;
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
use rustyline::Helper;
use rustyline::Highlighter;
use rustyline::Hinter;
use rustyline::Validator;
use shared_lib::{FlatLedgerEntry, LedgerEntry};
use std::collections::HashMap;
use std::path::Path;

use crate::accounts::base::fixed_account::FixedAccount;
use crate::accounts::base::interest_bearing_fixed_account::InterestBearingFixedAccount;
use crate::accounts::base::interest_bearing_fixed_account::InterestBearingLedger;
use crate::accounts::base::variable_account::get_positions;
use crate::accounts::base::variable_account::VariableAccountFileIO;
use crate::accounts::base::variable_account::VariableGrowth;
use crate::accounts::base::variable_account::VariableLedger;
use crate::accounts::base::variable_account::VariableValuable;
use crate::accounts::base::variable_account::{
    allocate_sale_stock, allocate_stock_split, confirm_public_ticker, get_position_stats,
    get_value_of_positions_on_day, initialize_buffer, manually_record_stock_close_price,
};
use crate::accounts::base::AccountContext;
use crate::accounts::base::AccountFileIO;
use crate::accounts::base::HasContext;
use crate::accounts::base::HasVariableAccountContext;
use crate::accounts::base::LedgerOps;
use crate::accounts::base::Valuable;
use crate::accounts::base::ValueLimited;
use crate::accounts::base::VariableAccountContext;
use crate::accounts::growth::report_growth;
use crate::accounts::growth::GrowthCalculable;
use crate::accounts::growth::GrowthMetric;
#[cfg(feature = "ratatui_support")]
use crate::accounts::render::*;
use crate::accounts::AnalysisPeriod;
use crate::accounts::FilePathHelper;
use crate::accounts::{
    KEY_COMPOUNDED_ANNUAL_RATE_OF_RETURN, KEY_CONTRIBUTION_LIMIT,
    KEY_MONEY_WEIGHTED_RATE_OF_RETURN, KEY_REMAINING_CONTRIBUTION,
    KEY_TIME_WEIGHTED_RATE_OF_RETURN,
};
#[cfg(feature = "ratatui_support")]
use crate::app::app::{App, DisplayValue, LineChart};
#[cfg(feature = "ratatui_support")]
use crate::app::screen::CurrentlySelecting;
use crate::database::DbConn;
use crate::tui::get_analysis_period_dates;
use crate::tui::query_user_for_analysis_period;
use crate::types::accounts::AccountInfo;
use crate::types::accounts::AccountRecord;
use crate::types::accounts::AccountTransaction;
use crate::types::accounts::AccountType;
use crate::types::hsa::HsaInfo;
use crate::types::investments::StockInfo;
use crate::types::investments::StockRecord;
use crate::types::investments::StockSplitInfo;
use crate::types::investments::StockSplitRecord;
use crate::types::ledger::LedgerInfo;
use crate::types::ledger::LedgerRecord;
use crate::types::participants::ParticipantType;
use crate::types::stock_prices::StockPriceInfo;
use csv::ReaderBuilder;
use rustyline::Editor;
use shared_lib::TransferType;

use super::base::variable_account::VariableAccount;
#[cfg(feature = "ratatui_support")]
use super::render_table_tabs;
use super::Account;
use super::AccountCreation;
use super::AccountData;
use super::AccountOperations;
#[cfg(feature = "ratatui_support")]
use super::AccountUI;
use super::KEY_TOTAL_VALUE;
#[cfg(feature = "ratatui_support")]
use crate::ui::{centered_rect, float_range};

pub struct HealthSavingsAccount {
    ctx: AccountContext,
    vctx: VariableAccountContext,
}

impl HasContext for HealthSavingsAccount {
    fn ctx(&self) -> &AccountContext {
        &self.ctx
    }
    fn ctx_mut(&mut self) -> &mut AccountContext {
        &mut self.ctx
    }
}

impl HasVariableAccountContext for HealthSavingsAccount {
    fn variable_ctx(&self) -> &VariableAccountContext {
        &self.vctx
    }
    fn variable_ctx_mut(&mut self) -> &mut VariableAccountContext {
        &mut self.vctx
    }
}

impl VariableAccount for HealthSavingsAccount {}

impl LedgerOps for HealthSavingsAccount {
    fn modify(&mut self, selected_record: LedgerRecord) -> Option<LedgerRecord> {
        self.modify_variable(selected_record)
    }
}

impl InterestBearingLedger for HealthSavingsAccount {}

impl VariableLedger for HealthSavingsAccount {}

impl InterestBearingFixedAccount for HealthSavingsAccount {}

impl FixedAccount for HealthSavingsAccount {}

impl Valuable for HealthSavingsAccount {
    fn account_value(&self) -> Option<f32> {
        self.variable_value()
    }

    fn get_account_value_on_day(&self, day: &NaiveDate) -> Option<f32> {
        self.variable_value_on_day(day)
    }
}

impl VariableValuable for HealthSavingsAccount {}

impl GrowthCalculable for HealthSavingsAccount {
    fn calculate_growth(
        &self,
        metric: GrowthMetric,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> f32 {
        self.variable_growth(metric, start_date, end_date)
    }
}

impl VariableGrowth for HealthSavingsAccount {}

impl ValueLimited for HealthSavingsAccount {
    fn account_limit(&self) -> f32 {
        let acct = self.ctx.db.get_hsa(self.ctx.uid, self.ctx.aid).unwrap();
        return acct.info.contribution_limit;
    }
    fn remaining(&self) -> f32 {
        let contribution_limit = self.account_limit();
        let (start, end) = get_analysis_period_dates(self.get_open_date(), &AnalysisPeriod::YTD);
        let contributions_ytd = self
            .ctx
            .db
            .get_ledger_entries_within_timestamps(self.ctx.uid, self.ctx.aid, start, end)
            .unwrap();
        let aggregate: f32 = contributions_ytd
            .iter()
            .filter(|x| x.info.transfer_type == TransferType::DepositFromExternalAccount)
            .map(|x| x.info.amount)
            .sum();
        contribution_limit - aggregate
    }
}

impl AccountFileIO for HealthSavingsAccount {
    fn import(&self) {
        self.import_variable_account();
    }
    fn export(&self) {
        self.export_variable_account();
    }
}

impl VariableAccountFileIO for HealthSavingsAccount {}

impl AccountCreation for HealthSavingsAccount {
    fn create(uid: u32, name: String, _db: &DbConn) -> AccountRecord {
        let has_bank = true;
        let has_stocks = true;
        let has_ledger = false;
        let has_budget = false;

        let account: AccountInfo = AccountInfo {
            atype: AccountType::HealthSavingsAccount,
            name: name,
            has_stocks: has_stocks,
            has_bank: has_bank,
            has_ledger: has_ledger,
            has_budget: has_budget,
        };

        let aid = _db.add_account(uid, &account).unwrap();
        let acct = Self::new(uid, aid, _db);

        let contribution_limit = CustomType::<f32>::new("Enter contribution limit:")
            .with_placeholder("4000.00")
            .with_default(7000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let hsa_info = HsaInfo {
            contribution_limit: contribution_limit,
        };

        _db.add_hsa_account(uid, aid, hsa_info).unwrap();

        let initialize_account =
            Confirm::new("Would you like to open the account with an initial deposit?")
                .prompt()
                .unwrap();

        if initialize_account {
            acct.deposit(None, false);
        }

        return AccountRecord {
            id: aid,
            info: account,
        };
    }
}

impl HealthSavingsAccount {
    pub fn new(uid: u32, id: u32, db: &DbConn) -> Self {
        let mut ledger = db.get_ledger(uid, id).unwrap();
        let open_date = if !ledger.is_empty() {
            ledger.sort_by(|l1, l2| (&l1.info.date).cmp(&l2.info.date));
            NaiveDate::parse_from_str(&ledger[0].info.date, "%Y-%m-%d").unwrap()
        } else {
            Local::now().date_naive()
        };

        let mut acct = Self {
            ctx: AccountContext {
                uid: uid,
                aid: id,
                db: db.clone(),
                open_date: open_date,
            },
            vctx: VariableAccountContext { buffer: None },
        };

        let data = initialize_buffer(&acct.ctx, &acct.vctx);
        acct.vctx.buffer = data;

        acct
    }
}

impl AccountOperations for HealthSavingsAccount {
    fn record(&mut self) {
        const RECORD_OPTIONS: [&'static str; 9] = [
            "Accrual",
            "Deposit",
            "Fee",
            "Purchase",
            "Sale",
            "Stock Split",
            "Stock Price",
            "Withdrawal",
            "None",
        ];
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
                "Purchase" => {
                    self.purchase_stock(None, false);
                }
                "Sale" => {
                    self.sell_stock(None, false);
                }
                "Stock Split" => {
                    self.split_stock(None, false);
                }
                "Stock Price" => {
                    manually_record_stock_close_price(self.ctx());
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
        <Self as AccountFileIO>::import(self);
        // initialize buffer after import
        let data = initialize_buffer(&self.ctx, &self.vctx);
        self.vctx.buffer = data;
    }

    fn modify(&mut self) {
        const MODIFY_OPTIONS: [&'static str; 5] = [
            "Ledger",
            "Categories",
            "Contribution Limit",
            "Participant",
            "None",
        ];
        loop {
            let modify_choice =
                Select::new("\nWhat would you like to modify:", MODIFY_OPTIONS.to_vec())
                    .prompt()
                    .unwrap();
            match modify_choice {
                "Ledger" => loop {
                    let record_or_none = self.select_ledger_entry();
                    if record_or_none.is_none() {
                        break;
                    }
                    let selected_record = record_or_none.unwrap();
                    <HealthSavingsAccount as LedgerOps>::modify(self, selected_record);
                    let go_again = Confirm::new("Modify additional records? (y/n)")
                        .prompt()
                        .unwrap();
                    if !go_again {
                        break;
                    }
                },
                "Contribution Limit" => {
                    let hsa = self.ctx.db.get_hsa(self.ctx.uid, self.ctx.aid).unwrap();
                    let new_contribution_limit =
                        CustomType::<f32>::new("Enter new contribution limit:")
                            .with_default(hsa.info.contribution_limit)
                            .with_error_message("Please type a valid amount!")
                            .prompt()
                            .unwrap();
                    let _ = self.ctx.db.update_hsa_contribution_limit(
                        self.ctx.uid,
                        self.ctx.aid,
                        new_contribution_limit,
                    );
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
                "Participant" => {
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
                        let go_again = Confirm::new("Modify additional participants? (y/n)")
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
        <Self as AccountFileIO>::export(self);
    }

    fn report(&self) {
        const REPORT_OPTIONS: [&'static str; 4] = ["Positions", "Total Value", "Growth", "None"];
        let choice = Select::new("What would you like to report: ", REPORT_OPTIONS.to_vec())
            .prompt()
            .unwrap()
            .to_string();
        match choice.as_str() {
            "Positions" => {
                let positions_wrapped = get_positions(&self.ctx);
                if positions_wrapped.is_some() {
                    let positions = positions_wrapped.unwrap();
                    println!("\nPositions:");
                    for position in positions {
                        println!("\t{} | {}", position.0, position.1);
                    }
                } else {
                    println!("\nNo positions found!");
                }
            }
            "Total Value" => {
                let value = self.get_value();
                println!("\tTotal Account Value: {}", value);
                println!(
                    "\t\tFixed Account Value: {}",
                    self.fixed_value().unwrap_or(f32::NAN)
                );
                let today = Local::now().date_naive();
                println!(
                    "\t\tVariable Account Value: {}",
                    get_value_of_positions_on_day(self.ctx(), self.variable_ctx(), &today)
                );
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

impl AccountData for HealthSavingsAccount {}

#[cfg(feature = "ratatui_support")]
impl AccountUI for HealthSavingsAccount {
    fn populate_page_cache_f32(&self, app: &mut App) {
        let mut kv: HashMap<String, DisplayValue> = HashMap::new();

        let start = if app.analysis_start < self.get_open_date() {
            self.get_open_date()
        } else {
            app.analysis_start
        };

        kv.insert(
            KEY_TOTAL_VALUE.into(),
            DisplayValue::Float(self.get_value()),
        );
        kv.insert(
            KEY_TIME_WEIGHTED_RATE_OF_RETURN.into(),
            DisplayValue::Float(self.calculate_growth(GrowthMetric::TWRR, start, app.analysis_end)),
        );
        kv.insert(
            KEY_COMPOUNDED_ANNUAL_RATE_OF_RETURN.into(),
            DisplayValue::Float(self.calculate_growth(GrowthMetric::CAGR, start, app.analysis_end)),
        );
        kv.insert(
            KEY_MONEY_WEIGHTED_RATE_OF_RETURN.into(),
            DisplayValue::Float(self.calculate_growth(GrowthMetric::MWRR, start, app.analysis_end)),
        );
        kv.insert(
            KEY_REMAINING_CONTRIBUTION.into(),
            DisplayValue::Float(self.remaining()),
        );
        kv.insert(
            KEY_CONTRIBUTION_LIMIT.into(),
            DisplayValue::Float(self.account_limit()),
        );

        app.page_cache_f32 = Some(kv);
        app.ledger_entries = Some(self.get_displayable_ledger());
        app.linechart_cache = get_time_period_investment_linechart(self, app);
        app.barchart_cache = None;
        app.positions_entries = get_position_stats(self.ctx(), self.variable_ctx());
    }

    fn render(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        let chunk = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let data_area = chunk[0];
        let table_area = chunk[1];

        let reports_graphs = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
            .split(data_area);

        let report_area = reports_graphs[0];
        let graph_area = reports_graphs[1];

        let reports_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(report_area);

        let account_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(reports_chunks[0]);

        let value_area = account_area[0];
        let contribution_area = account_area[1];

        let growth_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(reports_chunks[1]);

        let twrr_area = growth_area[0];
        let mwrr_area = growth_area[1];
        let cagr_area = growth_area[2];

        let table_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(table_area);

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
            _ => {
                render_positions_table(frame, ledger_area, app);
            }
        }
        render_time_period_investment_linechart(frame, graph_area, app);
        render_current_value(frame, value_area, app);
        render_remaining_contribution(frame, contribution_area, app);
        render_time_weighted_rate_of_return(frame, twrr_area, app);
        render_annualized_rate_of_return(frame, cagr_area, app);
        render_money_weighted_rate_of_return(frame, mwrr_area, app);
    }
}

impl Account for HealthSavingsAccount {
    fn kind(&self) -> AccountType {
        return AccountType::HealthSavingsAccount;
    }
    #[cfg(feature = "ratatui_support")]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn has_budget(&self) -> bool {
        let acct = self.ctx.db.get_account(self.ctx.uid, self.ctx.aid).unwrap();
        acct.info.has_budget
    }
    fn set_budget(&self) {
        let mut acct = self.ctx.db.get_account(self.ctx.uid, self.ctx.aid).unwrap();
        acct.info.has_budget = true;
        let _ = self
            .ctx
            .db
            .update_account(self.ctx.uid, self.ctx.aid, &acct.info)
            .unwrap();
    }
    #[cfg(feature = "ratatui_support")]
    fn renders_tables(&self) -> Vec<String> {
        return vec!["Transactions".to_string(), "Positions".to_string()];
    }
}
