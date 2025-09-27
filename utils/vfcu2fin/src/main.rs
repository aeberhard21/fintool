use chrono::Datelike;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use regex::Regex;
use serde::Deserialize;
use std::env;

use shared_lib::LedgerEntry;
use shared_lib::TransferType;

#[derive(Debug, Deserialize, Clone)]
pub struct VfcuRecord {
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
    #[serde(rename = "Classification")]
    pub classification: Option<String>,
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

    let mut transactions: Vec<VfcuRecord> = Vec::new();

    //deserialize into structure
    for result in rdr.deserialize::<VfcuRecord>() {
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
        if txn.classification.is_some() {
            match txn.classification.clone().unwrap().as_str() {
                "Transfer" => {
                    let re = Regex::new(r"(to|from)\s([A-Za-z0-9\s]+)").unwrap();
                    let x = re.captures(txn.description.as_str());

                    if debit_not_credit {
                        ttype = TransferType::WithdrawalToExternalAccount;
                        cat = "Withdrawal".to_string();
                    } else {
                        ttype = TransferType::DepositFromExternalAccount;
                        cat = "Deposit".to_string();
                    }

                    if x.is_some() {
                        let x = x.unwrap();
                        if x.get(1).unwrap().as_str() == "to" {
                            ttype = TransferType::WithdrawalToExternalAccount;
                            cat = "Withdrawal".to_string();
                        } else if x.get(1).unwrap().as_str() == "from" {
                            ttype = TransferType::DepositFromExternalAccount;
                            cat = "Deposit".to_string();
                        } else {
                            eprintln!("Unrecognized transfer type: {}", x.get(1).unwrap().as_str());
                            std::process::exit(1);
                        }

                        peer = x.get(2).unwrap().as_str().to_string();
                    } else {
                        peer = "Misc".to_string();
                        cat = txn.description.clone();
                        if !debit_not_credit {
                            ttype = TransferType::WithdrawalToExternalAccount;
                        } else {
                            ttype = TransferType::DepositFromExternalAccount;
                        }
                    }
                }
                "Interest Income" => {
                    ttype = TransferType::DepositFromInternalAccount;
                    peer = "Visions FCU".to_string();
                    cat = "Interest".to_string();
                }
                "Dividend &amp; Cap Gains" => {
                    // treating the same as interest/income
                    ttype = TransferType::DepositFromInternalAccount;
                    peer = "Visions FCU".to_string();
                    cat = "Interest".to_string();
                }
                "Cash" => {
                    ttype = TransferType::WithdrawalToExternalAccount;
                    peer = txn.description.clone();
                    cat = txn.classification.unwrap();
                }
                "Paycheck" => {
                    ttype = TransferType::DepositFromExternalAccount;
                    peer = txn.description.clone();
                    cat = txn.classification.unwrap();
                }
                "Investments" => {
                    ttype = TransferType::WithdrawalToExternalAccount;
                    peer = txn.description.clone();
                    cat = txn.classification.unwrap();
                }
                "Check" => {
                    ttype = TransferType::WithdrawalToExternalAccount;
                    peer = "Check".to_string();
                }
                "Food &amp; Dining" => {
                    if debit_not_credit {
                        ttype = TransferType::WithdrawalToExternalAccount;
                    } else {
                        ttype = TransferType::DepositFromExternalAccount;
                    }
                    peer = txn.description.clone();
                    cat = "Food and Dining".to_string();
                }
                "Credit Card Payment" => {
                    ttype = TransferType::WithdrawalToExternalAccount;
                    peer = txn.description.clone();
                    cat = "Credit Card Payment".to_string();
                }
                "Income" => {
                    ttype = TransferType::DepositFromExternalAccount;
                    peer = txn.description.clone();
                    cat = txn.classification.unwrap();
                }
                "Financial" => {
                    if debit_not_credit {
                        ttype = TransferType::WithdrawalToExternalAccount;
                    } else {
                        ttype = TransferType::DepositFromExternalAccount;
                    }
                    peer = txn.description.clone();
                    cat = "Financial".to_string();
                }
                "Mortgage &amp; Rent" => {
                    if debit_not_credit {
                        ttype = TransferType::WithdrawalToExternalAccount;
                    } else {
                        ttype = TransferType::DepositFromExternalAccount;
                    }
                    peer = txn.description.clone();
                    cat = "Mortgage & Rent".to_string();
                }
                "Travel" => {
                    if debit_not_credit {
                        ttype = TransferType::WithdrawalToExternalAccount;
                    } else {
                        ttype = TransferType::DepositFromExternalAccount;
                    }
                    peer = txn.description.clone();
                    cat = txn.classification.unwrap();
                }
                "Federal Tax" => {
                    if debit_not_credit {
                        ttype = TransferType::WithdrawalToExternalAccount;
                    } else {
                        ttype = TransferType::DepositFromExternalAccount;
                    }
                    peer = "IRS".to_string();
                    cat = txn.classification.unwrap();
                }
                "State Tax" => {
                    if debit_not_credit {
                        ttype = TransferType::WithdrawalToExternalAccount;
                    } else {
                        ttype = TransferType::DepositFromExternalAccount;
                    }
                    peer = "IRS".to_string();
                    cat = txn.classification.unwrap();
                }
                "Television" => {
                    if debit_not_credit {
                        ttype = TransferType::WithdrawalToExternalAccount;
                    } else {
                        ttype = TransferType::DepositFromExternalAccount;
                    }
                    peer = "Utilities".to_string();
                    cat = txn.classification.unwrap();
                }
                _ => {
                    eprintln!(
                        "Unrecognized classification type: {}",
                        txn.classification.unwrap()
                    );
                    std::process::exit(1);
                }
            };
        } else {
            let re = Regex::new(r"^(Deposit|Withdrawal|Check)").unwrap();
            let x = re.captures(txn.description.as_str()).unwrap();
            if x.get(1).unwrap().as_str() == "Deposit" {
                let re = Regex::new(r"Deposit\s+Dividend").unwrap();
                if re.is_match(txn.description.as_str()) {
                    ttype = TransferType::DepositFromInternalAccount;
                } else {
                    ttype = TransferType::DepositFromExternalAccount;
                }
                peer = "Misc".to_string();
                cat = "Deposit".to_string();
            }
            if x.get(1).unwrap().as_str() == "Withdrawal" {
                ttype = TransferType::WithdrawalToExternalAccount;
                peer = "Misc".to_string();
                cat = "Withdrawal".to_string();
            }
            if x.get(1).unwrap().as_str() == "Check" {
                ttype = TransferType::WithdrawalToExternalAccount;
                cat = "Check".to_string();
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
        );
    }
}
