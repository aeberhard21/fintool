use std::path::Path;

use inquire::Confirm;
use inquire::Select;
use inquire::Text;
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
use shared_lib::LedgerEntry;

use crate::database::DbConn;
use crate::tui::query_user_for_analysis_period;
use crate::types::accounts::AccountInfo;
use crate::types::accounts::AccountTransaction;
use crate::types::accounts::AccountType;
use crate::types::investments::StockInfo;
use crate::types::investments::StockSplitInfo;
use crate::types::investments::StockSplitRecord;
use crate::types::ledger::LedgerInfo;
use crate::types::ledger::LedgerRecord;
use crate::types::participants::ParticipantType;
use csv::ReaderBuilder;
use rustyline::Editor;
use shared_lib::TransferType;

use super::base::variable_account::VariableAccount;
use super::base::AccountCreation;
use super::base::AccountOperations;

pub struct InvestmentAccountManager {
    uid: u32,
    id: u32,
    db: DbConn,
    variable: VariableAccount,
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
    fn create() -> AccountInfo {
        let mut name: String = String::new();
        loop {
            name = Text::new("Enter investment account name:")
                .prompt()
                .unwrap()
                .to_string();
            if name.len() == 0 {
                println!("Invalid account name!")
            } else {
                break;
            }
        }
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

        return account;
    }
}

