use std::path::Path;
use core::f64;
use chrono::{NaiveDate, NaiveTime, Local, Days, Datelike};
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
        HighlightSpacing, List, ListItem, Padding, Paragraph, Row, Table, Tabs, Widget, Wrap,
        LegendPosition
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
use shared_lib::{LedgerEntry, FlatLedgerEntry};

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
use crate::types::investments::StockInfo;
use crate::types::investments::StockRecord;
use crate::types::investments::StockSplitInfo;
use crate::types::investments::StockSplitRecord;
use crate::types::ledger::LedgerInfo;
use crate::types::ledger::LedgerRecord;
use crate::types::participants::ParticipantType;
use csv::ReaderBuilder;
use rustyline::Editor;
use shared_lib::TransferType;

use super::base::variable_account::VariableAccount;
use super::base::Account;
use super::base::AccountCreation;
use super::base::AccountData;
use super::base::AccountOperations;
#[cfg(feature = "ratatui_support")]
use super::base::AccountUI;
#[cfg(feature = "ratatui_support")]
use crate::ui::{centered_rect, float_range};

pub struct InvestmentAccountManager {
    uid: u32,
    id: u32,
    db: DbConn,
    variable: VariableAccount,
    open_date : NaiveDate
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

impl AccountCreation for InvestmentAccountManager {
    fn create(uid: u32, name: String, _db: &DbConn) -> AccountRecord {
        let has_bank = true;
        let has_stocks = true;
        let has_ledger = false;
        let has_budget = false;

        let account: AccountInfo = AccountInfo {
            atype: AccountType::Investment,
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

impl InvestmentAccountManager {
    pub fn new(uid: u32, id: u32, db: &DbConn) -> Self {

        let mut ledger = db.get_ledger(uid, id).unwrap();
        let open_date = if !ledger.is_empty() { 
            ledger.sort_by(|l1, l2| (&l1.info.date).cmp(&l2.info.date));
            NaiveDate::parse_from_str(&ledger[0].info.date, "%Y-%m-%d").unwrap()
        } else { 
            Local::now().date_naive()
        };

        let acct = Self {
            uid: uid,
            id: id,
            db: db.clone(),
            variable: VariableAccount::new(uid, id, db, open_date),
            open_date : open_date,
        };

        acct
    }
}

impl AccountOperations for InvestmentAccountManager {
    fn record(&mut self) {
        const RECORD_OPTIONS: [&'static str; 6] = [
            "Deposit",
            "Withdrawal",
            "Purchase",
            "Sale",
            "Stock Split",
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
                "Deposit" => {
                    self.variable.fixed.deposit(None, false);
                }
                "Withdrawal" => {
                    self.variable.fixed.withdrawal(None, false);
                }
                "Purchase" => {
                    self.variable.purchase_stock(None, false);
                }
                "Sale" => {
                    self.variable.sell_stock(None, false);
                }
                "Stock Split" => {
                    self.variable.split_stock(None, false);
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

        for entry in ledger_entries {
            let ptype = if entry.transfer_type
                == shared_lib::TransferType::WithdrawalToExternalAccount
            {
                ParticipantType::Payee
            } else if entry.transfer_type == shared_lib::TransferType::WithdrawalToInternalAccount {
                ParticipantType::Payee
            } else if entry.transfer_type == shared_lib::TransferType::DepositFromExternalAccount {
                ParticipantType::Payer
            } else {
                ParticipantType::Payer
            };

            let lid: u32;
            let txn: LedgerInfo;
            if entry.stock_info.is_some() {
                let s: shared_lib::StockInfo = entry
                    .stock_info
                    .expect("Unable to obtain stock information!");

                if s.is_buy {
                    if s.is_split {
                        // if split, check that we own this symbol
                        let symbols_owned = self.db.get_stock_tickers(self.uid, self.id).unwrap();
                        let symbol_found = symbols_owned
                            .iter()
                            .any(|i| *i == entry.participant.clone());
                        if !symbol_found {
                            panic!("Attempting to register split of symbol not owned by account!");
                        }

                        txn = LedgerInfo {
                            date: entry.date,
                            amount: entry.amount,
                            transfer_type: entry.transfer_type as TransferType,
                            participant: self.db.check_and_add_participant(
                                self.uid,
                                self.id,
                                entry.participant.clone(),
                                ptype,
                                false,
                            ),
                            category_id: self.db.check_and_add_category(
                                self.uid,
                                self.id,
                                entry.category.to_ascii_uppercase(),
                            ),
                            description: entry.description,
                            ancillary_f32data: entry.ancillary_f32,
                        };

                        lid = self
                            .db
                            .add_ledger_entry(self.uid, self.id, txn.clone())
                            .unwrap();

                        // get total shares for ticker and divide by split
                        let stocks_owned = self
                            .db
                            .get_stocks(self.uid, self.id, entry.participant.clone())
                            .unwrap();
                        let all_shares: f32 = stocks_owned.iter().map(|x| x.info.remaining).sum();
                        // lpl takes the split and adds the difference to your account
                        // i.e., if the split is 3:1, it will take your 1 part and add 2 parts
                        let split_factor = (s.shares + all_shares) / all_shares;
                        let stock_split_id = self
                            .db
                            .add_stock_split(self.uid, self.id, split_factor.clone(), lid)
                            .unwrap();

                        let stock_split_record = StockSplitRecord {
                            id: stock_split_id,
                            info: StockSplitInfo {
                                split: split_factor,
                                ledger_id: lid,
                            },
                            txn_opt: Some(txn),
                        };

                        self.variable.allocate_stock_split(stock_split_record);
                    } else {
                        // if buy, confirm it is a valid ticker
                        let ticker_valid = self
                            .variable
                            .confirm_valid_ticker(entry.participant.clone());
                        if ticker_valid == false {
                            panic!("Stock symbol invalid!");
                        }

                        txn = LedgerInfo {
                            date: NaiveDate::parse_from_str(entry.date.as_str(), "%Y-%m-%d")
                                .unwrap()
                                .format("%Y-%m-%d")
                                .to_string(),
                            amount: entry.amount,
                            transfer_type: entry.transfer_type as TransferType,
                            participant: self.db.check_and_add_participant(
                                self.uid,
                                self.id,
                                entry.participant.clone(),
                                ptype,
                                false,
                            ),
                            category_id: self.db.check_and_add_category(
                                self.uid,
                                self.id,
                                entry.category.to_ascii_uppercase(),
                            ),
                            description: entry.description,
                            ancillary_f32data: entry.ancillary_f32,
                        };

                        lid = self.db.add_ledger_entry(self.uid, self.id, txn).unwrap();

                        let my_s: crate::types::investments::StockInfo = StockInfo {
                            shares: s.shares,
                            costbasis: s.costbasis,
                            remaining: s.remaining,
                            ledger_id: lid,
                        };

                        self.db.add_stock_purchase(self.uid, self.id, my_s).unwrap();
                    }
                } else {
                    // if sale, check that we own this symbol
                    let symbols_owned = self.db.get_stock_tickers(self.uid, self.id).unwrap();
                    let symbol_found = symbols_owned
                        .iter()
                        .any(|i| *i == entry.participant.clone());
                    if !symbol_found {
                        panic!("Attempting to register sale of symbol not owned by account!");
                    }

                    txn = LedgerInfo {
                        date: entry.date,
                        amount: entry.amount,
                        transfer_type: entry.transfer_type as TransferType,
                        participant: self.db.check_and_add_participant(
                            self.uid,
                            self.id,
                            entry.participant.clone(),
                            ptype,
                            false,
                        ),
                        category_id: self.db.check_and_add_category(
                            self.uid,
                            self.id,
                            entry.category.to_ascii_uppercase(),
                        ),
                        description: entry.description,
                        ancillary_f32data: entry.ancillary_f32,
                    };

                    lid = self
                        .db
                        .add_ledger_entry(self.uid, self.id, txn.clone())
                        .unwrap();

                    let my_s: crate::types::investments::StockInfo = StockInfo {
                        shares: s.shares,
                        costbasis: s.costbasis,
                        remaining: s.remaining,
                        ledger_id: lid,
                    };
                    let sale_id = self
                        .db
                        .add_stock_sale(self.uid, self.id, my_s.clone())
                        .unwrap();
                    self.variable.allocate_sale_stock(
                        StockRecord {
                            id: sale_id,
                            info: my_s,
                            txn_opt: Some(txn),
                        },
                        "LIFO".to_string(),
                    );
                }
            } else {
                // this is just a normal ledger transaction
                let txn: LedgerInfo = LedgerInfo {
                    date: NaiveDate::parse_from_str(entry.date.as_str(), "%Y-%m-%d")
                        .unwrap()
                        .format("%Y-%m-%d")
                        .to_string(),
                    amount: entry.amount,
                    transfer_type: entry.transfer_type as TransferType,
                    participant: self.db.check_and_add_participant(
                        self.uid,
                        self.id,
                        entry.participant,
                        ptype,
                        false,
                    ),
                    category_id: self.db.check_and_add_category(
                        self.uid,
                        self.id,
                        entry.category.to_ascii_uppercase(),
                    ),
                    description: entry.description,
                    ancillary_f32data: entry.ancillary_f32,
                };

                lid = self.db.add_ledger_entry(self.uid, self.id, txn).unwrap();
            }
        }
        self.variable.initialize_buffer();
    }

    fn modify(&mut self) {
        const MODIFY_OPTIONS: [&'static str; 4] = ["Ledger", "Categories", "Participant", "None"];
        let modify_choice =
            Select::new("\nWhat would you like to modify:", MODIFY_OPTIONS.to_vec())
                .prompt()
                .unwrap();
        match modify_choice {
            "Ledger" => {
                let record_or_none = self.variable.fixed.select_ledger_entry();
                if record_or_none.is_none() {
                    return;
                }
                let selected_record = record_or_none.unwrap();
                self.variable.modify(selected_record);
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
            "Participant" => {
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

    fn export(&self) {
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

        let mut wtr = csv::Writer::from_path(rl.readline("Enter path to CSV file: ").unwrap()).unwrap();
        let ledger = self.get_ledger();
        if !ledger.is_empty() {
            for record in ledger {
                let stock_record_opt = match record.info.transfer_type { 
                    TransferType::ZeroSumChange => { 
                        // this is a stock split
                        let stock_split_opt = self.db.check_and_get_stock_split_record_matching_from_ledger_id(self.uid, self.id, record.id).unwrap();
                        if let Some(ss_record) = stock_split_opt {
                            Some(shared_lib::StockInfo {
                                shares : 0.0, 
                                costbasis : 0.0, 
                                remaining : 0.0, 
                                is_buy : true, 
                                is_split : true
                            })
                        } else { 
                            None
                        }
                    }
                    TransferType::DepositFromInternalAccount => {
                        // this could either be a sale or a dividend, if dividend than expect to return none
                        let stock_sale_opt = self.db.check_and_get_stock_sale_record_matching_from_ledger_id(self.uid, self.id, record.id).unwrap();
                        if let Some(stock_sale) = stock_sale_opt { 
                            Some(shared_lib::StockInfo { 
                                shares: stock_sale.info.shares, 
                                costbasis: stock_sale.info.costbasis, 
                                remaining: 0.0, 
                                is_buy: false, 
                                is_split: false 
                            })
                        } else { 
                            None
                        }
                    }
                    TransferType::WithdrawalToInternalAccount => { 
                        // this is purchase
                        let purchase_opt = self.db.check_and_get_stock_purchase_record_matching_from_ledger_id(self.uid, self.id, self.id).unwrap();
                        if let Some(purchase) = purchase_opt { 
                            Some(shared_lib::StockInfo { 
                                shares: purchase.info.shares, 
                                costbasis: purchase.info.costbasis, 
                                remaining: 0.0, 
                                is_buy: false, 
                                is_split: false,
                            })
                        } else {
                            None
                        }
                    }
                    TransferType::DepositFromExternalAccount|TransferType::WithdrawalToExternalAccount => { 
                        None
                    }
                    
                };

                let csv_ledger_record : shared_lib::LedgerEntry = LedgerEntry { 
                    date: record.info.date, 
                    amount: record.info.amount,
                    transfer_type: record.info.transfer_type, 
                    participant: self.db.get_participant(self.uid, self.id, record.info.participant).unwrap(), 
                    category: self.db.get_category_name(self.uid, self.id, record.info.category_id).unwrap(), 
                    description: record.info.description, 
                    ancillary_f32: record.info.ancillary_f32data, 
                    stock_info: stock_record_opt 
                };
                let flattened = FlatLedgerEntry::from(csv_ledger_record);
                wtr.serialize(flattened).unwrap();
            }
        }
    }

    fn report(&self) {
        const REPORT_OPTIONS: [&'static str; 4] = [
            "Positions",
            "Total Value",
            "Time-Weighted Rate of Return",
            "None",
        ];
        let choice = Select::new("What would you like to report: ", REPORT_OPTIONS.to_vec())
            .prompt()
            .unwrap()
            .to_string();
        match choice.as_str() {
            "Positions" => {
                let positions_wrapped = self.variable.get_positions();
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
                let value = self.variable.get_current_value();
                println!("\tTotal Account Value: {}", value);
                println!(
                    "\t\tFixed Account Value: {}",
                    self.variable.fixed.get_current_value()
                );
                println!(
                    "\t\tVariable Account Value: {}",
                    self.variable
                        .db
                        .get_stock_current_value(self.uid, self.variable.id)
                        .unwrap()
                );
            }
            "Time-Weighted Rate of Return" => {
                let (period_start, period_end, _) = query_user_for_analysis_period(self.get_open_date());
                let twr = self.variable.time_weighted_return(period_start, period_end);
                println!("\tRate of return: {}%", twr);
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

impl AccountData for InvestmentAccountManager {
    fn get_id(&self) -> u32 {
        return self.id;
    }
    fn get_name(&self) -> String {
        return self.db.get_account_name(self.uid, self.id).unwrap();
    }
    fn get_ledger(&self) -> Vec<LedgerRecord> {
        let ledger = self.db.get_ledger(self.uid, self.id).unwrap();
        return ledger;
    }
    fn get_ledger_within_dates(&self, start: NaiveDate, end: NaiveDate) -> Vec<LedgerRecord> {
        let ledger =  self
            .db
            .get_ledger_entries_within_timestamps(self.uid, self.id, start, end)
            .unwrap();
        return ledger;
    }
    fn get_displayable_ledger(&self) -> Vec<crate::types::ledger::DisplayableLedgerRecord> {
        return self.db.get_displayable_ledger(self.uid, self.id).unwrap();
    }
    fn get_value(&self) -> f32 {
        return self.variable.get_current_value();
    }
    fn get_value_on_day(&self, day : NaiveDate) -> f32 {
        if let Some(value) = self.variable.get_account_value_on_day(&day) { 
            value
        } else { 
            0.0
        }
    }
    fn get_open_date(&self) -> NaiveDate {
        return self.open_date
    }
}

#[cfg(feature = "ratatui_support")]
impl InvestmentAccountManager {
    fn render_growth_chart(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        let (start, end) = (app.analysis_start, app.analysis_end);
        let mut ledger = self.get_ledger_within_dates(start, end);
        ledger.push(LedgerRecord { id : 0 , info : LedgerInfo { date: Local::now().date_naive().to_string(), amount: 0.0, transfer_type: TransferType::ZeroSumChange, participant: 0, category_id: 0, description: "".to_string(), ancillary_f32data: 0.0 }});
        let external_transfers = self.variable.db.get_external_transactions_between_timestamps(self.uid, self.id, start, end).unwrap();

        let mut tstamp_min =  f64::MAX;
        let mut tstamp_max = f64::MIN;
        let mut min_total = f64::MAX;
        let mut max_total = f64::MIN;
        
        // time period starting amount
        let time_period_investments_opt = if let Some(mut transactions) = external_transfers {
            if !transactions.is_empty() { 
                // this has to return a value because it will be inclusive of first entry
                let tpi_starting_amount = self.db.get_cumulative_total_of_ledger_of_external_transactions_on_date(self.uid, self.id, start).unwrap().unwrap();
                let initial = transactions.remove(0);
                let timestamp = NaiveDate::parse_from_str(&initial.info.date, "%Y-%m-%d").expect(format!("Unexpected data: {}", initial.info.date).as_str())
                    .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                    .and_utc()
                    .timestamp_millis() as f64;
                let mut aggregate = tpi_starting_amount as f64;
                let mut dataset = vec![(timestamp, aggregate)];
                transactions.push(LedgerRecord { id : 0 , info : LedgerInfo { date: Local::now().date_naive().to_string(), amount: 0.0, transfer_type: TransferType::ZeroSumChange, participant: 0, category_id: 0, description: "".to_string(), ancillary_f32data: 0.0 }});
                min_total = aggregate;
                max_total = aggregate;
                tstamp_min = timestamp;
                tstamp_max = tstamp_min;

                dataset.append(&mut
                    transactions.iter().map(|record| {
                        let date = NaiveDate::parse_from_str(&record.info.date, "%Y-%m-%d").unwrap();
                        let dt = date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                        let tstamp = dt.and_utc().timestamp_millis() as f64;
                        aggregate = match record.info.transfer_type {
                            TransferType::DepositFromExternalAccount => {
                                aggregate + record.info.amount as f64
                            }
                            TransferType::WithdrawalToExternalAccount => {
                                aggregate - record.info.amount as f64
                            }
                            _ => aggregate,
                        };
                        max_total = if aggregate > max_total {
                            aggregate
                        } else {
                            max_total
                        };
                        min_total = if aggregate < min_total {
                            aggregate
                        } else {
                            min_total
                        };
                        tstamp_max = if tstamp > tstamp_max {
                            tstamp
                        } else {
                            tstamp_max
                        };
                        (tstamp, aggregate)
                    }).collect()
                );
                Some(dataset)
            } else { 
                None
            }
        } else { 
            None
        };

        if let Some(time_period_investments) = time_period_investments_opt { 
            let mut datasets = vec![               
                Dataset::default()
                .name("Time Period Investment")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(tailwind::LIME.c400))
                .graph_type(GraphType::Line)
                .data(&time_period_investments)];

                
            let mut date = start;
            let mut total_account_values = Vec::new();
            while date < end { 
                let value = self.variable.get_account_value_on_day(&date.clone());
                if value.is_none() { 
                    break;
                } else {
                    use crate::accounts::base::AnalysisPeriod;

                    let tstamp = NaiveDate::parse_from_str(&date.to_string(), "%Y-%m-%d").expect(format!("Unexpected data: {}", date).as_str())
                    .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                    .and_utc()
                    .timestamp_millis() as f64;

                    let partial_value = self.variable.get_account_value_on_day(&date);
                    let mut aggregate = 0.0;
                    if partial_value.is_none() { 
                        aggregate = aggregate;
                    } else {
                        aggregate = partial_value.unwrap() as f64;
                    }
                    max_total = if aggregate > max_total {
                        aggregate
                    } else {
                        max_total
                    };
                    min_total = if aggregate < min_total {
                        aggregate
                    } else {
                        min_total
                    };
                    tstamp_max = if tstamp > tstamp_max {
                        tstamp
                    } else {
                        tstamp_max
                    };
                    total_account_values.push((tstamp, aggregate));


                    date = match app.analysis_period {
                        AnalysisPeriod::OneDay|AnalysisPeriod::OneWeek => { 
                            date.checked_add_days(Days::new(1)).unwrap()
                        }
                        AnalysisPeriod::OneMonth => { 
                            date.checked_add_days(Days::new(2)).unwrap()
                        }
                        AnalysisPeriod::OneYear|AnalysisPeriod::ThreeMonths|AnalysisPeriod::SixMonths|AnalysisPeriod::YTD => { 
                            date.checked_add_days(Days::new(7)).unwrap()
                        }
                        AnalysisPeriod::TwoYears => { 
                            date.checked_add_days(Days::new(20)).unwrap()
                        }
                        AnalysisPeriod::FiveYears => { 
                            date.checked_add_days(Days::new(50)).unwrap()
                        }
                        AnalysisPeriod::TenYears => { 
                            date.checked_add_days(Days::new(100)).unwrap()
                        }
                        AnalysisPeriod::Custom|AnalysisPeriod::AllTime => {
                            let diff = (end.num_days_from_ce() - start.num_days_from_ce()) as u32;
                            let days_to_add: u32 = if diff <= 365 {
                                1
                            } else if diff <= (365 * 2) {
                                2
                            } else if diff <= (365 * 5) { 
                                5
                            } else { 
                                10
                            };
                            date.checked_add_days(Days::new(days_to_add as u64)).unwrap()
                        }
                    };
                    // date = date.checked_add_days(Days::new(10)).unwrap();
                }
            }

            datasets.push(                
                Dataset::default()
                .name("Total Value")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(tailwind::BLUE.c400))
                .graph_type(GraphType::Line)
                .data(&total_account_values)
            );

            let chart = Chart::new(datasets)
                .block(
                    Block::bordered().title(Line::from(" Value Over Time ").cyan().bold().centered()),
                )
                .legend_position(Some(LegendPosition::TopLeft))
                .x_axis(
                    Axis::default()
                        .title("Time")
                        .style(Style::default().gray())
                        .bounds([tstamp_min, tstamp_max])
                        .labels([start.to_string(), end.to_string()]),
                )
                .y_axis(
                    Axis::default()
                        .title("Value (💰)")
                        .style(Style::default().gray())
                        .bounds([min_total, max_total])
                        .labels(
                            float_range(min_total, max_total, (max_total - min_total) / 5.0)
                                .into_iter()
                                .map(|x| format!("{:.2}", x)),
                        ),
                );

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
                        .title_alignment(layout::Alignment::Center)
                        .padding(Padding::new(0,0, area.height/2-2, 0)),
                )
                .bg(tailwind::SLATE.c900);

            frame.render_widget(display, area);
        }
    }

    fn render_time_weighted_rate_of_return(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        let (start, end) = (app.analysis_start, app.analysis_end);
        let value = self.variable.time_weighted_return(start, end);
        let fg_color = if value < 0.0 {
            tailwind::ROSE.c200
        } else {
            tailwind::EMERALD.c400
        };
        let value = ratatuiText::styled(
            format!("{:.2}%", value).to_string(),
            Style::default().fg(fg_color).bold(),
        );

        let display = Paragraph::new(value)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Time-Weighted Rate of Return - {} ", app.analysis_period))
                    .title_alignment(layout::Alignment::Center)
                    .padding(Padding::new(0,0, area.height/2-2, 0)),
            )
            .bg(tailwind::SLATE.c900);
        // let centered_area = centered_rect(10, 10, area);
        // let growth = Paragraph::new(value);
        // frame.render_widget(growth, centered_area);
        frame.render_widget(display, area);
    }

}

#[cfg(feature = "ratatui_support")]
impl AccountUI for InvestmentAccountManager {
    fn render(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        let chunk = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let data_area = chunk[0];
        let ledger_area = chunk[1];

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
        
        let value_area = reports_chunks[0];
        let twrr_area = reports_chunks[1];

        self.render_ledger_table(frame, ledger_area, app);
        self.render_growth_chart(frame, graph_area, app);
        self.render_current_value(frame, value_area, app);
        self.render_time_weighted_rate_of_return(frame, twrr_area, app);
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

impl Account for InvestmentAccountManager {
    fn kind(&self) -> AccountType { 
        return AccountType::Investment;
    }
    #[cfg(feature = "ratatui_support")]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
