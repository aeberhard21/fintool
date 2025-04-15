use chrono::Datelike;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use serde::Deserialize;
use std::env;

use shared_lib::LedgerEntry;
use shared_lib::StockInfo;
use shared_lib::TransferType;

#[derive(Debug, Deserialize, Clone)]
pub struct LplRecord {
    #[serde(rename = "Account Name")]
    pub acct_name: String,
    #[serde(rename = "Nickname")]
    pub nickname: String,
    #[serde(rename = "Account Number")]
    pub acct_number: String,
    #[serde(rename = "Activity")]
    pub activity: String,
    #[serde(rename = "Amount($)")]
    pub amount: f32,
    #[serde(rename = "Date")]
    pub date: String,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Held In")]
    pub held_in: String,
    #[serde(rename = "Price($)")]
    pub price: String,
    #[serde(rename = "Quantity")]
    pub quantity: String,
    #[serde(rename = "Security")]
    pub security: char,
    #[serde(rename = "Source")]
    pub source: String,
    #[serde(rename = "Symbol/CUSIP")]
    pub symbol: String,
    #[serde(rename = "TransCode")]
    pub transcode: String,
    #[serde(rename = "Transaction")]
    pub transaction: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        eprintln!("Only one argument is supported!")
    }
    let csv_file = args.get(1).expect("CSV file not provided!");

    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(csv_file)
        .unwrap();

    let mut transactions: Vec<LplRecord> = Vec::new();

    for result in rdr.deserialize::<LplRecord>() {
        match result {
            Ok(transaction) => {
                transactions.push(transaction);
            }
            Err(e) => {
                eprintln!("Unable to deserialize transaction: {}", e);
                std::process::exit(1);
            }
        }
    }

    for txn in transactions {
        let ttype: TransferType;
        let cat: String = txn.activity.clone();
        let peer: String;
        let description: String;
        let mut ancillary_data : f32 = 0.0; 
        let mut stock_txn: StockInfo = StockInfo {
            shares: 0.0,
            costbasis: 0.0,
            remaining: 0.0,
            is_buy: false,
            is_split: false,
        };
        let mut has_stock = false;

        let quantity = match txn.quantity.as_str() {
            "-" => 0.0 as f32,
            _ => txn.quantity.parse::<f32>().unwrap(),
        };
        let price = match txn.price.as_str() {
            "-" => 0.0 as f32,
            _ => txn.price.parse::<f32>().unwrap(),
        };

        let posted_date = NaiveDate::parse_from_str(&txn.date, "%m/%d/%Y").unwrap();

        match txn.activity.clone().to_lowercase().as_str() {
            "contribution" => {
                ttype = TransferType::DepositFromExternalAccount;
                peer = "External Account".to_string();
                description = txn.description.clone();
            }
            "fee" => {
                ttype = TransferType::WithdrawalToExternalAccount;
                peer = "LPL".to_string();
                description = txn.description.clone();
            }
            "credit int" | "interest" => {
                ttype = TransferType::DepositFromInternalAccount;
                peer = "Insured Cash Account".to_string();
                description = txn.description.clone();
            }
            "reinvest interest" | "interest reinvest" => {
                // skipping because it seems like that LPL takes
                // interest money and categorizes for reinvestment immediately.
                continue;
            }
            "cash dividend" => {
                ttype = TransferType::DepositFromInternalAccount;
                peer = txn.symbol;
                description = txn.description.clone();
            }
            "lt cap gain reinvest" => {
                ttype = TransferType::WithdrawalToInternalAccount;
                peer = txn.symbol.clone();
                description = txn.description.clone();
            }
            "long term cap gain" => {
                ttype = TransferType::DepositFromInternalAccount;
                peer = txn.symbol;
                description = txn.description.clone();
            }
            "st cap gain reinvest" => {
                ttype = TransferType::WithdrawalToInternalAccount;
                peer = txn.symbol;
                description = txn.description.clone();
            }
            "short term cap gain" => {
                ttype = TransferType::DepositFromInternalAccount;
                peer = txn.symbol;
                description = txn.description.clone();
            }
            "dividend reinvest" => {
                ttype = TransferType::WithdrawalToInternalAccount;
                peer = txn.symbol;
                description = txn.description.clone();
            }
            "buy" => {
                ttype = TransferType::WithdrawalToInternalAccount;
                peer = txn.symbol.clone();
                description = txn.description.clone();

                stock_txn = StockInfo {
                    shares: quantity,
                    costbasis: price,
                    remaining: quantity,
                    is_buy: true,
                    is_split: false,
                };
                has_stock = true;
            }
            "sell" => {
                ttype = TransferType::DepositFromInternalAccount;
                peer = txn.symbol.clone();
                description = txn.description.clone();

                stock_txn = StockInfo {
                    shares: quantity,
                    costbasis: price,
                    remaining: quantity,
                    is_buy: false,
                    is_split: false,
                };
                has_stock = true;
            }
            "ach funds" => {
                ttype = TransferType::DepositFromExternalAccount;
                peer = "External Account".to_string();
                description = txn.description;
            }
            "deposit" => {
                ttype = TransferType::DepositFromExternalAccount;
                peer = "External Account".to_string();
                description = txn.description;
            }
            "stock dividend/split" => {
                ttype = TransferType::ZeroSumChange;
                peer = txn.symbol.clone();
                description = txn.description.clone();
                ancillary_data = quantity;

                stock_txn = StockInfo {
                    shares: quantity,
                    costbasis: price,
                    remaining: quantity,
                    is_buy: true,
                    is_split: true,
                };
                has_stock = true;
            }
            "journal" => {
                ttype = TransferType::DepositFromExternalAccount;
                peer = "External Account".to_string();
                description = txn.description;
            }
            _ => {
                eprintln!("Unrecognized activity type: {}", txn.activity);
                std::process::exit(1);
            }
        };

        let amt = if txn.amount < 0.0 {
            -txn.amount
        } else {
            txn.amount
        };

        if has_stock {
            let ledger_entry = LedgerEntry {
                date: format!(
                    "{}-{}-{}",
                    posted_date.year(),
                    posted_date.month0() + 1,
                    posted_date.day0() + 1
                ),
                amount: amt,
                transfer_type: ttype,
                participant: peer,
                category: cat,
                description: format!("\"{}\"", description),
                ancillary_f32 : ancillary_data,
                stock_info: Some(stock_txn),
            };

            println!(
                "{},{},{},{},{},{},{},{},{},{},{}",
                ledger_entry.date,
                ledger_entry.amount,
                ledger_entry.transfer_type as u32,
                ledger_entry.participant,
                ledger_entry.category,
                ledger_entry.description,
                ledger_entry.stock_info.clone().unwrap().shares,
                ledger_entry.stock_info.clone().unwrap().costbasis,
                ledger_entry.stock_info.clone().unwrap().remaining,
                ledger_entry.stock_info.clone().unwrap().is_buy,
                ledger_entry.stock_info.clone().unwrap().is_split
            );
        } else {
            let ledger_entry = LedgerEntry {
                date: format!(
                    "{}-{}-{}",
                    posted_date.year(),
                    posted_date.month0() + 1,
                    posted_date.day0() + 1
                ),
                amount: amt,
                transfer_type: ttype,
                participant: peer,
                category: cat,
                description: format!("\"{}\"", description),
                ancillary_f32 : ancillary_data,
                stock_info: None,
            };

            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{}",
                ledger_entry.date,
                ledger_entry.amount,
                ledger_entry.transfer_type as u32,
                ledger_entry.participant,
                ledger_entry.category,
                ledger_entry.description,
                "",
                "",
                "",
                "",
                "",
                "",
                ""
            );
        };
    }
}
