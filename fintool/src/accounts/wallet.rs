use chrono::Date;
use chrono::Local;
use chrono::NaiveDate;
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
        HighlightSpacing, List, ListItem, Paragraph, Row, Table, Tabs, Widget, Wrap,
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
use std::hash::Hash;
use std::path::Path;
use std::rc;

#[cfg(feature = "ratatui_support")]
use crate::app::app::App;
#[cfg(feature = "ratatui_support")]
use crate::app::screen::ledger_table_constraint_len_calculator;
use crate::database::DbConn;
use crate::tui::query_user_for_analysis_period;
use crate::tui::get_analysis_period_dates;
use crate::types::accounts::AccountInfo;
use crate::types::accounts::AccountRecord;
use crate::types::accounts::AccountTransaction;
use crate::types::accounts::AccountType;
use crate::types::ledger::LedgerInfo;
use crate::types::ledger::LedgerRecord;
use crate::types::participants;
use crate::types::participants::ParticipantType;
use shared_lib::TransferType;

use super::base::fixed_account::FixedAccount;
use super::base::Account;
use super::base::AccountCreation;
use super::base::AccountData;
use super::base::AccountOperations;
#[cfg(feature = "ratatui_support")]
use super::base::AccountUI;
#[cfg(feature = "ratatui_support")]
use crate::ui::{centered_rect, float_range};
use crate::types::ledger::Expenditure;

pub struct Wallet {
    uid: u32,
    id: u32,
    db: DbConn,
    fixed: FixedAccount,
    open_date : NaiveDate,
}

#[derive(Helper, Completer, Hinter, Highlighter, Validator)]
struct FilePathHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    #[rustyline(Highlighter)]
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Wallet {
    pub fn new(uid: u32, id: u32, db: &DbConn) -> Self {
        let mut acct: Wallet = Self {
            uid: uid,
            id: id,
            db: db.clone(),
            fixed: FixedAccount::new(uid, id, db.clone()),
            open_date : Local::now().date_naive()
        };

        let mut ledger = acct.get_ledger();
        if !ledger.is_empty() { 
            ledger.sort_by(|l1, l2| (&l1.info.date).cmp(&l2.info.date));
            acct.open_date = NaiveDate::parse_from_str(&ledger[0].info.date, "%Y-%m-%d").unwrap();
        }

        acct
    }
}

impl AccountCreation for Wallet {
    fn create(uid: u32, name: String, _db: &DbConn) -> AccountRecord {
        let has_bank = true;
        let has_stocks = false;
        let has_ledger = false;
        let has_budget = false;

        let account: AccountInfo = AccountInfo {
            atype: AccountType::Wallet,
            name: name,
            has_stocks: has_stocks,
            has_bank: has_bank,
            has_ledger: has_ledger,
            has_budget: has_budget,
        };

        let aid = _db.add_account(uid, &account).unwrap();

        return AccountRecord {
            id: aid,
            info: account,
        };
    }
}

