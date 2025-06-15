use chrono::Datelike;
use chrono::NaiveDate;
use chrono::Months;
use csv::ReaderBuilder;
use inquire::Confirm;
use inquire::CustomType;
use inquire::DateSelect;
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
use rustyline::Editor;
use rustyline::Helper;
use rustyline::Highlighter;
use rustyline::Hinter;
use rustyline::Validator;
use shared_lib::LedgerEntry;
use std::path::Path;

use crate::database::DbConn;
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
use shared_lib::TransferType;

use super::base::fixed_account::FixedAccount;
use super::base::AccountCreation;
use super::base::AccountOperations;
use super::base::AccountData;
use super::base::Account;

pub struct CertificateOfDepositAccount {
    uid : u32, 
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

impl CertificateOfDepositAccount {
    pub fn new(uid: u32, id: u32, db: &mut DbConn) -> Self {
        let acct: CertificateOfDepositAccount = Self {
            uid: uid,
            id: id,
            db: db.clone(),
            fixed: FixedAccount::new(uid, id, db.clone()),
        };
        acct
    }
}

impl AccountCreation for CertificateOfDepositAccount {
    fn create(uid : u32, name: String, _db : &mut DbConn) -> AccountRecord {

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

        let open_date = DateSelect::new("Enter open date:")
            .prompt()
            .unwrap();

        let length = CustomType::<u32>::new("Enter length (in months) to maturity:")
            .with_placeholder("12")
            .with_default(12)
            .with_error_message("Please type a valid number!")
            .prompt()
            .unwrap();

        let maturity_date = open_date.checked_add_months(Months::new(length)).unwrap();

        let cd_info = CertificateOfDepositInfo { 
            apy : apy, 
            principal : principal, 
            maturity_date : maturity_date.format("%Y-%m-%d").to_string(),
            length_months : length
        };

        _db.add_certificate_of_deposit(uid, aid, cd_info.clone()).unwrap();

        let initialize_ledger = Confirm::new("Initialize ledger with principal?")
            .prompt()
            .unwrap();

        if initialize_ledger { 
            let link = Confirm::new("Link transaction to another account?")
                .prompt()
                .unwrap();
            let input = if link { 
                cd.fixed.link_transaction(None)
            } else { None } ;

            let peer = if input.is_none() {
                let payer = Text::new("Enter payer:")
                    .prompt()
                    .unwrap();
                (None, payer)
            } else { 
                let (acct, account_name) = input.unwrap();
                (Some(acct), account_name)
            };

            let initial = crate::types::ledger::LedgerInfo { 
                date : open_date.format("%Y-%m-%d").to_string(),
                amount : principal, 
                transfer_type : TransferType::DepositFromExternalAccount, 
                participant : _db.check_and_add_participant(uid, aid, peer.1, ParticipantType::Payer, peer.0.is_some()),
                category_id : _db.check_and_add_category(uid, aid,"Deposit".to_ascii_uppercase().to_string()),
                description : format!("Open {} APY {} month CD with ${}", cd_info.apy, cd_info.length_months, cd_info.principal),
                ancillary_f32data : 0.0,
            };

            let lid = _db.add_ledger_entry(uid, aid, initial.clone()).unwrap();
            if peer.0.is_some() { 
                peer.0.unwrap().link(cd.id, LedgerRecord { id: lid, info : initial } );
            }
        }

        return AccountRecord { id : aid, info : account };
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

        let mut ledger_entries = Vec::new();
        for result in rdr.deserialize::<LedgerEntry>() {
            ledger_entries.push(result.unwrap());
        };
        ledger_entries.sort_by(
            |x,y| { 
                (NaiveDate::parse_from_str(&x.date, "%Y-%m-%d").unwrap())
                .cmp(&NaiveDate::parse_from_str(&y.date, "%Y-%m-%d").unwrap())
            }
        );
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
                date: NaiveDate::parse_from_str( rcrd.date.as_str(),"%Y-%m-%d").unwrap().format("%Y-%m-%d").to_string(),
                amount: rcrd.amount,
                transfer_type: rcrd.transfer_type as TransferType,
                participant: self
                    .db
                    .check_and_add_participant(self.uid, self.id, rcrd.participant, ptype, false),
                category_id: self.db.check_and_add_category(self.uid, self.id, rcrd.category.to_ascii_uppercase()),
                description: rcrd.description,
                ancillary_f32data : 0.0
            };
            let _lid: u32 = self.db.add_ledger_entry(self.uid, self.id, entry).unwrap();
        }
    }

    fn modify(&mut self) {
        const MODIFY_OPTIONS: [&'static str; 7] = ["APY", "Ledger", "Length", "Categories", "People", "Principal", "None"];
        let modify_choice = Select::new("\nWhat would you like to modify:", MODIFY_OPTIONS.to_vec())
        .prompt()
        .unwrap();
        match modify_choice { 
            "APY" => { 
                let cd = self.db.get_certificate_of_deposit(self.uid, self.id).unwrap();
                let updated_apy = CustomType::<f32>::new("Enter annual percentage yield:")
                    .with_placeholder("3.00")
                    .with_default(cd.info.apy)
                    .with_error_message("Please type a valid percentage!")
                    .prompt()
                    .unwrap();
                self.db.update_cd_apy(self.uid, self.id, updated_apy).unwrap();
            }
            "Ledger" => {
                let record_or_none = self.fixed.select_ledger_entry();
                if record_or_none.is_none() {
                    return;
                }
                let selected_record = record_or_none.unwrap();
                let updated_record = self.fixed.modify(selected_record.clone());
                // record 0 should always be the initial of the account. 
                // if the date of the deposit changed, then so should the maturity date
                if updated_record.id == 0 { 
                    let cd = self.db.get_certificate_of_deposit(self.uid, self.id).unwrap();
                    if selected_record.info.date != updated_record.info.date {
                        let new_date_nv = NaiveDate::parse_from_str(&updated_record.info.date.as_str(), "%Y-%m-%d").unwrap();
                        // let original_date_nv = NaiveDate::parse_from_str(&selected_record.info.date.as_str(), "%Y-%m-%d").unwrap();
                        // let diff_in_months = (new_date_nv.month() as i32) - (original_date_nv.month() as i32);
                        // let updated_maturity_date = if diff_in_months > 0 { 
                        //     NaiveDate::parse_from_str(&cd.info.maturity_date, "%Y-%m-%d").unwrap()
                        //         .checked_add_months(Months::new(diff_in_months as u32)).unwrap().format("%Y-%m-%d").to_string()
                        // } else { 
                        //     NaiveDate::parse_from_str(&cd.info.maturity_date, "%Y-%m-%d").unwrap()
                        //         .checked_sub_months(Months::new((0-diff_in_months) as u32)).unwrap().format("%Y-%m-%d").to_string()
                        // };
                        let updated_maturity_date = new_date_nv.checked_add_months(Months::new(cd.info.length_months)).unwrap()
                            .format("%Y-%m-%d").to_string();
                        self.db.update_cd_maturity_date(self.uid, self.id, updated_maturity_date).unwrap();
                    }
                }
            }
            "Length" => { 
                let cd = self.db.get_certificate_of_deposit(self.uid, self.id).unwrap();
                let updated_length = CustomType::<u32>::new("Enter length (in months) to maturity:")
                    .with_placeholder("12")
                    .with_default(cd.info.length_months)
                    .with_error_message("Please type a valid number!")
                    .prompt()
                    .unwrap();
                let len_difference = (updated_length as i32) - (cd.info.length_months as i32);
                let updated_maturity_date =  if len_difference > 0 { 
                    NaiveDate::parse_from_str(&cd.info.maturity_date.as_str(), "%Y-%m-%d")
                        .unwrap()
                        .checked_add_months(Months::new(len_difference as u32))
                        .unwrap().format("%Y-%m-%d").to_string()
                } else { 
                    NaiveDate::parse_from_str(&cd.info.maturity_date.as_str(), "%Y-%m-%d")
                        .unwrap()
                        .checked_sub_months(Months::new((0-len_difference) as u32))
                        .unwrap().format("%Y-%m-%d").to_string()
                };
                self.db.update_cd_length(self.uid, self.id, updated_length).unwrap();
                self.db.update_cd_maturity_date(self.uid, self.id, updated_maturity_date).unwrap();
            }
            "Categories" => {
                let records = self.db.get_categories(self.uid, self.id).unwrap();
                let mut choices: Vec<String> = records.iter().map(|x| x.category.name.clone()).collect::<Vec<String>>();
                choices.push("None".to_string());
                let chosen_category = Select::new("Select category to modify:", choices)
                    .prompt()
                    .unwrap();

                if chosen_category == "None" {
                    return;
                }

                const MODIFY_ACTIONS: [&'static str; 3] = ["Update", "Remove", "None"];
                let update_or_remove = Select::new("What would you like to do:", MODIFY_ACTIONS.to_vec())
                    .prompt()
                    .unwrap();
                match update_or_remove { 
                    "Update" => {
                        let new_name = Text::new("Enter category name:").prompt().unwrap().to_string();
                        self.db.update_category_name(self.uid, self.id, chosen_category, new_name).unwrap();
                     }
                    "Remove" => {
                        // check if category is referenced by any current ledger
                        let is_referenced = self.db.check_if_ledger_references_category(self.uid, self.id, chosen_category.clone()).unwrap();
                        if is_referenced.is_some() {
                            let matched_records = is_referenced.unwrap();
                            println!("The following records were found:");
                            for record in matched_records {
                                let v = format!(
                                    "{} | {} | {} | {} ",
                                    record.info.date,
                                    chosen_category.clone(),
                                    self.db.get_participant(self.uid, self.id, record.info.participant).unwrap(),
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
                            self.db.remove_category(self.uid, self.id, chosen_category.clone());
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
                let selected_ptype = Select::new("What type of person:", PTYPE_OPTIONS.to_vec()).prompt().unwrap();
                let ptype = match selected_ptype { 
                    "Payer" => {
                        ParticipantType::Payer
                    }
                    "Payee" => { 
                        ParticipantType::Payee
                    }
                    "Both" => { 
                        ParticipantType::Both
                    }
                    _ => {
                        panic!("Unrecognized input: {}", selected_ptype);
                    }
                };
                let participants = self.db.get_participants(self.uid, self.id, ptype).unwrap();
                let mut people = participants.iter().map(|x| x.participant.name.clone()).collect::<Vec<String>>();
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
                let update_or_remove = Select::new("What would you like to do:", MODIFY_ACTIONS.to_vec())
                    .prompt()
                    .unwrap();

                match update_or_remove {
                    "Update" => { 
                        let new_name = Text::new("Enter person's name:").prompt().unwrap().to_string();
                        self.db.update_participant_name(self.uid, self.id, ptype, chosen_person.clone(), new_name).unwrap();
                    }
                    "Remove" => { 
                        // check if participant is referenced by any current ledger
                        let is_referenced = self.db.check_if_ledger_references_participant(self.uid, self.id,ptype, chosen_person.clone()).unwrap();
                        if is_referenced.is_some() {
                            let matched_records = is_referenced.unwrap();
                            println!("The following records were found:");
                            for record in matched_records {
                                let v = format!(
                                    "{} | {} | {} | {} ",
                                    record.info.date,
                                    self.db.get_category_name(self.uid, self.id, record.info.category_id).unwrap(),
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
                                    self.db.remove_participant(self.uid, self.id, ParticipantType::Payee, chosen_person.clone()).unwrap();
                                }
                                ParticipantType::Payer => {
                                    self.db.remove_participant(self.uid, self.id, ParticipantType::Payer, chosen_person.clone()).unwrap();
                                }
                                _ => {
                                    self.db.remove_participant(self.uid, self.id, ParticipantType::Payee, chosen_person.clone()).unwrap();
                                    self.db.remove_participant(self.uid, self.id, ParticipantType::Payer, chosen_person.clone()).unwrap();
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
            "Principal" => {
                let cd = self.db.get_certificate_of_deposit(self.uid, self.id).unwrap();
                let updated_principal = CustomType::<f32>::new("Enter principal:")
                    .with_placeholder("10000.00")
                    .with_default(cd.info.principal)
                    .with_error_message("Please type a valid amount!")
                    .prompt()
                    .unwrap();
                self.db.update_cd_principal(self.uid, self.id, updated_principal).unwrap();
            }
            "None" => { 
                return;
            }
            _ => { 
                panic!("Unrecognized input!")
            }
        }
    }

    fn export(&mut self) {}

    fn report(&mut self) {
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
                let (period_start, period_end) = query_user_for_analysis_period();
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

    fn link(&mut self, transacting_account: u32, entry: LedgerRecord) -> Option<u32> {
        let from_account;
        let to_account;

        let cid;
        let pid;
        let transacting_account_name : String;
        let (new_ttype, description) = match entry.info.transfer_type {
            TransferType::DepositFromExternalAccount => {
                // if the transacting account received a deposit, then self must be the "from" account
                from_account = self.id;
                to_account = transacting_account;
                cid =self.db.check_and_add_category(self.uid,self.id, "Withdrawal".to_ascii_uppercase());
                transacting_account_name = self.db.get_account_name(self.uid, transacting_account).unwrap();
                pid =self.db.check_and_add_participant(self.uid, self.id, transacting_account_name.clone(), ParticipantType::Payee, true);
                (
                    TransferType::WithdrawalToExternalAccount,
                    format!("[Link]: Withdrawal of ${} to account {} on {}.", entry.info.amount, transacting_account_name, entry.info.date)
                )
            }
            TransferType::WithdrawalToExternalAccount => {
                // if the transacting account had an amount withdrawn, then self must be the "to" account
                from_account = transacting_account;
                to_account = self.id;
                cid =self.db.check_and_add_category(self.uid,self.id, "Deposit".to_ascii_uppercase());
                transacting_account_name = self.db.get_account_name(self.uid, transacting_account).unwrap();
                pid =self.db.check_and_add_participant(self.uid, self.id, transacting_account_name.clone(), ParticipantType::Payer, true);
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
            transfer_type : new_ttype.clone(), 
            participant : pid, 
            category_id: cid, 
            description : description,
            ancillary_f32data : 0.0
        };

        let (from_ledger_id, to_ledger_id) = match new_ttype { 
            TransferType::WithdrawalToExternalAccount => { (
                self.db.add_ledger_entry(self.uid, self.id, linked_entry).unwrap(), 
                entry.id
            )}
            TransferType::DepositFromExternalAccount => {(
                entry.id,
                self.db.add_ledger_entry(self.uid, self.id, linked_entry).unwrap(), 
            )}
            _ => { panic!("Unrecognized input!")}
        };

        let transaction_record = AccountTransaction {
            from_account: from_account,
            to_account: to_account,
            from_ledger: from_ledger_id,
            to_ledger: to_ledger_id,
        };

        return Some(self.db.add_account_transaction(self.uid, transaction_record).unwrap());
    }
}

impl AccountData for CertificateOfDepositAccount {
    fn get_id(&mut self) -> u32 {
        return self.id
    } 
}

impl Account for CertificateOfDepositAccount {

}
