use core::num;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::format;
use std::hash::Hash;
use std::process::CommandArgs;

use crate::database;
// use crate::database::db_accounts::AccountFilter;
// use crate::database::db_accounts::AccountType;
use crate::types::accounts::*;
use crate::types::investments::StockRecord;
use crate::types::investments::StockEntries;
// use crate::database::db_ledger::LedgerEntry;
// use crate::database::db_ledger::TransferType;
use crate::types::ledger::*;
use crate::types::transfer_types::TransferType;
use crate::types::participants::ParticipantType;
use crate::database::DbConn;
// use crate::ledger;
// use crate::ledger::Ledger;
use crate::types::ledger;
use crate::stocks;
use crate::tui::tui_accounts::*;
use crate::tui::tui_ledger::*;
use crate::tui::tui_user::*;
use chrono::NaiveDate;
use inquire::*;
use tokio::runtime::EnterGuard;
use yahoo_finance_api::Dividend;

use self::tui_budgets::amend_budget;

pub mod tui_accounts;
mod tui_ledger;
pub mod tui_budgets;
pub mod tui_user;

pub fn menu(_db: &mut DbConn) {
    let mut uid: u32;

    // set current user first!
    uid = tui_set_user(_db);

    loop {
        let commands: Vec<&str> = vec!["create", "change", "modify", "record", "report", "view", "exit"];
        let command: String = Select::new("What would you like to do:", commands)
            .prompt()
            .unwrap()
            .to_string();

        match command.as_str() {
            "create" => {
                tui_create(uid, _db);
            }
            "change" => {
                uid = tui_set_user(_db);
            }
            "record" => {
                tui_record(uid, _db);
            }
            "modify" => {
                tui_modify(uid, _db);
            }
            "report" => {
                tui_report(uid, _db);
            }
            "view" => {
                tui_view(uid, _db);
            }
            "exit" => {
                println!("Exiting...");
                break;
            }
            _ => {
                panic!("Invalid command.");
            }
        }
    }
}

fn tui_create(_uid: u32, _db: &mut DbConn) {
    let mut commands: Vec<&str> = Vec::new();
    let aid;
    if _db.is_admin(_uid).unwrap() {
        commands = vec![
            "user",
            "Bank",
            "CD",
            "Health",
            "Investment",
            "Ledger",
            "Retirement",
            "none",
        ];
    } else {
        commands = vec![
            "Bank",
            "CD",
            "Health",
            "Investment",
            "Retirement",
            "none",
        ];
    }
    let command: String = Select::new("\nWhat would you like to add:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "user" => {
            create_user(_db);
        }
        "Bank"|"CD" => {
            aid = create_account(AccountType::from(command), _uid, _db);
            let bank = record_ledger_entry(aid, _db, None);
        }
        "Investment"|"Retirement"|"Health" => {
            aid = create_account(AccountType::from(command), _uid, _db);
            
            _db.add_participant(aid, ParticipantType::Payee, "Fixed".to_string());
            _db.add_category(aid, "Bought".to_string());
            _db.add_category(aid, "Cash Dividend".to_string());
            _db.add_category(aid, "Interest".to_string());
            _db.add_category(aid, "Dividend-Reinvest".to_string());
            _db.add_category(aid, "Sold".to_string());
            _db.add_category(aid, "Deposit".to_string());
            _db.add_category(aid, "Withdrawal".to_string());

            let has_bank = Confirm::new("Would you like to record a fixed account?")
                .with_default(false)
                .prompt()
                .unwrap();
            if has_bank {
                let bank = record_ledger_entry(aid, _db, None);
            }
            let has_stock = Confirm::new("Would you like to record investments?")
                .with_default(false)
                .prompt()
                .unwrap();
            if has_stock {
                loop {
                    match record_stock_purchase(_uid) {
                        Some(record) => {
                            _db.add_stock(aid, record).expect("Unable to add stock!");
                        }
                        None => return,
                    }
                    let another: bool = Confirm::new("Add another stock to investment?")
                        .with_default(false)
                        .prompt()
                        .unwrap();
                    if false == another {
                        break;
                    }
                }
            }
        }
        "none" => return,
        _ => {
            panic!("Invalid command");
        }
    }
}

