use chrono::Datelike;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use regex::Regex;
use serde::Deserialize;
use std::env;

use shared_lib::LedgerEntry;
use shared_lib::TransferType;

#[derive(Debug, Deserialize, Clone)]
pub struct VfcuCreditCardRecord {
    #[serde(rename = "Account Number")]
    pub acct: String,
    #[serde(rename = "Post Date")]
    pub posted_date: String,
    #[serde(rename = "Check")]
    pub check_number: Option<u32>,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Debit")]
    pub debit: Option<f32>,
    #[serde(rename = "Credit")]
    pub credit: Option<f32>,
    #[serde(rename = "Status")]
    pub status: String,
}

fn main() {
    // get args from command line
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        panic!("Only one argument is supported!");
    }
    let csv_file = args.get(1).expect("CSV file not provided!");

    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(csv_file)
        .unwrap();

    let mut transactions: Vec<VfcuCreditCardRecord> = Vec::new();

    //deserialize into structure
    for result in rdr.deserialize::<VfcuCreditCardRecord>() {
        match result {
            Ok(transaction) => {
                // println!("{:?}", transaction);
                transactions.push(transaction);
            }
            Err(err) => {
                panic!("{}", err);
            }
        }
    }

    // move credit/debit into one amount, with specified transaction type
    for txn in transactions {
        // println!("Record: {:?}", txn.clone());

        if txn.debit.is_some() && txn.credit.is_some() {
            panic!("Transactions cannot have debit and credit!");
        }
        if txn.debit.is_none() && txn.credit.is_none() {
            panic!("Transactions must have at least a credit or a debit!");
        }

        let mut amt = 0.0;
        let mut debit_not_credit: bool = false;
        if txn.debit.is_some() {
            amt = txn.debit.unwrap();
            debit_not_credit = true;
        }
        if txn.credit.is_some() {
            amt = txn.credit.unwrap();
            debit_not_credit = false;
        }

        let mut ttype: TransferType = TransferType::WithdrawalToInternalAccount;
        let mut peer = String::new();
        let mut cat = String::new();

        let re_loan = Regex::new(r"^Loan Advance Credit Card[\sA-Za-z]*\/(\s*|FSP*|TST\*|SQ\*)([A-Za-z0-9*#_\-\.\/\'&,]+\s[A-Za-z0-9*#_\-\.\/\'&,]+)").unwrap();
        let x = re_loan.captures(&txn.description.as_str());
        if x.is_some() {
            let x: regex::Captures<'_> = x.unwrap();
            if x.get(0).is_some() {
                if x.get(2).is_none() {
                    panic!("Peer not found for loan!");
                }
                peer = x.get(2).unwrap().as_str().to_string();
                cat = "Charge".to_string();
                ttype = TransferType::WithdrawalToExternalAccount;
            } else {
                panic!("Loan not recognized: {}", txn.description);
            }
        } else {
            let re_payment =
                Regex::new(r"Payments\s+Transfer\s+\-\s+From\s+([A-Za-z0-9\s]+)\/").unwrap();
            let x = re_payment.captures(&txn.description.as_str());
            if x.is_some() {
                let x = x.unwrap();
                if x.get(0).is_some() {
                    if x.get(1).is_none() {
                        panic!("Peer not found for payment!");
                    }
                    peer = x.get(1).unwrap().as_str().to_string();
                    cat = "Payment".to_string();
                    ttype = TransferType::DepositFromExternalAccount;
                } else {
                    panic!("Payment not recognized: {}", txn.description);
                }
            } else {
                panic!(
                    "Statment could not be matched for charge or payment: {}",
                    txn.description
                );
            }
        }

        let posted_date = NaiveDate::parse_from_str(&txn.posted_date, "%m/%d/%Y").unwrap();
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
            description: format!("\"{}\"", txn.description),
            ancillary_f32: 0.0,
            stock_info: None,
        };

        println!(
            "{},{:.2},{},{},{},{},{},,,,,",
            ledger_entry.date,
            ledger_entry.amount,
            ledger_entry.transfer_type as u32,
            ledger_entry.participant,
            ledger_entry.category,
            ledger_entry.description,
            ledger_entry.ancillary_f32
        );
    }
}
