use chrono::format::Fixed;
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
use chrono::{Days, Local, Months, NaiveDate, NaiveTime};
use core::f32;
use csv::ReaderBuilder;
use inquire::Confirm;
use inquire::CustomType;
use inquire::DateSelect;
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
use shared_lib::LedgerEntry;
use std::collections::HashMap;
use std::path::Path;

use crate::accounts::base::budget::Budget;
use crate::accounts::base::fixed_account::{FixedAccountFileIO, FixedGrowth, FixedValuable};
use crate::accounts::base::interest_bearing_fixed_account::{
    InterestBearingFixedAccount, InterestBearingLedger,
};
use crate::accounts::base::liquid_account::LiquidAccount;
use crate::accounts::base::{AccountContext, AccountFileIO, Valuable};
use crate::accounts::base::{HasContext, LedgerOps};
use crate::accounts::growth::GrowthCalculable;
use crate::accounts::growth::GrowthMetric;
#[cfg(feature = "ratatui_support")]
use crate::accounts::render::*;
use crate::accounts::FilePathHelper;
#[cfg(feature = "ratatui_support")]
use crate::accounts::{
    KEY_COMPOUNDED_ANNUAL_RATE_OF_RETURN, KEY_DAYS_TO_MATURITY, KEY_MATURITY_DATE,
    KEY_SIMPLE_RATE_OF_RETURN,
};
#[cfg(feature = "ratatui_support")]
use crate::app::app::{App, DisplayValue, LineChart};
#[cfg(feature = "ratatui_support")]
use crate::app::screen::ledger_table_constraint_len_calculator;
use crate::database::DbConn;
use crate::tui::get_analysis_period_dates;
use crate::tui::query_user_for_analysis_period;
use crate::types::accounts::AccountInfo;
use crate::types::accounts::AccountRecord;
use crate::types::accounts::AccountTransaction;
use crate::types::accounts::AccountType;
use crate::types::certificate_of_deposit::CertificateOfDepositInfo;
use crate::types::ledger::LedgerInfo;
use crate::types::ledger::LedgerRecord;
use crate::types::participants;
use crate::types::participants::ParticipantType;
#[cfg(feature = "ratatui_support")]
use crate::ui::{centered_rect, float_range};
use shared_lib::{FlatLedgerEntry, TransferType};

use super::base::fixed_account::FixedAccount;
use super::Account;
use super::AccountCreation;
use super::AccountData;
use super::AccountOperations;
#[cfg(feature = "ratatui_support")]
use super::AccountUI;
use super::AnalysisPeriod;
use super::KEY_TOTAL_VALUE;

pub struct CertificateOfDepositAccount {
    ctx: AccountContext,
}

impl HasContext for CertificateOfDepositAccount {
    fn ctx(&self) -> &AccountContext {
        &self.ctx
    }
    fn ctx_mut(&mut self) -> &mut AccountContext {
        &mut self.ctx
    }
}

impl InterestBearingLedger for CertificateOfDepositAccount {}

impl LedgerOps for CertificateOfDepositAccount {
    fn modify(&mut self, selected_record: LedgerRecord) -> Option<LedgerRecord> {
        self.modify_interest_bearing(selected_record)
    }
}

impl FixedAccount for CertificateOfDepositAccount {}

impl InterestBearingFixedAccount for CertificateOfDepositAccount {}

impl Valuable for CertificateOfDepositAccount {
    fn account_value(&self) -> Option<f32> {
        self.fixed_value()
    }
    fn get_account_value_on_day(&self, day: &NaiveDate) -> Option<f32> {
        self.fixed_value_on_day(day)
    }
}

impl FixedValuable for CertificateOfDepositAccount {}