fn tui_modify(_uid: u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec![
        "budget",
        "none",
    ];
    let command: String = Select::new("\nWhat would you like to modify:", commands)
        .prompt()
        .unwrap()
        .to_string();

    match command.as_str() {
        "budget" => {
            let aid = select_account_by_filter(_uid, _db, AccountFilter::Budget);
            amend_budget(aid, _db);
        }
        "none" => {
            return
        }
        _ => {
            panic!("Invalid command!");
        }
    }
    return;

}

fn tui_record(_uid: u32, _db: &mut DbConn) {
    loop {
        let commands: Vec<&str> = vec![
            "Bank",
            "CD",
            "Health",
            "Investment",
            "Ledger",
            "Retirement",
            "none",
        ];

        let command: String = Select::new("\nWhat would you like to add:", commands)
            .prompt()
            .unwrap()
            .to_string();

        match command.as_str() {
            "Bank" => {
                let (aid, account) = select_account_by_type(_uid, _db, AccountType::Bank);
                // let record = record_f32_amount(_uid, _db);
                let fixed = record_ledger_entry(aid, _db, None);
                _db.add_ledger_entry(aid, fixed);
            }
            "CD" => {
                let (aid, account) = select_account_by_type(_uid, _db, AccountType::CD);
                let fixed = record_ledger_entry(aid, _db, Some(TransferType::DepositFromExternalAccount));
                _db.add_ledger_entry(aid, fixed);
            }
            "Retirement"|"Investment"|"Health" => {
                let (aid, account) = select_account_by_type(_uid, _db, AccountType::from(command));
                let command: String = Select::new("\nWhat would you like to record:", vec!["Fixed", "Variable"])
                    .prompt()
                    .unwrap()
                    .to_string();

                let transfer_type : TransferType;
                match command.as_str() {
                    "Fixed" => {
                        let transfer_source = Select::new("What type of transfer is this: ", vec!["Internal", "External"])
                            .prompt()
                            .unwrap()
                            .to_string();
                        let add_subtract = Select::new("Deposit or withdrawal:", vec!["Withdrawal", "Deposit"])
                            .prompt()
                            .unwrap()
                            .to_string();
                        if transfer_source == "Internal" {
                            if add_subtract == "Widthdrawal" {
                                transfer_type = TransferType::WidthdrawalToInternalAccount;
                            } else {

                                transfer_type = TransferType::DepositFromInternalAccount;
                            }
                        } else {
                            if add_subtract == "Widthdrawal" {
                                transfer_type = TransferType::WidthdrawalToExternalAccount;
                            } else {
                                transfer_type = TransferType::DepositFromExternalAccount;
                            }
                        }
                        let fixed = record_ledger_entry(aid, _db, Some(transfer_type));
                        _db.add_ledger_entry(aid, fixed);
                    }
                    "Variable" => {
                        loop {
                            let command: String = Select::new("\nWhat would you like to record:", vec!["Purchase", "Sale"])
                                .prompt()
                                .unwrap()
                                .to_string();

                            match command.as_str() {
                                "Purchase" => {
                                    match record_stock_purchase(_uid) {
                                        Some(record) => {
                                            let command: String = Select::new("\nPurchase from internal, external account,:", vec!["External", "Internal", "Dividend-Reinvest"])
                                                .prompt()
                                                .unwrap()
                                                .to_string();
                                            let mut purchase : LedgerEntry;
                                            let mut dividend : LedgerEntry;

                                            let payees = _db.get_people(aid, PeopleType::Payee).unwrap();
                                            let mut stock_pid = 0;
                                            if payees.is_empty() {
                                                stock_pid = _db.add_person(aid, PeopleType::Payee, record.ticker.clone()).unwrap();
                                            } else {
                                                let mut stock_found = false;
                                                let fixed_found = false;
                                                for payee in payees {
                                                    if payee == record.ticker.clone() {
                                                        stock_pid = _db.get_person_id(aid, payee).unwrap();
                                                        stock_found = true;
                                                        break;
                                                    }
                                                }
                                                if !stock_found {
                                                    stock_pid = _db.add_person(aid, PeopleType::Payee, record.ticker.clone()).unwrap();
                                                }
                                            }

                                            let mut fixed_pid = _db.get_person_id(aid, "Fixed".to_string()).unwrap();

                                            match command.as_str() {
                                                "External" => {

                                                    let mut fixed_pid = _db.get_person_id(aid, "Fixed".to_string()).unwrap();
                                                    let mut fixed_cid = _db.get_category_id(aid, "Deposit".to_string()).unwrap();
                                                    let mut stock_cid = _db.get_category_id(aid, "Bought".to_string()).unwrap();

                                                    let deposit  = LedgerEntry { 
                                                        date: format!("{}", record.date), 
                                                        amount: record.shares * record.costbasis,
                                                        transfer_type: TransferType::DepositFromExternalAccount,
                                                        payee_id: fixed_pid, 
                                                        category_id: fixed_cid, 
                                                        description : format!("[External]: Purchase {} shares of {} at ${} on {}.", record.shares, record.ticker , record.costbasis, record.date)
                                                    };
                                                    _db.add_ledger_entry(aid, deposit);

                                                    purchase = LedgerEntry { 
                                                        date: format!("{}", record.date), 
                                                        amount: record.shares * record.costbasis,
                                                        transfer_type: TransferType::WidthdrawalToInternalAccount,
                                                        payee_id: stock_pid, 
                                                        category_id: stock_cid, 
                                                        description : format!("[External]: Purchase {} shares of {} at ${} on {}.", record.shares, record.ticker, record.costbasis, record.date)
                                                    };
                                                }
                                                "Dividend-Reinvest" => {

                                                    let mut fixed_cid = _db.get_category_id(aid, "Cash Dividend".to_string()).unwrap();
                                                    let mut stock_cid = _db.get_category_id(aid, "Dividend-Reinvest".to_string()).unwrap();

                                                    let mut dividend = LedgerEntry { 
                                                        date: format!("{}", record.date), 
                                                        amount: record.shares * record.costbasis,
                                                        transfer_type: TransferType::DepositFromExternalAccount,
                                                        payee_id: fixed_pid, 
                                                        category_id: fixed_cid, 
                                                        description : format!("[Dividend-Reinvest]: Dividend of ${} from {} on {}.", record.shares * record.costbasis, record.ticker, record.date)
                                                    };

                                                    _db.add_ledger_entry(aid, dividend);

                                                    purchase = LedgerEntry { 
                                                        date: format!("{}", record.date), 
                                                        amount: record.shares * record.costbasis,
                                                        transfer_type: TransferType::WidthdrawalToInternalAccount,
                                                        payee_id: stock_pid, 
                                                        category_id: stock_cid, 
                                                        description : format!("[Dividend-Reinvest]: Purchase {} shares of {} at ${} on {}.", record.shares, record.ticker, record.costbasis, record.date)
                                                    };

                                                }
                                                "Internal" => { 
                                                    let mut stock_cid = _db.get_category_id(aid, "Bought".to_string()).unwrap();

                                                    purchase = LedgerEntry { 
                                                        date: format!("{}", record.date), 
                                                        amount: record.shares * record.costbasis,
                                                        transfer_type: TransferType::WidthdrawalToInternalAccount,
                                                        payee_id: stock_pid, 
                                                        category_id: stock_cid, 
                                                        description : format!("[Internal]: Purchase {} shares of {} at ${} on {}.", record.shares, record.ticker, record.costbasis, record.date)
                                                    };
                                                }
                                                _ => {
                                                    panic!("Invalid input type!");
                                                }
                                            }

                                            _db.add_ledger_entry(aid, purchase);
                                            _db.add_stock(aid, record);
                                            
                                        }
                                        None => return,
                                    }
                                }
                                "Sale" => {
                                    let tickers = _db.get_stock_tickers(aid).unwrap();
                                    let ticker = Select::new("\nSelect which stock you would like to record a sale of:", tickers)
                                        .prompt()
                                        .unwrap()
                                        .to_string();
                                    let owned_stocks = _db.get_stocks(aid, ticker.clone()).unwrap();

                                    let command: String = Select::new("Sell all or partial:", vec!["All", "Partial"])
                                        .prompt()
                                        .unwrap()
                                        .to_string(); 

                                    let sale_date: Result<NaiveDate, InquireError> = DateSelect::new("Enter date of purchase").prompt();

                                    let sale_price : f32 = CustomType::<f32>::new("Enter sale price: ")
                                        .with_placeholder("00000.00")
                                        .with_default(00000.00)
                                        .with_error_message("Please type a valid amount!")
                                        .prompt()
                                        .unwrap();

                                    let mut number_of_shares_sold : f32 = 0.0;
                                    let mut cost_basis_of_shares_sold : f32 = 0.0;

                                    match command.as_str() {
                                        "All" => {
                                            number_of_shares_sold = owned_stocks.iter().map(|stock_entry| stock_entry.record.shares).sum();
                                            cost_basis_of_shares_sold = owned_stocks.iter().map(|stock_entry| stock_entry.record.costbasis).sum();
                                            _db.drop_stock(aid, ticker.clone());
                                        }
                                        "Partial" => {
                                            let mut entry_map : HashMap<String, u32> = HashMap::new();
                                            let mut record_map: HashMap<u32, StockRecord> = HashMap::new();
                                            let mut commands : Vec<String> = Vec::new();
                                            for entries in owned_stocks {

                                                let key = format!(
                                                    "{}",
                                                    [entries.record.ticker.clone(), entries.record.date.clone(), entries.record.costbasis.to_string().clone()].join("\t")
                                                );

                                                // record_map.insert(entries.id, entries.record);

                                                entry_map.insert(key, entries.id.clone());
                                            }
                                            commands = entry_map.keys().cloned().collect();
                                            let selected_entry: String = Select::new("\nWhat stock would you like to sell", commands)
                                                .prompt()
                                                .unwrap()
                                                .to_string();
                                            let stock_id_to_update: u32 = entry_map.get(&selected_entry).expect("Stock not found!").to_owned();
                                            
                                            number_of_shares_sold = CustomType::<f32>::new("Enter number of shares sold: ")
                                                .with_placeholder("00000.00")
                                                .with_default(00000.00)
                                                .with_error_message("Please type a valid amount!")
                                                .prompt()
                                                .unwrap();

                                            let selected_record = record_map.get(&stock_id_to_update).expect("Stock record not found");
                                            // let updated_stock_entry : StockRecord = StockRecord { 
                                            //     ticker : selected_record.ticker.clone(),
                                            //     shares : selected_record.shares - number_of_shares_sold,
                                            //     costbasis : selected_record.costbasis,
                                            //     date : selected_record.date.clone()
                                            // };
                                            let updated_shares = selected_record.shares - number_of_shares_sold;

                                            if updated_shares == 0.0 { 
                                                _db.drop_stock_by_id(stock_id_to_update);
                                            } else {
                                                _db.update_stock_shares(stock_id_to_update, updated_shares);
                                            }

                                        }
                                        _ => {
                                            panic!("Unrecognized input!");
                                        }
                                    }

                                    let value_received = number_of_shares_sold * cost_basis_of_shares_sold;
                                    let stock_cid = _db.get_category_id(aid, "Sold".to_string()).unwrap();
                                    let payers = _db.get_people(aid, PeopleType::Payer).unwrap();
                                    let mut stock_pid = 0;
                                    if payers.is_empty() {
                                        stock_pid = _db.add_person(aid, PeopleType::Payer, ticker.clone()).unwrap();
                                    } else {
                                        let mut payer_found = false;
                                        for payer in payers { 
                                            if payer == ticker.clone() {
                                                stock_pid = _db.get_person_id(aid, payer).unwrap();
                                                payer_found = true;
                                                break;
                                            }
                                        }
                                        if !payer_found { 
                                            stock_pid = _db.add_person(aid, PeopleType::Payer, ticker.clone()).unwrap();
                                        }
                                    }

                                    let sale = LedgerEntry { 
                                        date: sale_date.as_ref().unwrap().to_string(), 
                                        amount : value_received, 
                                        transfer_type: TransferType::DepositFromInternalAccount,
                                        payee_id : stock_pid,
                                        category_id: stock_cid, 
                                        description : format!("[Internal]: Sold {} shares of {} at ${} on {}.", number_of_shares_sold, ticker, sale_price, sale_date.as_ref().unwrap().clone().to_string())
                                    };
                                    _db.add_ledger_entry(aid, sale).unwrap();
                                }
                                _ => {

                                }
                            }

                            let another: bool = Confirm::new("Add another stock to investment?")
                                .with_default(false)
                                .prompt()
                                .unwrap();
                            if false == another {
                                break;
                            }
                        }
                    }
                    _ => {
                        panic!("Unrecognized input!");
                    }
                }
            }
            "none" => {
                return;
            }
            _ => {
                panic!("Invalid command!");
            }
        }
    }
}

