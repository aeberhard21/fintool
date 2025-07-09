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
use std::env::current_exe;
use std::hash::Hash;
use std::path::Path;
use std::rc;

#[cfg(feature = "ratatui_support")]
use crate::app::app::App;
#[cfg(feature = "ratatui_support")]
use crate::app::screen::ledger_table_constraint_len_calculator;
use crate::database::DbConn;
use crate::tui::query_user_for_analysis_period;
use crate::{tui::get_analysis_period_dates, types::credit_card::CreditCardExpense};
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
use shared_lib::TransferType;

use super::base::charge_account::ChargeAccount;
use super::base::Account;
use super::base::AccountCreation;
use super::base::AccountData;
use super::base::AccountOperations;
#[cfg(feature = "ratatui_support")]
use super::base::AccountUI;
use crate::accounts::float_range;

pub struct CreditCardAccount {
    uid: u32,
    id: u32,
    db: DbConn,
    charge: ChargeAccount,
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

impl CreditCardAccount {
    pub fn new(uid: u32, id: u32, db: &DbConn) -> Self {
        let acct: CreditCardAccount = Self {
            uid: uid,
            id: id,
            db: db.clone(),
            charge: ChargeAccount::new(uid, id, db.clone()),
        };
        acct
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

        return AccountRecord {
            id: aid,
            info: account,
        };
    }
}

impl AccountOperations for CreditCardAccount {
    fn record(&self) {
        const RECORD_OPTIONS: [&'static str; 3] = ["Charge", "Payment", "None"];
        loop {
            let action = Select::new(
                "\nWhat transaction would you like to record?",
                RECORD_OPTIONS.to_vec(),
            )
            .prompt()
            .unwrap()
            .to_string();
            match action.as_str() {
                "Payment" => {
                    self.charge.pay(None, false);
                }
                "Charge" => {
                    self.charge.charge(None, false);
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

    fn import(&self) {
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

    fn modify(&self) {
        const MODIFY_OPTIONS: [&'static str; 6] = [
            "Ledger",
            "Credit Line",
            "Statement Due Date",
            "Categories",
            "People",
            "None",
        ];
        let modify_choice =
            Select::new("\nWhat would you like to modify:", MODIFY_OPTIONS.to_vec())
                .prompt()
                .unwrap();
        match modify_choice {
            "Ledger" => {
                let record_or_none = self.charge.select_ledger_entry();
                if record_or_none.is_none() {
                    return;
                }
                let selected_record = record_or_none.unwrap();
                self.charge.modify(selected_record);
            }
            "Credit Line" => {
                let credit_card = self.db.get_credit_card(self.uid, self.id).unwrap();
                let updated_credit_line = CustomType::<f32>::new("Enter updated credit line:")
                    .with_default(credit_card.info.credit_line)
                    .with_placeholder("1000.00")
                    .with_error_message("Enter a valid credit line!")
                    .prompt()
                    .unwrap();
                self.db
                    .update_credit_line(self.uid, self.id, updated_credit_line)
                    .unwrap();
            }
            "Statement Due Date" => {
                let credit_card = self.db.get_credit_card(self.uid, self.id).unwrap();
                let updated_statement_due_date =
                    CustomType::<u32>::new("Enter updated statement due date:")
                        .with_default(credit_card.info.statement_due_date)
                        .with_placeholder("1")
                        .with_error_message("Enter a statement due date!")
                        .prompt()
                        .unwrap();
                self.db
                    .update_statement_due_date(self.uid, self.id, updated_statement_due_date)
                    .unwrap();
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
                let value = self.charge.get_current_balance();
                println!("\tCurrent Balance: {}", value);
            }
            "Credit Line" => {
                println!("\tCredit Line: {}", self.charge.get_credit_line());
            }
            "Remaining Credit" => {
                println!(
                    "\tRemaining credit: {}",
                    self.charge.get_remaining_in_credit_line()
                );
            }
            "Spend Analyzer" => {
                let (start, end) = query_user_for_analysis_period(self);
                let expenses_wrapped = self
                    .charge
                    .db
                    .get_credit_expenditures_between_dates(self.uid, self.id, start, end)
                    .unwrap();
                if expenses_wrapped.is_some() {
                    let mut expenses = expenses_wrapped.unwrap();
                    expenses.sort_by(|x, y| { (x.amount).partial_cmp(&y.amount).unwrap_or(std::cmp::Ordering::Equal) });
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

impl AccountData for CreditCardAccount {
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
        return self.charge.get_current_balance();
    }
}

#[cfg(feature = "ratatui_support")]
impl AccountUI for CreditCardAccount {
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
            .constraints([Constraint::Percentage(33), Constraint::Percentage(34), Constraint::Percentage(33)])
            .split(graphs_reports[0]);

        let value_area = report_chunks[0];
        let due_date = report_chunks[1];

        self.render_ledger_table(frame, chunk[1], app);
        self.render_current_value(frame, report_chunks[0], app);
        self.render_remaining_credit(frame, report_chunks[1], app);
        self.render_days_until_due_date(frame, report_chunks[2], app);
        self.render_spend_chart(frame, graphs_reports[1], app);
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

    fn render_current_value(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        let value = ratatuiText::styled(
            self.get_value().to_string(),
            Style::default().fg(tailwind::EMERALD.c400).bold(),
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

#[cfg(feature = "ratatui_support")]
impl CreditCardAccount {
    fn get_statement_due_date(&self) -> NaiveDate {
        use chrono::Datelike;

        let credit_card = self.db.get_credit_card(self.uid, self.id).unwrap();
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

    fn get_days_until_due_date(&self) -> u32 {
        use chrono::Datelike;
        let due_date = self.get_statement_due_date();
        let today = Local::now().date_naive();
        return due_date.num_days_from_ce() as u32 - today.num_days_from_ce() as u32;
    }

    fn render_days_until_due_date(&self, frame: &mut Frame, area: Rect, app: &App) {
        let days_to = self.get_days_until_due_date();
        let statement_date = self.get_statement_due_date().to_string();
        let days_to_text = vec![
            Span::styled(
                format!("{} {}", days_to, if days_to > 1 { "days" } else { "day" }),
                Style::default().bold().fg(if days_to < 5 {
                    tailwind::ROSE.c100
                } else if days_to < 15 {
                    tailwind::ROSE.c200
                } else {
                    tailwind::EMERALD.c400
                }),
            ),
            Span::styled(
                format!(" until {}", statement_date),
                Style::default().bold().fg(tailwind::EMERALD.c400),
            ),
        ];
        let line = Line::from(days_to_text);
        let text = ratatuiText::from(line);
        let p = Paragraph::new(text)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Statement Due Date Countdown")
                    .title_alignment(layout::Alignment::Center),
            )
            .bg(tailwind::SLATE.c900);
        frame.render_widget(p, area);
    }

    fn render_remaining_credit(&self, frame: &mut Frame, area: Rect, app: &App) {
        let credit_remaining = self.charge.get_remaining_in_credit_line();
        let credit_line = self.charge.get_credit_line();
        let days_to_text = vec![
            Span::styled(
                format!("${:.2}", credit_remaining),
                Style::default().bold().fg(if credit_remaining < 500. {
                    tailwind::ROSE.c100
                } else if credit_remaining < 100. {
                    tailwind::ROSE.c200
                } else {
                    tailwind::EMERALD.c400
                }),
            ),
            Span::styled(
                format!(" of ${:.2} remaining.", credit_line),
                Style::default().bold().fg(tailwind::EMERALD.c400),
            ),
        ];
        let line = Line::from(days_to_text);
        let text = ratatuiText::from(line);
        let p = Paragraph::new(text)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Statement Due Date Countdown")
                    .title_alignment(layout::Alignment::Center),
            )
            .bg(tailwind::SLATE.c900);
        frame.render_widget(p, area);
    }


    fn render_spend_chart(&self, frame: &mut Frame, area: Rect, app: &App) {
        let (start, end) = get_analysis_period_dates(self, app.analysis_period.clone());
        if let Some(mut entries ) = self.charge.db.get_credit_expenditures_between_dates(self.uid, self.id, start, end).unwrap() { 
            entries.sort_by(|x, y| { (x.amount).partial_cmp(&y.amount).unwrap_or(std::cmp::Ordering::Equal) });
            let grouped_others : Option<CreditCardExpense> = if entries.len() > 10 {
                let misc = entries.drain(10..entries.len()-1).collect::<Vec<CreditCardExpense>>();
                let amount = misc.into_iter().map(|x| x.amount).sum();
                Some(CreditCardExpense { category : "Misc".to_string(), amount : amount })
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

impl Account for CreditCardAccount {}
