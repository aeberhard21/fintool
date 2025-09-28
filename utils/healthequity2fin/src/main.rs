use chrono::Datelike;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use shared_lib::stocks::get_stock_quote;
use std::env;
use std::io;
use std::io::Write;

use shared_lib::LedgerEntry;
use shared_lib::StockInfo;
use shared_lib::TransferType;

#[derive(Debug, Deserialize, Clone)]
pub struct HealthEquityRecord {
    #[serde(rename = "Date")]
    pub date: String,
    #[serde(rename = "Transaction")]
    pub transaction: String,
    #[serde(rename = "Amount", deserialize_with = "deserialize_accounting_f32")]
    pub amount: f32,
    #[serde(
        rename = "HSA Cash Balance",
        deserialize_with = "deserialize_accounting_f32"
    )]
    pub acct_balance: f32,
    #[serde(rename = "Attachments")]
    pub attachments: String,
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

    let mut transactions: Vec<HealthEquityRecord> = Vec::new();

    for result in rdr.deserialize::<HealthEquityRecord>() {
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
        let mut cat: String = "BUY".to_string();
        let mut description: String = "TEMPORARY".to_string();
        let ancillary_data: f32 = 0.0;
        let t_type: TransferType;
        let mut peer = "SELF".to_string();
        let mut has_stock = false;

        let posted_date = NaiveDate::parse_from_str(&txn.date, "%m/%d/%Y").unwrap();

        let amt = if txn.amount < 0.0 {
            -txn.amount
        } else {
            txn.amount
        };

        let mut stock: Option<StockInfo> = None;

        let transaction_re = Regex::new(
            r"^(Investment:|Employee Contribution|Employer Contribution|Incentive Contribution|Interest|Balance|From BenefitWallet:|Transfer from BenefitWallet)\s+(.+)"
        ).unwrap();
        let x = transaction_re.captures(&txn.transaction.as_str());
        if x.is_none() {
            panic!("1 - {} did not match!", txn.transaction);
        }
        let x = x.unwrap();
        if x.get(0).is_some() {
            let activity = x.get(1).unwrap().as_str().to_string();
            let helper_data = x.get(2).unwrap().as_str().to_string();
            match activity.as_str() {
                "Investment:" => {
                    let ticker = helper_data.clone();
                    let history = get_stock_quote(ticker.clone(), posted_date);
                    if let Ok(quote) = history {
                        let close = quote.close as f32;
                        let shares = amt / close;

                        // purchase of stock
                        let si = StockInfo {
                            shares: shares.clone(),
                            remaining: shares.clone(),
                            costbasis: close,
                            is_buy: true,
                            is_split: false,
                        };
                        stock = Some(si);

                        t_type = TransferType::WithdrawalToInternalAccount;
                        description = format!(
                            "Purchase {} shares of {} at ${} on {}.",
                            shares, ticker, amt, txn.date
                        );
                        cat = "BUY".to_string();
                        peer = helper_data;
                        has_stock = true;
                    } else {
                        panic!("Ticker not recognized: {}!", ticker);
                    }
                }
                "Employee Contribution" | "Incentive Contribution" => {
                    t_type = TransferType::DepositFromExternalAccount;
                    cat = "DEPOSIT".to_string();
                    description =
                        format!("Employee contribution of ${} on {}.", txn.amount, txn.date);
                    peer = "SELF".to_string();
                }
                "Employer Contribution" => {
                    t_type = TransferType::DepositFromExternalAccount;
                    cat = "DEPOSIT".to_string();
                    description =
                        format!("Employee contribution of ${} on {}.", txn.amount, txn.date);
                    peer = "EMPLOYER".to_string();
                }
                "Interest" => {
                    t_type = TransferType::DepositFromInternalAccount;
                    cat = "INTEREST".to_string();
                    peer = "Health Equity".to_ascii_uppercase().to_string();
                    description =
                        format!("Employee contribution of ${} on {}.", txn.amount, txn.date);
                }
                "Balance" => {
                    t_type = TransferType::DepositFromInternalAccount;
                }
                "Transfer from BenefitWallet" => {
                    t_type = TransferType::DepositFromInternalAccount;
                    cat = "DEPOSIT".to_string();
                    peer = "Benefit Wallet".to_ascii_uppercase().to_string();
                    description = format!(
                        "Transfer of funds amounting to ${} on {}.",
                        txn.amount, txn.date
                    );
                }
                "From BenefitWallet:" => {
                    let helper_data = match helper_data.as_str().find('(') {
                        Some(index) => helper_data[..index].trim_end().to_string(),
                        None => helper_data,
                    };

                    let helper_data = match helper_data.as_str().find("POSTED THROUGH") {
                        Some(index) => helper_data[..index].trim_end().to_string(),
                        None => helper_data,
                    };

                    match helper_data.as_str() {
                        "Transfer" => {
                            t_type = TransferType::DepositFromInternalAccount;
                            cat = "DEPOSIT".to_string();
                            peer = "Benefit Wallet".to_ascii_uppercase().to_string();
                            description = format!(
                                "Transfer of funds amounting to ${} on {}.",
                                txn.amount, txn.date
                            );
                        }
                        "HSA INVEST" => {
                            t_type = TransferType::WithdrawalToInternalAccount;
                            cat = "BUY".to_string();
                            peer = "Investment Fund".to_ascii_uppercase().to_string();
                            description = format!("Purchases of funds amounting to ${} on {}. Funds purchased not disclosed in data provided by HealthEquity.", txn.amount, txn.date);
                        }
                        "HSA CHECK DISBURSEMENT" => {
                            t_type = TransferType::WithdrawalToExternalAccount;
                            cat = "Check Disbursement".to_string();
                            peer = "Self".to_ascii_uppercase().to_string();
                            description = format!(
                                "Distribution of funds amounting to ${} on {}.",
                                txn.amount, txn.date
                            );
                        }
                        "EMPLOYEE PAYROLL CONTRIBUTION" => {
                            t_type = TransferType::DepositFromExternalAccount;
                            cat = "CONTRIBUTION".to_string();
                            peer = "Self".to_ascii_uppercase().to_string();
                            description = format!(
                                "Contribution of funds amounting to ${} on {}.",
                                txn.amount, txn.date
                            );
                        }
                        "EMPLOYEE PAYROLL DEBIT" => {
                            t_type = TransferType::WithdrawalToExternalAccount;
                            cat = "DEBIT".to_string();
                            peer = "Self".to_ascii_uppercase().to_string();
                            description = format!(
                                "Debit of funds amounting to ${} on {}.",
                                txn.amount, txn.date
                            );
                        }
                        "PARTIAL MONTH INTEREST" | "INTEREST" => {
                            t_type = TransferType::DepositFromInternalAccount;
                            cat = "INTEREST".to_string();
                            peer = "Health Equity".to_ascii_uppercase().to_string();
                            description = format!(
                                "Account interest amounting to ${} on {}.",
                                txn.amount, txn.date
                            );
                        }
                        "WELLNESS PAYROLL CREDIT" => {
                            t_type = TransferType::DepositFromExternalAccount;
                            cat = "Wellness Incentive".to_string();
                            peer = "Employer".to_ascii_uppercase().to_string();
                            description = format!(
                                "Wellness payroll credit amounting to ${} on {}.",
                                txn.amount, txn.date
                            );
                        }
                        "EMPLOYER PAYROLL CONTRIBUTION" => {
                            t_type = TransferType::DepositFromExternalAccount;
                            cat = "CONTRIBUTION".to_string();
                            peer = "Employer".to_ascii_uppercase().to_string();
                            description = format!(
                                "Employer contribution amounting to ${} on {}.",
                                txn.amount, txn.date
                            );
                        }
                        "Starting Balance" | "Transfer to HealthEquity" => {
                            continue;
                        }
                        _ => {
                            panic!("Unrecognized helper data: {}", helper_data);
                        }
                    }
                }
                _ => {
                    panic!("Unrecognized activity: {}", activity);
                }
            }
        } else {
            panic!(
                "{} did not produce a valid match for a transaction!",
                txn.transaction
            );
        }
        if has_stock {
            let ledger_entry = LedgerEntry {
                date: format!(
                    "{}-{}-{}",
                    posted_date.year(),
                    posted_date.month0() + 1,
                    posted_date.day0() + 1
                ),
                amount: amt,
                transfer_type: t_type,
                participant: peer.to_ascii_uppercase(),
                category: cat.to_ascii_uppercase(),
                description: format!("\"{}\"", description),
                stock_info: stock,
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
                transfer_type: t_type,
                participant: peer,
                category: cat,
                description: format!("\"{}\"", description),
                stock_info: None,
            };

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

fn deserialize_accounting_f32<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;

    // Trim whitespace
    let s = s.trim();
    let s = s.replace(",", "");

    // Handle parentheses for negative numbers
    let cleaned = if s.starts_with('(') && s.ends_with(')') {
        let inner = &s[1..s.len() - 1].trim(); // remove parens and trim
        format!("-{}", inner.trim_start_matches('$')) // remove dollar and add minus
    } else {
        s.trim_start_matches('$').to_string()
    };

    let x = cleaned.parse::<f32>().map_err(serde::de::Error::custom);
    x
}