fn tui_report(_uid: u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["bank", "growth", "health", "investment", "ledger", "wealth", "none"];
    let command: String = Select::new("What would you like to report:", commands)
        .prompt()
        .unwrap()
        .to_string();
    let mut aid= 0;
    let account : String;

    match command.as_str() {
        "bank" => {
            (aid, account) = select_account_by_type(_uid, _db, AccountType::Bank);
            let value = _db.get_bank_value(aid).unwrap().amount;
            println!("The value of account {} is: {}", &account, value)
        }
        "health" => {
            // (aid, account) = select_account_by_type(_uid, _db, AccountType::Health);
            // let account = _db.get_account_name(_uid, aid).unwrap();
            // let acct = _db.get_hsa_value(aid).expect("Unable to get HSA account!");
            // let mut total_investment_value = 0.0;
            // for stock in acct.investments {
            //     total_investment_value += stocks::get_stock_at_close(stock.ticker)
            //         .expect("Unable to retrieve stock value!")
            //         * (stock.shares as f64);
            // }
            // let value = acct.fixed.amount as f64 + total_investment_value;
            // println!("The value of account {} is: {}", &account, value);
        }
        "growth" => {
            let account_types = vec![
                "Bank",
                "CD",
                "Health",
                "Investment",
                "Retirement",
                "none",
            ];
            let selected_type: String = Select::new("What would you like to analyze:", account_types)
                .prompt()
                .unwrap()
                .to_string();

            if selected_type != "none" {
                let (aid, account) = select_account_by_type(_uid, _db, AccountType::from(selected_type));
                get_growth(aid,_db);
            }
        }
        "investment" => {
            let (aid, account) = select_account_by_type(_uid, _db, AccountType::Investment);
            let report_all = Confirm::new(
                "Report total of entire account (y) or an individual stock ticker (n)",
            )
            .with_default(false)
            .prompt()
            .unwrap();
            let mut ticker = database::SQLITE_WILDCARD.to_string();
            if !report_all {
                ticker = select_stock(
                    _db.get_stock_tickers(aid)
                        .expect("Unable to retrieve stock tickers for this account!"),
                );
            }
            println!(
                "Value at last closing: {}",
                get_total_of_stocks(aid, _db, ticker)
            );
        }
        "ledger" => {
            // println!("Balance of account: {}", _ledger.sum());
        }
        "wealth" => {
            println!("Net wealth: {}", get_net_wealth(_uid, _db));
        }
        "none" => {
            return;
        }
        _ => {
            panic!("Invalid command!");
        }
    }

    // let acct = _db.get_account(aid).expect("Unable to retrieve user account!");
    // if acct.has

}

fn tui_view(_user: u32, _db: &mut DbConn) {
    let commands: Vec<&str> = vec!["ledger", "portfolio", "none"];
    let command: String = Select::new("What would you like to view:", commands)
        .prompt()
        .unwrap()
        .to_string();

    // match command.as_str() {
    //     "ledger" => {
    //         print_ledger(_user, _db);
    //     }
    //     _ => {
    //         panic!("Invalid command!");
    //     }
    // }
}

pub trait AccountOperations {
    fn create( account_id : u32, db : &mut DbConn );
    fn record( account_id : u32, db : &mut DbConn );
    fn modify( account_id : u32, db : &mut DbConn );
    fn export( account_id : u32, db : &mut DbConn );
    fn report( account_id : u32, db : &mut DbConn );
}