impl AccountOperations for Wallet {
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
                    self.fixed.deposit(None, false);
                }
                "Withdrawal" => {
                    self.fixed.withdrawal(None, false);
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
            csv = rl.readline("Enter path to CSV file: ").unwrap();
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
                participant: self.db.check_and_add_participant(
                    self.uid,
                    self.id,
                    rcrd.participant,
                    ptype,
                    false,
                ),
                category_id: self.db.check_and_add_category(
                    self.uid,
                    self.id,
                    rcrd.category.to_ascii_uppercase(),
                ),
                description: rcrd.description,
                ancillary_f32data: 0.0,
            };
            let _lid: u32 = self.db.add_ledger_entry(self.uid, self.id, entry).unwrap();
        }
    }

    fn modify(&mut self) {
        const MODIFY_OPTIONS: [&'static str; 4] = ["Ledger", "Categories", "People", "None"];
        let modify_choice =
            Select::new("\nWhat would you like to modify:", MODIFY_OPTIONS.to_vec())
                .prompt()
                .unwrap();
        match modify_choice {
            "Ledger" => {
                let record_or_none = self.fixed.select_ledger_entry();
                if record_or_none.is_none() {
                    return;
                }
                let selected_record = record_or_none.unwrap();
                self.fixed.modify(selected_record);
            }
            "Categories" => {
                let records = self.db.get_categories(self.uid, self.id).unwrap();
                let mut choices: Vec<String> = records
                    .iter()
                    .map(|x| x.category.name.clone())
                    .collect::<Vec<String>>();
                choices.push("None".to_string());
                let chosen_category = Select::new("Select category to modify:", choices)
                    .prompt()
                    .unwrap();

                if chosen_category == "None" {
                    return;
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
                        self.db
                            .update_category_name(self.uid, self.id, chosen_category, new_name);
                    }
                    "Remove" => {
                        // check if category is referenced by any current ledger
                        let is_referenced = self
                            .db
                            .check_if_ledger_references_category(
                                self.uid,
                                self.id,
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
                                    self.db
                                        .get_participant(self.uid, self.id, record.info.participant)
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
                            self.db
                                .remove_category(self.uid, self.id, chosen_category.clone());
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
            "People" => {
                const PTYPE_OPTIONS: [&'static str; 3] = ["Payer", "Payee", "Both"];
                let selected_ptype = Select::new("What type of person:", PTYPE_OPTIONS.to_vec())
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
                let participants = self.db.get_participants(self.uid, self.id, ptype).unwrap();
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
                    return;
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
                        self.db
                            .update_participant_name(
                                self.uid,
                                self.id,
                                ptype,
                                chosen_person.clone(),
                                new_name,
                            )
                            .unwrap();
                    }
                    "Remove" => {
                        // check if participant is referenced by any current ledger
                        let is_referenced = self
                            .db
                            .check_if_ledger_references_participant(
                                self.uid,
                                self.id,
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
                                    self.db
                                        .get_category_name(
                                            self.uid,
                                            self.id,
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
                                    self.db
                                        .remove_participant(
                                            self.uid,
                                            self.id,
                                            ParticipantType::Payee,
                                            chosen_person.clone(),
                                        )
                                        .unwrap();
                                }
                                ParticipantType::Payer => {
                                    self.db
                                        .remove_participant(
                                            self.uid,
                                            self.id,
                                            ParticipantType::Payer,
                                            chosen_person.clone(),
                                        )
                                        .unwrap();
                                }
                                _ => {
                                    self.db
                                        .remove_participant(
                                            self.uid,
                                            self.id,
                                            ParticipantType::Payee,
                                            chosen_person.clone(),
                                        )
                                        .unwrap();
                                    self.db
                                        .remove_participant(
                                            self.uid,
                                            self.id,
                                            ParticipantType::Payer,
                                            chosen_person.clone(),
                                        )
                                        .unwrap();
                                }
                            }
                        }
                    }
                    "None" => {
                        return;
                    }
                    _ => {
                        panic!("Unrecognized input: {}", update_or_remove);
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
    }

    fn export(&self) {}

    fn report(&self) {
        const REPORT_OPTIONS: [&'static str; 3] = ["Total Value", "Simple Growth Rate", "None"];
        let choice: String =
            Select::new("What would you like to report: ", REPORT_OPTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();
        match choice.as_str() {
            "Total Value" => {
                let value = self.fixed.get_current_value();
                println!("\tTotal Account Value: {}", value);
            }
            "Simple Growth Rate" => {
                let (period_start, period_end, _) = query_user_for_analysis_period(self.get_open_date());
                let rate = self.fixed.simple_rate_of_return(period_start, period_end);
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

    fn link(&self, transacting_account: u32, entry: LedgerRecord) -> Option<u32> {
        let from_account;
        let to_account;

        let cid;
        let pid;
        let transacting_account_name: String;
        let (new_ttype, description) = match entry.info.transfer_type {
            TransferType::DepositFromExternalAccount => {
                // if the transacting account received a deposit, then self must be the "from" account
                from_account = self.id;
                to_account = transacting_account;
                cid = self.db.check_and_add_category(
                    self.uid,
                    self.id,
                    "Withdrawal".to_ascii_uppercase(),
                );
                transacting_account_name = self
                    .db
                    .get_account_name(self.uid, transacting_account)
                    .unwrap();
                pid = self.db.check_and_add_participant(
                    self.uid,
                    self.id,
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
                to_account = self.id;
                cid = self.db.check_and_add_category(
                    self.uid,
                    self.id,
                    "Deposit".to_ascii_uppercase(),
                );
                transacting_account_name = self
                    .db
                    .get_account_name(self.uid, transacting_account)
                    .unwrap();
                pid = self.db.check_and_add_participant(
                    self.uid,
                    self.id,
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
            ancillary_f32data: 0.0,
        };

        let (from_ledger_id, to_ledger_id) = match new_ttype {
            TransferType::WithdrawalToExternalAccount => (
                self.db
                    .add_ledger_entry(self.uid, self.id, linked_entry)
                    .unwrap(),
                entry.id,
            ),
            TransferType::DepositFromExternalAccount => (
                entry.id,
                self.db
                    .add_ledger_entry(self.uid, self.id, linked_entry)
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
            self.db
                .add_account_transaction(self.uid, transaction_record)
                .unwrap(),
        );
    }
}

impl AccountData for Wallet {
    fn get_id(&self) -> u32 {
        return self.id;
    }
    fn get_name(&self) -> String {
        return self.db.get_account_name(self.uid, self.id).unwrap();
    }
    fn get_ledger(&self) -> Vec<LedgerRecord> {
        return self.db.get_ledger(self.uid, self.id).unwrap();
    }
    fn get_ledger_within_dates(&self, start: NaiveDate, end: NaiveDate) -> Vec<LedgerRecord> {
        return self
            .db
            .get_ledger_entries_within_timestamps(self.uid, self.id, start, end)
            .unwrap();
    }
    fn get_displayable_ledger(&self) -> Vec<crate::types::ledger::DisplayableLedgerRecord> {
        return self.db.get_displayable_ledger(self.uid, self.id).unwrap();
    }

    fn get_value(&self) -> f32 {
        return self.fixed.get_current_value();
    }
    fn get_value_on_day(&self, day : NaiveDate) -> f32 {
        return self.fixed.get_value_on_day(day);
    }
    fn get_open_date(&self) -> NaiveDate {
        return self.open_date
    }
}

#[cfg (feature = "ratatui_support")]
impl Wallet { 
    fn render_spend_chart(&self, frame: &mut Frame, area: Rect, app: &App) {
        let (start, end) = (app.analysis_start, app.analysis_end);
        if let Some(mut entries)= self.db.get_expenditures_between_dates(self.uid, self.id, start, end).unwrap() {
            entries.sort_by(|x, y| { (x.amount).partial_cmp(&y.amount).unwrap_or(std::cmp::Ordering::Equal) });
            let grouped_others : Option<Expenditure> = if entries.len() > 10 {
                let misc = entries.drain(10..entries.len()-1).collect::<Vec<Expenditure>>();
                let amount = misc.into_iter().map(|x| x.amount).sum();
                Some(Expenditure { category : "Misc".to_string(), amount : amount })
            } else { 
                None
            };

            let mut data : Vec<(f64, f64)> = entries.iter().enumerate().map(|x| { ((x.0 as f64 / (entries.len()-1) as f64) * 100. as f64, x.1.amount as f64)}).collect::<Vec<(f64, f64)>>();
            data.sort_by(|x,y| { (x.1).partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal) });
            if grouped_others.is_some() { 
                let grouped = grouped_others.unwrap();
                data.push((1. + 1. / entries.len() as f64, grouped.amount as f64));
            }

            let max_amount = entries[entries.len()-1].amount;
            let max_range = (max_amount) as f64;
            let dataset = Dataset::default()
                .marker(symbols::Marker::HalfBlock)
                .style(Style::new().fg(tailwind::EMERALD.c500))
                .graph_type(GraphType::Bar)
                .data(&data);

            let chart = Chart::new(vec![dataset])
                .block(Block::bordered().title_top(Line::from("Spend Analyzer").cyan().bold().centered()))
                .style(Style::new().bg(tailwind::SLATE.c900))
                .x_axis(
                    Axis::default()
                        .style(Style::default().gray())
                        .bounds([0., 100.])
                        .labels(entries.iter().map(|x| { x.category.clone().drain(0..(if (x.category.len() > 10) { 10} else {x.category.len()})).collect::<String>() } ).collect::<Vec<String>>())
                        .labels_alignment(layout::Alignment::Right)
                )
                .y_axis(
                    Axis::default()
                        .style(Style::default().gray())
                        .bounds([0.0, max_range])
                        .labels(
                            float_range(0., max_range, (max_range) / 5.0)
                                .into_iter()
                                .map(|x| format!("{:.2}", x)),
                        ),
                )
                .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));
            frame.render_widget(chart, area);
        } else { 

            let value = ratatuiText::styled(
                "No data to display!",
                Style::default().fg(tailwind::ROSE.c400).bold(),
            );

            let display = Paragraph::new(value)
                .centered()
                .alignment(layout::Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Current Balance")
                        .title_alignment(layout::Alignment::Center),
                )
                .bg(tailwind::SLATE.c900);

            frame.render_widget(display, area);
        }
    }

}

#[cfg(feature = "ratatui_support")]
impl AccountUI for Wallet {
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

        self.render_current_value(frame, report_area, app);
        self.render_ledger_table(frame, chunk[1], app);
        self.render_spend_chart(frame, chart_area, app);
    }

    fn render_ledger_table(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        use ratatui::style::Modifier;

        let header_style = Style::default()
            .fg(app.ledger_table_colors.header_fg)
            .bg(app.ledger_table_colors.header_bg);

        let selected_row_style = Style::new()
            .add_modifier(Modifier::REVERSED)
            .fg(app.ledger_table_colors.selected_row_style_fg);

        let header = [
            "ID",
            "Date",
            "Type",
            "Amount",
            "Category",
            "Peer",
            "Description",
        ]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);

        let data = self.get_displayable_ledger();
        app.ledger_entries = Some(data.clone());

        let rows = data.iter().enumerate().map(|(i, record)| {
            let color = match i % 2 {
                0 => app.ledger_table_colors.normal_row_color,
                _ => app.ledger_table_colors.alt_row_color,
            };
            let item = [
                &record.id.to_string(),
                &record.info.date,
                &record.info.transfer_type,
                &record.info.amount.to_string(),
                &record.info.category,
                &record.info.participant.to_string(),
                &record.info.description,
            ];
            item.into_iter()
                .map(|content| Cell::from(ratatuiText::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(app.ledger_table_colors.row_fg).bg(color))
                .height(4)
        });

        let bar: &'static str = " █ ";
        let constraint_lens = ledger_table_constraint_len_calculator(&data);
        let t = Table::new(
            rows,
            [
                Constraint::Length(constraint_lens.0 + 1),
                Constraint::Min(constraint_lens.1 + 1),
                Constraint::Min(constraint_lens.2 + 1),
                Constraint::Min(constraint_lens.3 + 1),
                Constraint::Min(constraint_lens.4 + 1),
                Constraint::Min(constraint_lens.5 + 1),
                Constraint::Min(constraint_lens.6 + 1),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .highlight_symbol(ratatuiText::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .bg(app.ledger_table_colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(t, area, &mut app.ledger_table_state);
    }

 }

impl Account for Wallet {
    fn kind(&self) -> AccountType { 
        return AccountType::Wallet;
    }
    #[cfg(feature = "ratatui_support")]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
