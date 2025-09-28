use chrono::Datelike;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Cursor;

use std::fs::File;
use std::io::Read;

pub mod ofx_defs;
use ofx_defs::OFX;

use shared_lib::LedgerEntry;
use shared_lib::StockInfo;
use shared_lib::TransferType;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        eprintln!("Only one argument is supported!")
    }
    let ofx_file_path = args.get(1).expect("OFX file not provided!");
    let ofx_file = fs::read_to_string(ofx_file_path).unwrap();
    let xml = ofx_to_xml(&ofx_file);

    let ofx: OFX = serde_xml_rs::from_str(xml.as_str()).unwrap();

    let transactions: Vec<LedgerEntry> = ofx
        .investment_sign_on_msg
        .map(|msg| {
            let invtran = msg
                .investment_statement_transaction_response
                .investment_statement_response
                .investment_transaction_list;

            let mut v = Vec::new();

            // if let Some(txns) = invtran.investment_bank_transactions {
            //     v.extend(txns.into_iter().map(LedgerEntry::from));
            // }

            if let Some(buys) = invtran.buy_mf {
                v.extend(buys.into_iter().map(LedgerEntry::from));
            }

            // if let Some(sells) = invtran.sell_mf {
            //     v.extend(sells.into_iter().map(LedgerEntry::from));
            // }

            v
        })
        .unwrap_or_default();

    for ledger_entry in transactions { 
       if ledger_entry.stock_info.is_some() {

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
            println!(
                "{},{},{},{},{},{},{},{},{},{},{}",
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
            );
        };
    }
}

/// Convert raw OFX (SGML-ish) into well-formed XML string.
fn ofx_to_xml(ofx_text: &str) -> String {
    // 1) Find the first '<' (start of the OFX XML body). Drop preceding header lines.
    let start = ofx_text.find('<').unwrap_or(0);
    let body = &ofx_text[start..];
    let xml = body.to_string();
    xml
}