impl GrowthCalculable for CertificateOfDepositAccount {
    fn calculate_growth(
        &self,
        metric: super::growth::GrowthMetric,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> f32 {
        self.fixed_growth(metric, start_date, end_date)
    }
}

impl FixedGrowth for CertificateOfDepositAccount {}

impl Budget for CertificateOfDepositAccount {}

impl AccountFileIO for CertificateOfDepositAccount {
    fn import(&self) {
        self.import_fixed_account();
    }
    fn export(&self) {
        self.export_fixed_account();
    }
}

impl FixedAccountFileIO for CertificateOfDepositAccount {}

impl CertificateOfDepositAccount {
    pub fn new(uid: u32, id: u32, db: &DbConn) -> Self {
        let mut acct: CertificateOfDepositAccount = Self {
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

impl AccountCreation for CertificateOfDepositAccount {
    fn create(uid: u32, name: String, _db: &DbConn) -> AccountRecord {
        let has_bank = true;
        let has_stocks = false;
        let has_ledger = false;
        let has_budget = false;

        let account: AccountInfo = AccountInfo {
            atype: AccountType::CD,
            name: name,
            has_stocks: has_stocks,
            has_bank: has_bank,
            has_ledger: has_ledger,
            has_budget: has_budget,
        };

        let aid = _db.add_account(uid, &account).unwrap();

        let mut cd: CertificateOfDepositAccount = CertificateOfDepositAccount::new(uid, aid, _db);

        let principal = CustomType::<f32>::new("Enter principal:")
            .with_placeholder("10000.00")
            .with_default(10000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let apy = CustomType::<f32>::new("Enter annual percentage yield:")
            .with_placeholder("3.00")
            .with_default(3.00)
            .with_error_message("Please type a valid percentage!")
            .prompt()
            .unwrap();

        let open_date = DateSelect::new("Enter open date:").prompt().unwrap();

        let length = CustomType::<u32>::new("Enter length (in months) to maturity:")
            .with_placeholder("12")
            .with_default(12)
            .with_error_message("Please type a valid number!")
            .prompt()
            .unwrap();

        let maturity_date = open_date.checked_add_months(Months::new(length)).unwrap();

        let cd_info = CertificateOfDepositInfo {
            apy: apy,
            principal: principal,
            maturity_date: maturity_date.format("%Y-%m-%d").to_string(),
            length_months: length,
        };

        _db.add_certificate_of_deposit(uid, aid, cd_info.clone())
            .unwrap();

        let initialize_ledger = Confirm::new("Initialize ledger with principal?")
            .prompt()
            .unwrap();

        if initialize_ledger {
            let link = Confirm::new("Link transaction to another account?")
                .prompt()
                .unwrap();
            let input = if link {
                cd.link_transaction(None)
            } else {
                None
            };

            let peer = if input.is_none() {
                let payer = Text::new("Enter payer:").prompt().unwrap();
                (None, payer)
            } else {
                let (acct, account_name) = input.unwrap();
                (Some(acct), account_name)
            };

            let initial = crate::types::ledger::LedgerInfo {
                date: open_date.format("%Y-%m-%d").to_string(),
                amount: principal,
                transfer_type: TransferType::DepositFromExternalAccount,
                participant: _db.check_and_add_participant(
                    uid,
                    aid,
                    peer.1,
                    ParticipantType::Payer,
                    peer.0.is_some(),
                ),
                category_id: _db.check_and_add_category(
                    uid,
                    aid,
                    "Deposit".to_ascii_uppercase().to_string(),
                ),
                description: format!(
                    "Open {} APY {} month CD with ${}",
                    cd_info.apy, cd_info.length_months, cd_info.principal
                ),
            };

            let lid = _db.add_ledger_entry(uid, aid, initial.clone()).unwrap();
            if peer.0.is_some() {
                peer.0.unwrap().link(
                    cd.ctx.aid,
                    LedgerRecord {
                        id: lid,
                        info: initial,
                    },
                );
            }
        }

        return AccountRecord {
            id: aid,
            info: account,
        };
    }
}

impl AccountOperations for CertificateOfDepositAccount {
    fn record(&mut self) {
        const RECORD_OPTIONS: [&'static str; 3] = ["Deposit", "Withdrawal", "None"];
        loop {
            let action = Select::new(
                "\nWhat transaction would you like to record?",
                RECORD_OPTIONS.to_vec(),
            )
            .prompt()
            .unwrap()
            .to_string();
            match action.as_str() {
                "Deposit" => {
                    <CertificateOfDepositAccount as FixedAccount>::deposit(self, None, false);
                }
                "Withdrawal" => {
                    <CertificateOfDepositAccount as FixedAccount>::withdrawal(self, None, false);
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
        const MODIFY_OPTIONS: [&'static str; 7] = [
            "APY",
            "Ledger",
            "Length",
            "Categories",
            "People",
            "Principal",
            "None",
        ];
        loop {
            let modify_choice =
                Select::new("\nWhat would you like to modify:", MODIFY_OPTIONS.to_vec())
                    .prompt()
                    .unwrap();
            match modify_choice {
                "APY" => {
                    let cd = self
                        .ctx
                        .db
                        .get_certificate_of_deposit(self.ctx.uid, self.ctx.aid)
                        .unwrap();
                    let updated_apy = CustomType::<f32>::new("Enter annual percentage yield:")
                        .with_placeholder("3.00")
                        .with_default(cd.info.apy)
                        .with_error_message("Please type a valid percentage!")
                        .prompt()
                        .unwrap();
                    self.ctx
                        .db
                        .update_cd_apy(self.ctx.uid, self.ctx.aid, updated_apy)
                        .unwrap();
                }
                "Ledger" => {
                    loop {
                        let record_or_none = self.select_ledger_entry();
                        if record_or_none.is_none() {
                            break;
                        }
                        let selected_record = record_or_none.unwrap();
                        let updated_record_opt = <CertificateOfDepositAccount as LedgerOps>::modify(
                            self,
                            selected_record.clone(),
                        );
                        if updated_record_opt.is_none() {
                            break;
                        }
                        let updated_record = updated_record_opt.unwrap();
                        // record 0 should always be the initial of the account.
                        // if the date of the deposit changed, then so should the maturity date
                        if updated_record.id == 0 {
                            let cd = self
                                .ctx
                                .db
                                .get_certificate_of_deposit(self.ctx.uid, self.ctx.aid)
                                .unwrap();
                            if selected_record.info.date != updated_record.info.date {
                                let new_date_nv = NaiveDate::parse_from_str(
                                    &updated_record.info.date.as_str(),
                                    "%Y-%m-%d",
                                )
                                .unwrap();
                                let updated_maturity_date = new_date_nv
                                    .checked_add_months(Months::new(cd.info.length_months))
                                    .unwrap()
                                    .format("%Y-%m-%d")
                                    .to_string();
                                self.ctx
                                    .db
                                    .update_cd_maturity_date(
                                        self.ctx.uid,
                                        self.ctx.aid,
                                        updated_maturity_date,
                                    )
                                    .unwrap();
                            }

                            let go_again = Confirm::new("Modify additional records? (y/n)")
                                .prompt()
                                .unwrap();
                            if !go_again {
                                break;
                            }
                        }
                    }
                }
                "Length" => {
                    let cd = self
                        .ctx
                        .db
                        .get_certificate_of_deposit(self.ctx.uid, self.ctx.aid)
                        .unwrap();
                    let updated_length =
                        CustomType::<u32>::new("Enter length (in months) to maturity:")
                            .with_placeholder("12")
                            .with_default(cd.info.length_months)
                            .with_error_message("Please type a valid number!")
                            .prompt()
                            .unwrap();
                    let len_difference = (updated_length as i32) - (cd.info.length_months as i32);
                    let updated_maturity_date = if len_difference > 0 {
                        NaiveDate::parse_from_str(&cd.info.maturity_date.as_str(), "%Y-%m-%d")
                            .unwrap()
                            .checked_add_months(Months::new(len_difference as u32))
                            .unwrap()
                            .format("%Y-%m-%d")
                            .to_string()
                    } else {
                        NaiveDate::parse_from_str(&cd.info.maturity_date.as_str(), "%Y-%m-%d")
                            .unwrap()
                            .checked_sub_months(Months::new((0 - len_difference) as u32))
                            .unwrap()
                            .format("%Y-%m-%d")
                            .to_string()
                    };
                    self.ctx
                        .db
                        .update_cd_length(self.ctx.uid, self.ctx.aid, updated_length)
                        .unwrap();
                    self.ctx
                        .db
                        .update_cd_maturity_date(self.ctx.uid, self.ctx.aid, updated_maturity_date)
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
                                self.ctx
                                    .db
                                    .update_category_name(
                                        self.ctx.uid,
                                        self.ctx.aid,
                                        chosen_category,
                                        new_name,
                                    )
                                    .unwrap();
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
                "Principal" => {
                    let cd = self
                        .ctx
                        .db
                        .get_certificate_of_deposit(self.ctx.uid, self.ctx.aid)
                        .unwrap();
                    let updated_principal = CustomType::<f32>::new("Enter principal:")
                        .with_placeholder("10000.00")
                        .with_default(cd.info.principal)
                        .with_error_message("Please type a valid amount!")
                        .prompt()
                        .unwrap();
                    self.ctx
                        .db
                        .update_cd_principal(self.ctx.uid, self.ctx.aid, updated_principal)
                        .unwrap();
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
        const REPORT_OPTIONS: [&'static str; 3] = ["Total Value", "Simple Growth Rate", "None"];
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
            "Simple Growth Rate" => {
                let (period_start, period_end, _) =
                    query_user_for_analysis_period(self.get_open_date());
                let rate = self.calculate_growth(
                    crate::accounts::growth::GrowthMetric::SimpleReturn,
                    period_start,
                    period_end,
                );
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

impl AccountData for CertificateOfDepositAccount {}

#[cfg(feature = "ratatui_support")]
impl AccountUI for CertificateOfDepositAccount {
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
            KEY_COMPOUNDED_ANNUAL_RATE_OF_RETURN.into(),
            DisplayValue::Float(self.calculate_growth(GrowthMetric::CAGR, start, app.analysis_end)),
        );
        kv.insert(
            KEY_MATURITY_DATE.into(),
            DisplayValue::Text(self.get_maturity_date()),
        );
        kv.insert(
            KEY_DAYS_TO_MATURITY.into(),
            DisplayValue::UInt(self.get_days_to_maturity()),
        );

        app.page_cache_f32 = Some(kv);
        app.ledger_entries = Some(self.get_displayable_ledger());
        app.linechart_cache = get_account_value_linechart(self, app);
        app.barchart_cache = None;
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

        let report_area = graphs_reports[0];
        let chart_area = graphs_reports[1];

        let report_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(report_area);

        let value_area = report_chunks[0];
        let growth_area = report_chunks[1];
        let maturity_area = report_chunks[2];

        render_current_value(frame, value_area, app);
        render_simple_growth(frame, growth_area, app);
        render_days_to_maturity(frame, maturity_area, app);
        render_account_value_linechart(frame, chart_area, app);
        render_ledger_table(frame, chunk[1], app);
    }
}

#[cfg(feature = "ratatui_support")]
impl CertificateOfDepositAccount {
    fn get_maturity_date(&self) -> String {
        let cd = self
            .ctx
            .db
            .get_certificate_of_deposit(self.ctx.uid, self.ctx.aid)
            .unwrap();
        return cd.info.maturity_date;
    }

    fn get_days_to_maturity(&self) -> u32 {
        use chrono::Datelike;
        let maturity_date = self.get_maturity_date();
        let maturity_date_naive = NaiveDate::parse_from_str(&maturity_date, "%Y-%m-%d").unwrap();
        let local = Local::now().date_naive();
        return (maturity_date_naive.num_days_from_ce() - local.num_days_from_ce()) as u32;
    }
}

impl Account for CertificateOfDepositAccount {
    fn kind(&self) -> AccountType {
        return AccountType::CD;
    }
    #[cfg(feature = "ratatui_support")]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
