use chrono::Datelike;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use serde::Deserialize;
use std::env;

use shared_lib::LedgerEntry;
use shared_lib::StockInfo;
use shared_lib::TransferType;

#[derive(Debug, Deserialize, Clone)]
pub struct EmpowerRecord {
    #[serde(rename = "Date")]
    pub date: String,
    #[serde(rename = "Account")]
    pub account: String,
    #[serde(rename = "Description")]
    pub description : String, 
    #[serde(rename = "Category")]
    pub category : String, 
    #[serde(rename = "Tags")]
    pub tags : String, 
    #[serde(rename = "Amount")]
    pub amount : f32, 
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

    let mut transactions: Vec<EmpowerRecord> = Vec::new();

    for result in rdr.deserialize::<EmpowerRecord>() {
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
        let cat: String = txn.category.clone();
        let peer: String;
        let description: String = txn.description.clone();
        let mut ancillary_data: f32 = 0.0;
        let mut stock_txn: StockInfo = StockInfo {
            shares: 0.0,
            costbasis: 0.0,
            remaining: 0.0,
            is_buy: false,
            is_split: false,
        };
        let mut has_stock = false;

        let posted_date = NaiveDate::parse_from_str(&txn.date, "%Y-%m-%d").unwrap();

        match txn.category.clone().to_lowercase().as_str() {
            "retirement contributions" => {
                ttype = shared_lib::TransferType::DepositFromExternalAccount;
                peer = "Self".to_ascii_uppercase().to_string();
            },
            _ => {
                let err_str = format!("Unrecognized category: {}", txn.category).to_string();
                // panic!(err_str);
            }
        };

        let amt = if txn.amount < 0.0 {
            -txn.amount
        } else {
            txn.amount
        };

    //         let ledger_entry = LedgerEntry {
    //             date: format!(
    //                 "{}-{}-{}",
    //                 posted_date.year(),
    //                 posted_date.month0() + 1,
    //                 posted_date.day0() + 1
    //             ),
    //             amount: amt,
    //             transfer_type: ttype,
    //             participant: peer,
    //             category: cat,
    //             description: format!("\"{}\"", description),
    //             stock_info: Some(stock_txn),
    //         };

    //     println!(
    //     "{},{},{},{},{},{},{},{},{},{},{},{}",
    //     ledger_entry.date,
    //     ledger_entry.amount,
    //     ledger_entry.transfer_type as u32,
    //     ledger_entry.participant,
    //     ledger_entry.category,
    //     ledger_entry.description,
    //     ledger_entry.stock_info.clone().unwrap().shares,
    //     ledger_entry.stock_info.clone().unwrap().costbasis,
    //     ledger_entry.stock_info.clone().unwrap().remaining,
    //     ledger_entry.stock_info.clone().unwrap().is_buy,
    //     ledger_entry.stock_info.clone().unwrap().is_split
    // );
    }
}
