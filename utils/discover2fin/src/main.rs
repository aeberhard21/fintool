/* ------------------------------------------------------------------------
    Copyright (C) 2025  Andrew J. Eberhard

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
  -----------------------------------------------------------------------*/
use chrono::Datelike;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use regex::Regex;
use serde::Deserialize;
use std::env;

use shared_lib::LedgerEntry;
use shared_lib::StockInfo;
use shared_lib::TransferType;

#[derive(Debug, Deserialize, Clone)]
pub struct DiscoverRecord {
    #[serde(rename = "Trans. Date")]
    pub transaction_date: String,
    #[serde(rename = "Post Date")]
    pub post_date: String,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Amount")]
    pub amount: f32,
    #[serde(rename = "Category")]
    pub category: String,
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

    let mut transactions: Vec<DiscoverRecord> = Vec::new();

    for result in rdr.deserialize::<DiscoverRecord>() {
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
        let cat: String = txn.category.clone();
        let description: String = txn.description.clone();
        let ancillary_data: f32 = 0.0;

        let posted_date = NaiveDate::parse_from_str(&txn.transaction_date, "%m/%d/%Y").unwrap();

        let amt = if txn.amount < 0.0 {
            -txn.amount
        } else {
            txn.amount
        };

        let x;
        let mut captured_peer: String;
        let (peer, ttype) = match cat.as_str() {
            "Awards and Rebate Credits" => {
                (
                    // credits from discover
                    "Discover Financial Services",
                    TransferType::DepositFromInternalAccount,
                )
            }
            "Payments and Credits" => {
                ("Checking Account", TransferType::DepositFromExternalAccount)
            }
            _ => {
                let re = Regex::new(
                    r"^(\s*|TST\*|SQ\*)([A-Za-z0-9*#_\-\.\/\'&,]+\s[A-Za-z0-9*#_\-\.\/\'&,]+)",
                )
                .unwrap();
                x = re.captures(&txn.description.as_str()).unwrap();
                if x.get(0).is_none() && x.get(2).is_none() {
                    panic!(
                        "{} did not produce a valid match for a participant!",
                        txn.description
                    );
                }
                captured_peer = format!("\"{}\"", x.get(2).unwrap().as_str()).clone();
                (
                    captured_peer.as_str(),
                    TransferType::WithdrawalToExternalAccount,
                )
            }
        };

        let ledger_entry = LedgerEntry {
            date: format!(
                "{}-{}-{}",
                posted_date.year(),
                posted_date.month0() + 1,
                posted_date.day0() + 1
            ),
            amount: amt,
            transfer_type: ttype,
            participant: peer.to_string(),
            category: cat,
            description: format!("\"{}\"", description),
            stock_info: None,
        };

        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
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
            ""
        );
    }
}
