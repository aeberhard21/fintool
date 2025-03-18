use std::path::Path;
use inquire::Confirm;
use inquire::Select;
use inquire::Text;
use rustyline::completion::Candidate;
use rustyline::completion::FilenameCompleter;
use rustyline::highlight::Highlighter;
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
use csv::ReaderBuilder;
use rustyline::{Editor, Result};
use shared_lib::LedgerEntry;

use crate::database::DbConn;
use crate::tui::query_user_for_analysis_period;
use crate::types::accounts::AccountInfo;
use crate::types::accounts::AccountTransaction;
use crate::types::accounts::AccountType;
use crate::types::ledger::LedgerInfo;
use crate::types::ledger::LedgerRecord;
use crate::types::participants::ParticipantType;
use shared_lib::TransferType;

use super::base::fixed_account::FixedAccount;
use super::base::AccountCreation;
use super::base::AccountOperations;

pub struct BankAccount {
    id: u32,
    db: DbConn,
    fixed: FixedAccount,
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

impl BankAccount {
    pub fn new(uid: u32, id: u32, db: &mut DbConn) -> Self {
        let mut acct: BankAccount = Self {
            id: id,
            db: db.clone(),
            fixed: FixedAccount::new(uid, id, db.clone()),
        };
        // acct.db.add_participant(id, ParticipantType::Payee, "Fixed".to_string());
        acct
    }
}

impl AccountCreation for BankAccount {
    fn create() -> AccountInfo {
        let mut name: String = String::new();
        loop {
            name = Text::new("Enter account name:")
                .prompt()
                .unwrap()
                .to_string();
            if name.len() == 0 {
                println!("Invalid account name!")
            } else {
                break;
            }
        }
        let mut has_bank = true;
        let mut has_stocks = false;
        let mut has_ledger = false;
        let mut has_budget = false;

        let account: AccountInfo = AccountInfo {
            atype: AccountType::Bank,
            name: name,
            has_stocks: has_stocks,
            has_bank: has_bank,
            has_ledger: has_ledger,
            has_budget: has_budget,
        };

        return account;
    }
}

impl AccountOperations for BankAccount {

    fn record(&mut self) {
        loop {
            let action = Select::new(
                "\nWhat transaction would you like to record?",
                vec!["Deposit", "Withdrawal"],
            )
            .prompt()
            .unwrap()
            .to_string();
            match action.as_str() {
                "Deposit" => {
                    self.fixed.deposit();
                }
                "Withdrawal" => {
                    self.fixed.withdrawal();
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
        let csv = rl.readline("Enter path to CSV file: ").unwrap();

        let mut fp = Path::new("~");
        println!("{}", Path::new(&csv).display());
        match Path::new(&csv).try_exists() {
            Ok(true) => {
                fp = Path::new(&csv);
            }
            Ok(false) => {
                println!("cannot be found!");
            }
            Err(e) => {
                println!("error is: {}", e);
            }
        };

        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_path(fp)
            .unwrap();

        for record in rdr.deserialize::<LedgerEntry>() { 
            let rcrd = record.unwrap();
            let ptype = 
                if rcrd.transfer_type == TransferType::WithdrawalToExternalAccount { ParticipantType::Payee } 
                else if rcrd.transfer_type == TransferType::WithdrawalToInternalAccount { ParticipantType::Payee } 
                else if rcrd.transfer_type == TransferType::DepositFromExternalAccount { ParticipantType::Payer } 
                else { ParticipantType::Payer };
            let entry : LedgerInfo = LedgerInfo { 
                date : rcrd.date,
                amount : rcrd.amount,
                transfer_type : rcrd.transfer_type as TransferType,
                participant : self.db.check_and_add_participant(self.id, rcrd.participant, ptype),
                category_id : self.db.check_and_add_category(self.id, rcrd.category),
                description : rcrd.description
            };
            let lid: u32 = self.db.add_ledger_entry(self.id, entry).unwrap();
        }
    }

    fn modify(&mut self) {}

    fn export(&mut self) {}

    fn report(&mut self) {
        const REPORT_OPTIONS: [&'static str; 2] = ["Total Value", "Simple Growth Rate"];
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
                let (period_start, period_end) = query_user_for_analysis_period();
                let rate = self.fixed.simple_rate_of_return(period_start, period_end);
                println!("\tRate of return: {}%", rate);
            }
            _ => {
                panic!("Unrecognized input!");
            }
        }
    }

    fn link(&mut self, transacting_account: u32, entry: LedgerRecord) -> Option<u32> {
        let mut my_entry = entry.clone();
        let mut from_account;
        let mut to_account;

        match my_entry.info.transfer_type {
            TransferType::DepositFromExternalAccount => {
                my_entry.info.transfer_type = TransferType::WithdrawalToExternalAccount;
                from_account = self.id;
                to_account = transacting_account;
            }
            TransferType::WithdrawalToExternalAccount => {
                my_entry.info.transfer_type = TransferType::DepositFromExternalAccount;
                from_account = transacting_account;
                to_account = self.id;
            }
            _ => {
                return None;
            }
        }

        let transaction_record = AccountTransaction {
            from_account: from_account,
            to_account: to_account,
            from_ledger: entry.id,
            to_ledger: self.db.add_ledger_entry(self.id, my_entry.info).unwrap(),
        };

        return Some(self.db.add_account_transaction(transaction_record).unwrap());
    }
}