impl InvestmentAccountManager {
    pub fn new(uid: u32, id: u32, db: &mut DbConn) -> Self {
        let acct = Self {
            uid: uid,
            id: id,
            db: db.clone(),
            variable: VariableAccount::new(uid, id, db),
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
                    self.variable.fixed.deposit();
                }
                "Withdrawal" => {
                    self.variable.fixed.withdrawal();
                }
                "Purchase" => {
                    self.variable.purchase_stock(None, false);
                }
                "Sale" => {
                    self.variable.sell_stock(None,false);
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
                Ok(true) => {
                    false
                }
                Ok(false) => {
                    println!("File {} cannot be found!", Path::new(&csv).display());
                    true
                }
                Err(e) => {
                    println!("File {} cannot be found: {}!", e, Path::new(&csv).display());
                    true
                }
            };
            if !bad_path { break; } 
            else { 
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

        for result in rdr.deserialize::<LedgerEntry>() {
            // println!("{}", result.unwrap().amount);
            let entry = result.unwrap();

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

            let lid : u32;
            let txn : LedgerInfo;
            if entry.stock_info.is_some() {

                let s: shared_lib::StockInfo = entry.stock_info.expect("Unable to obtain stock information!");

                if s.is_buy {
                    if s.is_split {
                        // if split, check that we own this symbol
                        let symbols_owned = self.db.get_stock_tickers(self.uid, self.id).unwrap();
                        let symbol_found = symbols_owned.iter().any(|i| *i == entry.participant.clone());
                        if !symbol_found {
                            panic!("Attempting to register split of symbol not owned by account!");
                        }

                        txn = LedgerInfo {
                            date: entry.date,
                            amount: entry.amount,
                            transfer_type: entry.transfer_type as TransferType,
                            participant: self
                                .db
                                .check_and_add_participant(self.uid, self.id, entry.participant.clone(), ptype, false),
                            category_id: self.db.check_and_add_category(self.uid, self.id, entry.category.to_ascii_uppercase()),
                            description: entry.description,
                            ancillary_f32data : entry.ancillary_f32
                        };

                        lid = self.db.add_ledger_entry(self.uid, self.id, txn.clone()).unwrap();

                        // get total shares for ticker and divide by split
                        let stocks_owned =
                            self.db.get_stocks(self.uid, self.id, entry.participant.clone()).unwrap();
                        let all_shares: f32 = stocks_owned.iter().map(|x| x.info.shares).sum();
                        let split_factor = s.shares / all_shares;
                        let stock_split_id = self.db
                            .add_stock_split(self.uid, self.id, split_factor.clone(), lid)
                            .unwrap();

                        let stock_split_record = StockSplitRecord { 
                            id : stock_split_id, 
                            info : StockSplitInfo { 
                                split : split_factor,
                                ledger_id : lid
                            },
                            txn_opt : Some(txn)
                        };

                        self.variable.allocate_stock_split(stock_split_record);
                    } else {
                        // if buy, confirm it is a valid ticker
                        let ticker_valid = self.variable.confirm_valid_ticker(entry.participant.clone());
                        if ticker_valid == false { 
                            panic!("Stock symbol invalid!");
                        }

                        txn = LedgerInfo {
                            date: entry.date,
                            amount: entry.amount,
                            transfer_type: entry.transfer_type as TransferType,
                            participant: self
                                .db
                                .check_and_add_participant(self.uid,self.id, entry.participant.clone(), ptype, false),
                            category_id: self.db.check_and_add_category(self.uid, self.id, entry.category.to_ascii_uppercase()),
                            description: entry.description,
                            ancillary_f32data : entry.ancillary_f32
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
                    // if split, check that we own this symbol
                    let symbols_owned = self.db.get_stock_tickers(self.uid, self.id).unwrap();
                    let symbol_found = symbols_owned.iter().any(|i| *i == entry.participant.clone());
                    if !symbol_found {
                        panic!("Attempting to register sale of symbol not owned by account!");
                    }

                    txn = LedgerInfo {
                        date: entry.date,
                        amount: entry.amount,
                        transfer_type: entry.transfer_type as TransferType,
                        participant: self
                            .db
                            .check_and_add_participant(self.uid, self.id, entry.participant.clone(), ptype, false),
                        category_id: self.db.check_and_add_category(self.uid, self.id, entry.category.to_ascii_uppercase()),
                        description: entry.description,
                        ancillary_f32data : entry.ancillary_f32
                    };

                    lid = self.db.add_ledger_entry(self.uid, self.id, txn).unwrap();

                    let my_s: crate::types::investments::StockInfo = StockInfo {
                        shares: s.shares,
                        costbasis: s.costbasis,
                        remaining: s.remaining,
                        ledger_id: lid,
                    };
                    self.db.add_stock_sale(self.uid,self.id, my_s).unwrap();
                }
            } else {
                // this is just a normal ledger transaction
                let txn: LedgerInfo = LedgerInfo {
                    date: entry.date,
                    amount: entry.amount,
                    transfer_type: entry.transfer_type as TransferType,
                    participant: self
                        .db
                        .check_and_add_participant(self.uid, self.id, entry.participant, ptype, false),
                    category_id: self.db.check_and_add_category(self.uid, self.id, entry.category.to_ascii_uppercase()),
                    description: entry.description,
                    ancillary_f32data : entry.ancillary_f32
                };
    
                lid = self.db.add_ledger_entry(self.uid, self.id, txn).unwrap();
            }
        }
    }

    fn modify(&mut self) {
        let record_or_none = self.variable.fixed.select_ledger_entry();
        if record_or_none.is_none() {
            return;
        }
        let selected_record = record_or_none.unwrap();
        self.variable.modify(selected_record);
    }

    fn export(&mut self) {}

    fn report(&mut self) {
        const REPORT_OPTIONS: [&'static str; 3] =
            ["Total Value", "Time-Weighted Rate of Return", "None"];
        let choice = Select::new("What would you like to report: ", REPORT_OPTIONS.to_vec())
            .prompt()
            .unwrap()
            .to_string();
        match choice.as_str() {
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
                let (period_start, period_end) = query_user_for_analysis_period();
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

    fn link(&mut self, transacting_account: u32, entry: LedgerRecord) -> Option<u32> {
        let from_account;
        let to_account;

        let cid;
        let pid;
        let transacting_account_name : String;
        let (new_ttype, description) = match entry.info.transfer_type {
            TransferType::DepositFromExternalAccount => {
                from_account = self.id;
                to_account = transacting_account;
                cid =self.db.check_and_add_category(self.uid,self.id, "Withdrawal".to_ascii_uppercase());
                transacting_account_name = self.db.get_account_name(self.uid, transacting_account).unwrap();
                pid =self.db.check_and_add_category(self.uid, self.id, transacting_account_name.clone());
                (
                    TransferType::WithdrawalToExternalAccount,
                    format!("[Link]: Withdrawal of ${} to account {} on {}.", entry.info.amount, transacting_account_name, entry.info.date)
                )
            }
            TransferType::WithdrawalToExternalAccount => {
                from_account = transacting_account;
                to_account = self.id;
                cid =self.db.check_and_add_category(self.uid,self.id, "Deposit".to_ascii_uppercase());
                transacting_account_name = self.db.get_account_name(self.uid, transacting_account).unwrap();
                pid =self.db.check_and_add_category(self.uid, self.id, transacting_account_name.clone());
                (
                    TransferType::DepositFromExternalAccount,
                    format!("[Link]: Deposit of ${} from account {} on {}.", entry.info.amount, transacting_account_name, entry.info.date)
                )
            }
            _ => {
                return None;
            }
        };

        let linked_entry = LedgerInfo { 
            date : entry.info.date,
            amount : entry.info.amount,
            transfer_type : new_ttype, 
            participant : pid, 
            category_id: cid, 
            description : description,
            ancillary_f32data : 0.0
        };

        let transaction_record = AccountTransaction {
            from_account: from_account,
            to_account: to_account,
            from_ledger: entry.id,
            to_ledger: self.db.add_ledger_entry(self.uid, self.id, linked_entry).unwrap(),
        };

        return Some(self.db.add_account_transaction(self.uid, transaction_record).unwrap());
    }
}
