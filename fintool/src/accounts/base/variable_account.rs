use core::alloc;
use std::backtrace;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

use chrono::NaiveTime;
use chrono::{Date, Days, Local, NaiveDate, NaiveDateTime};
use csv::DeserializeError;
use inquire::*;
use rusqlite::types::Value;
use rustyline::validate::Validator;
use time::OffsetDateTime;
use yahoo_finance_api::Quote;
use yahoo_finance_api::YahooError;

use crate::accounts::base::{SharesOwned, StockData};
use crate::database::DbConn;
use crate::types::investments::{
    SaleAllocationInfo, SaleAllocationRecord, StockInfo, StockRecord, StockSplitAllocationInfo,
    StockSplitInfo, StockSplitRecord,
};
use crate::types::ledger::{LedgerInfo, LedgerRecord};
use crate::types::participants::ParticipantAutoCompleter;
use crate::types::participants::ParticipantType;
use crate::types::stock_prices::StockPriceInfo;
use crate::types::stock_prices::StockPriceRecord;
use shared_lib::stocks::{self, get_stock_history};
use shared_lib::{LedgerEntry, TransferType};

use super::fixed_account::FixedAccount;

pub struct VariableAccount {
    pub id: u32,
    pub uid: u32,
    pub db: DbConn,
    pub fixed: FixedAccount,
    pub buffer: Option<Vec<StockData>>,
    pub open_date: NaiveDate,
}

impl VariableAccount {
    pub fn new(uid: u32, id: u32, db: &DbConn, open_date: NaiveDate) -> Self {
        let mut acct: VariableAccount = Self {
            id: id,
            uid: uid,
            db: db.clone(),
            fixed: FixedAccount::new(uid, id, db.clone()),
            buffer: None,
            open_date: open_date,
        };
        acct.initialize_buffer();
        acct
    }

    pub fn initialize_buffer(&mut self) {
        // this is a quick hack to update the buffer after a stock has been purchased, sold or split
        let earliest_date = self.open_date;
        let latest_date = Local::now().date_naive();

        let x = self.db.get_positions_by_ledger(self.id, self.uid).unwrap();
        if x.is_some() {
            let x = x.unwrap();
            let mut tickers = x.iter().map(|x| x.0.clone()).collect::<Vec<String>>();
            tickers.dedup();
            let mut data: Vec<StockData> = Vec::new();

            let buffer = if let Some(buffer) = self.buffer.take() {
                buffer
            } else {
                Vec::new()
            };

            for ticker in tickers {
                let date_shares = x
                    .iter()
                    .filter(|data| data.0 == ticker)
                    .map(|x: &(String, String, f32)| {
                        (SharesOwned {
                            date: NaiveDate::parse_from_str(&&x.1, "%Y-%m-%d")
                                .expect(format!("Unable to decode {}", &x.1).as_str()),
                            shares: x.2.clone(),
                        })
                    })
                    .collect::<Vec<SharesOwned>>();
                let quotes = buffer
                    .iter()
                    .find(|x| x.ticker == ticker)
                    .and_then(|x| Some(x.quotes.clone()));
                let quotes = quotes
                    .or_else(|| {
                        Some({
                            let pid = self
                                .db
                                .get_participant_id(
                                    self.uid,
                                    self.id,
                                    ticker.clone(),
                                    ParticipantType::Payee,
                                )
                                .unwrap();
                            let manual_prices = self
                                .db
                                .check_and_get_stock_price_record_matching_from_participant_id(
                                    self.uid, self.id, pid,
                                )
                                .unwrap();
                            if manual_prices.is_empty() {
                                get_stock_history(ticker.clone(), earliest_date, latest_date)
                                    .unwrap()
                            } else {
                                Self::convert_stock_price_record_to_quotes(&manual_prices)
                            }
                        })
                    })
                    .unwrap();
                data.push(StockData {
                    ticker: ticker.clone(),
                    quotes: quotes,
                    history: date_shares,
                });
            }
            self.buffer = Some(data);
        }
    }

    pub fn purchase_stock(
        &mut self,
        initial_opt: Option<StockRecord>,
        overwrite_entry: bool,
    ) -> Option<LedgerRecord> {
        let purchase: LedgerInfo;
        let defaults_to_use: bool;
        let mut initial: StockRecord = StockRecord {
            id: 0,
            info: StockInfo {
                shares: 0.0,
                costbasis: 0.0,
                remaining: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };

        if initial_opt.is_some() {
            defaults_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            defaults_to_use = false;
        }

        let ticker_msg = "Enter stock ticker:";
        let ticker= if defaults_to_use {
            let pid = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .participant;
            let initial_ticker = self.db.get_participant(self.uid, self.id, pid).unwrap();
            let entered_ticker = Text::new(ticker_msg)
                .with_default(initial_ticker.as_str())
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string();

            entered_ticker
        } else {
            let entered_ticker = Text::new(ticker_msg)
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string();

            entered_ticker
        };

        let public_ticker = self.confirm_public_ticker(ticker.clone());
        let manual_entry = if !public_ticker {
            // check if already a member that is being tracked.
            let pid_opt = self.db.get_participant_id(self.uid, self.id, ticker.clone(), ParticipantType::Payee);
            if let Some(pid) = pid_opt { 
                let stock_is_tracked = self.db.check_and_get_stock_price_record_matching_from_participant_id(self.uid, self.id, pid).unwrap();
                if stock_is_tracked.is_empty() { 
                    panic!("Non-public ticker does not have a stock price record!");
                }
                false
            } else { 
                let manual_entry = Confirm::new(
                    format!("Ticker {} was not publicly found. Would you like to enter its price manually?", ticker.clone())
                    .as_str())
                    .prompt()
                    .unwrap();

                if !manual_entry {
                    println!("Stock was not purchased!");
                    return None;
                }

                true
            }
        } else { 
            false
        };

        let pid = self.db.check_and_add_participant(
            self.uid,
            self.id,
            ticker.clone(),
            ParticipantType::Payee,
            false,
        );

        let date_msg = "Enter date of purchase:";
        let date_input = if defaults_to_use {
            let initial_date = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .date;
            DateSelect::new(date_msg)
                .with_default(NaiveDate::parse_from_str(&initial_date, "%Y-%m-%d").unwrap())
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string()
        } else {
            DateSelect::new(date_msg)
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string()
        };

        let shares_msg = "Enter number of shares purchased:";
        let shares: f32 = if defaults_to_use {
            CustomType::<f32>::new(shares_msg)
                .with_placeholder("0.00")
                .with_default(initial.info.shares)
                .with_error_message("Please enter a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(shares_msg)
                .with_placeholder("0.00")
                .with_default(0.00)
                .with_error_message("Please enter a valid amount!")
                .prompt()
                .unwrap()
        };

        let costbasis_msg = "Enter cost basis of shares purchased:";
        let costbasis: f32 = if defaults_to_use {
            CustomType::<f32>::new(costbasis_msg)
                .with_placeholder("0.00")
                .with_default(initial.info.costbasis)
                .with_error_message("Please enter a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(costbasis_msg)
                .with_placeholder("0.00")
                .with_default(0.00)
                .with_error_message("Please enter a valid amount!")
                .prompt()
                .unwrap()
        };

        let cid = self
            .db
            .check_and_add_category(self.uid, self.id, "buy".to_ascii_uppercase());

        purchase = LedgerInfo {
            date: date_input.clone(),
            amount: shares * costbasis,
            transfer_type: TransferType::WithdrawalToInternalAccount,
            participant: pid.clone(),
            category_id: cid,
            description: format!(
                "[Internal] Purchase {} shares of {} at ${} on {}.",
                shares,
                ticker,
                costbasis.clone(),
                date_input.clone()
            ),
            ancillary_f32data: 0.0,
        };

        let ledger_id = if defaults_to_use && overwrite_entry {
            self.db
                .update_ledger_item(
                    self.uid,
                    self.id,
                    LedgerRecord {
                        id: initial.info.ledger_id,
                        info: purchase.clone(),
                    },
                )
                .unwrap()
        } else {
            self.db
                .add_ledger_entry(self.uid, self.id, purchase.clone())
                .unwrap()
        };

        let stock_record = StockInfo {
            shares: shares,
            costbasis: costbasis,
            remaining: shares,
            ledger_id: ledger_id,
        };

        if manual_entry {
            let stock_price_info = StockPriceInfo {
                date: date_input.clone(),
                stock_ticker_peer_id: pid,
                price_per_unit_share: costbasis.clone(),
            };

            self.db
                .add_stock_price(self.uid, self.id, stock_price_info)
                .unwrap();
        }

        self.db
            .add_stock_purchase(self.uid, self.id, stock_record)
            .unwrap();

        self.initialize_buffer();

        return Some(LedgerRecord {
            id: ledger_id,
            info: purchase.clone(),
        });
    }

    pub fn sell_stock(
        &mut self,
        initial_opt: Option<StockRecord>,
        overwrite_entry: bool,
    ) -> Option<LedgerRecord> {
        let defaults_to_use: bool;
        let mut initial: StockRecord = StockRecord {
            id: 0,
            info: StockInfo {
                shares: 0.0,
                costbasis: 0.0,
                remaining: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };

        if initial_opt.is_some() {
            defaults_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            defaults_to_use = false;
        }

        let ticker: String;
        let ticker_msg = "Select which stock you would like to record a sale of:";
        let mut tickers_undup = self.db.get_stock_tickers(self.uid, self.id).unwrap();
        tickers_undup.sort();
        tickers_undup.dedup();
        let tickers = tickers_undup;
        ticker = if defaults_to_use {
            let pid = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .participant;
            let initial_ticker = self.db.get_participant(self.uid, self.id, pid).unwrap();
            Select::new(ticker_msg, tickers)
                .with_starting_filter_input(initial_ticker.as_str())
                .prompt()
                .unwrap()
                .to_string()
                .to_ascii_uppercase()
        } else {
            Select::new(ticker_msg, tickers)
                .prompt()
                .unwrap()
                .to_string()
                .to_ascii_uppercase()
        };

        let pid = self.db.check_and_add_participant(
            self.uid,
            self.id,
            ticker.clone(),
            ParticipantType::Payer,
            false,
        );

        let date_msg = "Enter date of sale:";
        let sale_date = if defaults_to_use {
            let initial_date = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .date;
            DateSelect::new(date_msg)
                .with_default(NaiveDate::parse_from_str(&initial_date, "%Y-%m-%d").unwrap())
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
        } else {
            DateSelect::new(date_msg)
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
        };

        let price_msg = "Enter sale price (per share):";
        let sale_price: f32 = if defaults_to_use {
            CustomType::<f32>::new(price_msg)
                .with_placeholder("00000.00")
                .with_default(initial.info.costbasis)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(price_msg)
                .with_placeholder("00000.00")
                .with_default(00000.00)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        };

        let shares_msg = "Enter quantity sold:";
        let number_of_shares_sale: f32 = if defaults_to_use {
            CustomType::<f32>::new(shares_msg)
                .with_placeholder("00000.00")
                .with_default(initial.info.shares)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(shares_msg)
                .with_placeholder("00000.00")
                .with_default(00000.00)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        };

        let value_received = number_of_shares_sale * sale_price;
        let stock_cid =
            self.db
                .check_and_add_category(self.uid, self.id, "sale".to_ascii_uppercase());

        let sale = LedgerInfo {
            date: sale_date.to_string(),
            amount: value_received,
            transfer_type: TransferType::DepositFromInternalAccount,
            participant: pid,
            category_id: stock_cid,
            description: format!(
                "[Internal]: Sell {} shares of {} at ${} on {}.",
                number_of_shares_sale,
                ticker,
                sale_price,
                sale_date.to_string()
            ),
            ancillary_f32data: 0.0,
        };

        let ledger_id: u32 = if defaults_to_use && overwrite_entry {
            self.db
                .update_ledger_item(
                    self.uid,
                    self.id,
                    LedgerRecord {
                        id: initial.info.ledger_id,
                        info: sale.clone(),
                    },
                )
                .unwrap()
        } else {
            self.db
                .add_ledger_entry(self.uid, self.id, sale.clone())
                .unwrap()
        };

        let sale_record = StockInfo {
            shares: number_of_shares_sale,
            costbasis: sale_price,
            remaining: 0.0,
            ledger_id: ledger_id,
        };

        let sale_id = self
            .db
            .add_stock_sale(self.uid, self.id, sale_record.clone())
            .unwrap();

        let sale_info = StockRecord {
            id: sale_id,
            info: sale_record.clone(),
            txn_opt: Some(sale.clone()),
        };

        const SALE_METHOD_OPTIONS: [&'static str; 2] = ["LIFO", "FIFO"];
        let sell_method: String =
            Select::new("Select sale methodology:", SALE_METHOD_OPTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();

        self.allocate_sale_stock(sale_info, sell_method);
        self.initialize_buffer();

        return Some(LedgerRecord {
            id: ledger_id,
            info: sale.clone(),
        });
    }

    pub fn split_stock(
        &mut self,
        initial_opt: Option<StockSplitRecord>,
        overwrite_entry: bool,
    ) -> Option<LedgerRecord> {
        let defaults_to_use: bool;
        let mut initial: StockSplitRecord = StockSplitRecord {
            id: 0,
            info: StockSplitInfo {
                split: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };

        if initial_opt.is_some() {
            defaults_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            defaults_to_use = false;
        }

        let mut tickers_undup = self.db.get_stock_tickers(self.uid, self.id).unwrap();
        tickers_undup.sort();
        tickers_undup.dedup();
        tickers_undup.push("None".to_string());
        let mut tickers = tickers_undup;
        let ticker_msg = "Select which stock you would like to report a split of:";
        let ticker = if defaults_to_use {
            let pid = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .participant;
            let initial_ticker: String = self.db.get_participant(self.uid, self.id, pid).unwrap();
            Select::new(ticker_msg, tickers)
                .with_starting_filter_input(&initial_ticker.as_str())
                .prompt()
                .unwrap()
                .to_string()
        } else {
            Select::new(ticker_msg, tickers)
                .prompt()
                .unwrap()
                .to_string()
        };

        if ticker == "None" {
            return None;
        }

        let split_msg = "Enter split factor:";
        let split: f32 = if defaults_to_use {
            CustomType::<f32>::new(split_msg)
                .with_default(initial.info.split)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(split_msg)
                .with_placeholder("2.0")
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        };

        let date_msg = "Enter date of split:";
        let split_date = if defaults_to_use {
            let initial_date = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .date;
            DateSelect::new(date_msg)
                .with_starting_date(
                    NaiveDate::parse_from_str(initial_date.as_str(), "%Y-%m-%d")
                        .expect("Unable to convert date to NaiveDate"),
                )
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string()
        } else {
            DateSelect::new(date_msg)
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string()
        };

        let pid = self.db.check_and_add_participant(
            self.uid,
            self.id,
            ticker.clone(),
            ParticipantType::Both,
            false,
        );
        let cid = self.db.check_and_add_category(
            self.uid,
            self.id,
            "stock dividend/split".to_ascii_uppercase(),
        );

        // if split is for a manually entered stock,
        // then all previous stock price records
        // need to be updated
        let price_records = self
            .db
            .check_and_get_stock_price_record_matching_from_participant_id(self.uid, self.id, pid)
            .unwrap();
        if !price_records.is_empty() {
            self.db
                .apply_stock_split_to_stock_prices(self.uid, self.id, pid, split);
        }

        let ledger_entry = LedgerInfo {
            date: split_date.clone(),
            amount: 0.0,
            transfer_type: TransferType::ZeroSumChange,
            participant: pid,
            category_id: cid,
            description: format!(
                "[Internal]: Split of {} by factor of {} on {}.",
                ticker.clone(),
                split.clone(),
                split_date
            ),
            ancillary_f32data: self
                .db
                .get_stocks(self.uid, self.id, ticker.clone())
                .unwrap()
                .iter()
                .map(|rcrd| rcrd.info.remaining)
                .sum::<f32>()
                * split,
        };

        let lid = if defaults_to_use && overwrite_entry {
            self.db
                .update_ledger_item(
                    self.uid,
                    self.id,
                    LedgerRecord {
                        id: initial.info.ledger_id,
                        info: ledger_entry.clone(),
                    },
                )
                .unwrap()
        } else {
            self.db
                .add_ledger_entry(self.uid, self.id, ledger_entry.clone())
                .unwrap()
        };

        let stock_split_id = self
            .db
            .add_stock_split(self.uid, self.id, split.clone(), lid)
            .unwrap();

        let stock_split_record = StockSplitRecord {
            id: stock_split_id,
            info: StockSplitInfo {
                split: split.clone(),
                ledger_id: lid.clone(),
            },
            txn_opt: Some(ledger_entry.clone()),
        };

        self.allocate_stock_split(stock_split_record);
        self.initialize_buffer();

        return Some(LedgerRecord {
            id: lid,
            info: ledger_entry,
        });
    }

    pub fn modify(&mut self, record: LedgerRecord) -> Option<LedgerRecord> {
        let was_stock_purchase_opt = self
            .db
            .check_and_get_stock_purchase_record_matching_from_ledger_id(
                self.uid, self.id, record.id,
            )
            .unwrap();
        let was_stock_sale_opt = self
            .db
            .check_and_get_stock_sale_record_matching_from_ledger_id(self.uid, self.id, record.id)
            .unwrap();
        let was_stock_split_opt = self
            .db
            .check_and_get_stock_split_record_matching_from_ledger_id(self.uid, self.id, record.id)
            .unwrap();

        let mut is_stock_purchase: bool = false;
        let mut is_stock_sale: bool = false;
        let mut is_stock_split: bool = false;
        let mut stock_record: StockRecord = StockRecord {
            id: 0,
            info: StockInfo {
                shares: 0.0,
                costbasis: 0.0,
                remaining: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };
        let mut split_record: StockSplitRecord = StockSplitRecord {
            id: 0,
            info: StockSplitInfo {
                split: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };
        if was_stock_purchase_opt.is_none()
            && was_stock_sale_opt.is_none()
            && was_stock_split_opt.is_none()
        {
            return Some(self.fixed.modify(record));
        }

        if was_stock_purchase_opt.is_some() {
            is_stock_purchase = true;
            stock_record = was_stock_purchase_opt.unwrap();
            stock_record.txn_opt = Some(record.info.clone());
        } else if was_stock_sale_opt.is_some() {
            is_stock_sale = true;
            stock_record = was_stock_sale_opt.unwrap();
            stock_record.txn_opt = Some(record.info.clone());
        } else {
            is_stock_split = true;
            split_record = was_stock_split_opt.unwrap();
            split_record.txn_opt = Some(record.info.clone());
        }

        const OPTIONS: [&'static str; 3] = ["Update", "Remove", "None"];
        let modify_choice = Select::new("What would you like to do:", OPTIONS.to_vec())
            .prompt()
            .unwrap();
        match modify_choice {
            "Update" => {
                if is_stock_purchase {
                    self.db
                        .remove_stock_purchase(self.uid, self.id, stock_record.id)
                        .unwrap();
                    return self.purchase_stock(Some(stock_record), true);
                } else if is_stock_sale {
                    self.deallocate_sale_stock(stock_record.id);
                    self.db
                        .remove_stock_sale(self.uid, self.id, stock_record.info.ledger_id)
                        .unwrap();
                    return self.sell_stock(Some(stock_record), true);
                } else {
                    // split stock
                    self.deallocate_stock_split(split_record.clone());
                    return self.split_stock(Some(split_record.clone()), true);
                }
            }
            "Remove" => {
                if is_stock_purchase {
                    self.db
                        .remove_ledger_item(self.uid, self.id, stock_record.info.ledger_id)
                        .unwrap();
                } else if is_stock_sale {
                    self.deallocate_sale_stock(stock_record.clone().id);
                    self.db
                        .remove_ledger_item(self.uid, self.id, stock_record.info.ledger_id)
                        .unwrap();
                } else {
                    self.deallocate_stock_split(split_record.clone());
                    self.db
                        .remove_ledger_item(self.uid, self.id, split_record.info.ledger_id)
                        .unwrap();
                }
                return Some(record);
            }
            "None" => {
                return Some(record);
            }
            _ => {
                panic!("Input not recognized!");
            }
        }
    }

    pub fn allocate_sale_stock(&self, record: StockRecord, method: String) {
        let stocks: Vec<StockRecord>;
        let ticker = self
            .db
            .get_participant(
                self.uid,
                self.id,
                record
                    .txn_opt
                    .expect("Transaction required but not found!")
                    .participant,
            )
            .unwrap();
        match method.as_str() {
            "LIFO" => {
                stocks = self
                    .db
                    .get_stock_history_ascending(self.uid, self.id, ticker)
                    .unwrap();
            }
            "FIFO" => {
                stocks = self
                    .db
                    .get_stock_history_descending(self.uid, self.id, ticker)
                    .unwrap();
            }
            _ => {
                panic!("Unrecognized input!");
            }
        }

        let mut num_shares_remaining_to_allocate = record.info.shares;
        let mut num_shares_allocated: f32;
        for mut stock in stocks {
            let purchase_id = stock.id;

            // can't sell what you don't have
            if stock.info.remaining == 0.0 {
                continue;
            }

            if stock.info.remaining > num_shares_remaining_to_allocate {
                stock.info.remaining = stock.info.remaining - num_shares_remaining_to_allocate;
                num_shares_allocated = num_shares_remaining_to_allocate;
            } else {
                num_shares_allocated = stock.info.remaining;
                stock.info.remaining = 0.0;
            }
            self.db
                .update_stock_remaining(self.uid, self.id, stock.id, stock.info.remaining)
                .unwrap();
            self.db
                .add_stock_sale_allocation(
                    self.uid,
                    self.id,
                    purchase_id,
                    record.id,
                    num_shares_allocated,
                )
                .unwrap();
            num_shares_remaining_to_allocate =
                num_shares_remaining_to_allocate - num_shares_allocated;

            // if there are no shares to allocate, we are done here and all sales
            // are accounted for
            if num_shares_remaining_to_allocate == 0.0 {
                break;
            }
        }
    }

    fn deallocate_sale_stock(&self, sale_id: u32) {
        let stock_allocation_records = self
            .db
            .get_stock_sale_allocation_for_sale_id(self.uid, self.id, sale_id)
            .unwrap();
        for record in stock_allocation_records {
            // add shares back to ledger
            let _ = self
                .db
                .add_to_stock_remaining(
                    self.uid,
                    self.id,
                    record.info.purchase_id,
                    record.info.quantity,
                )
                .unwrap();
            self.db
                .remove_stock_sale_allocation(self.uid, self.id, record.id);
        }
    }

    pub fn allocate_stock_split(&self, record: StockSplitRecord) {
        if record.txn_opt.is_none() {
            panic!(
                "Expected ledger data matching stock split id: {}",
                record.id
            );
        }
        let split_txn = record.txn_opt.unwrap();

        let ticker = self
            .db
            .get_participant(self.uid, self.id, split_txn.participant)
            .unwrap();
        let stock_purchase_records = self.db.get_stocks(self.uid, self.id, ticker).unwrap();

        let mut sales_to_update: Vec<(u32, f32)> = Vec::new();
        for stock in stock_purchase_records {
            // update shares so it looks like we have always purchased those
            self.db
                .update_stock_shares_purchased(
                    self.uid,
                    self.id,
                    stock.id,
                    stock.info.shares * record.info.split,
                )
                .unwrap();
            self.db
                .update_stock_remaining(
                    self.uid,
                    self.id,
                    stock.id,
                    stock.info.remaining * record.info.split,
                )
                .unwrap();
            self.db
                .update_cost_basis(
                    self.uid,
                    self.id,
                    stock.id,
                    stock.info.costbasis / record.info.split,
                )
                .unwrap();
            self.db
                .add_stock_split_allocation(
                    self.uid,
                    self.id,
                    StockSplitAllocationInfo {
                        stock_split_id: record.id,
                        stock_purchase_id: stock.id,
                    },
                )
                .unwrap();

            // if stock was part of sale, we need to increase number of stocks sold by factor
            let sale_allocations_opt = self
                .db
                .check_and_get_stock_sale_allocation_record_matching_from_purchase_id(
                    self.uid, self.id, stock.id,
                )
                .unwrap();
            if sale_allocations_opt.is_none() {
                // no sale allocations founds
                continue;
            }
            let sale_allocations = sale_allocations_opt.unwrap();
            for sale_allocation in sale_allocations {
                let sale_txn_opt = self
                    .db
                    .check_and_get_stock_sale_record_matching_from_sale_id(
                        self.uid,
                        self.id,
                        sale_allocation.info.sale_id,
                    )
                    .unwrap();
                if sale_txn_opt.is_none() {
                    panic!(
                        "Stock stale record not found for sale id: {}",
                        sale_allocation.info.sale_id
                    );
                }
                let stock_sale = sale_txn_opt.unwrap();
                if stock_sale.txn_opt.is_none() {
                    panic!(
                        "Transaction is missing with sale transaction matching id: {}",
                        stock_sale.id
                    );
                }
                let sale_txn: LedgerInfo = stock_sale.txn_opt.unwrap();
                // if the sale occured after the split, ignore it.
                if sale_txn.date > split_txn.date {
                    continue;
                }
                self.db
                    .update_stock_sale_allocation_quantity(
                        self.uid,
                        self.id,
                        sale_allocation.id,
                        sale_allocation.info.quantity * record.info.split,
                    )
                    .unwrap();
                sales_to_update.push((stock_sale.id, stock_sale.info.shares));
            }
        }

        if !sales_to_update.is_empty() {
            sales_to_update.sort_by(|a, b| (a.0).cmp(&b.0));
            sales_to_update.dedup_by(|a, b| a.0 == b.0);
            for sale in sales_to_update {
                self.db
                    .update_stock_shares_sold(self.uid, self.id, sale.0, sale.1 * record.info.split)
                    .unwrap();
            }
        }
    }

    fn deallocate_stock_split(&self, record: StockSplitRecord) {
        let mut stock_split_alloc_records = self
            .db
            .get_stock_split_allocation_for_stock_split_id(self.uid, self.id, record.id)
            .unwrap();
        // remove the highest ids first
        stock_split_alloc_records.sort_by(|a, b| (b.id).cmp(&a.id));

        if record.txn_opt.is_none() {
            panic!(
                "Expected ledger data matching stock split id: {}",
                record.id
            );
        }
        let split_txn = record.txn_opt.unwrap();

        let mut sales_to_update = Vec::new();

        for alloc_record in stock_split_alloc_records {
            // add shares back to ledger
            let stock_purchase = self
                .db
                .check_and_get_stock_purchase_record_matching_from_purchase_id(
                    self.uid,
                    self.id,
                    alloc_record.info.stock_purchase_id,
                )
                .unwrap()
                .expect("Stock record not returned");

            let updated_shares = stock_purchase.info.remaining / record.info.split;
            let updated_costbasis = stock_purchase.info.costbasis * record.info.split;

            let _ = self.db.update_stock_remaining(
                stock_purchase.id,
                self.id,
                alloc_record.info.stock_purchase_id,
                updated_shares,
            );
            let _ = self.db.update_stock_shares_purchased(
                self.uid,
                self.id,
                stock_purchase.id,
                updated_shares,
            );
            let _ = self.db.update_cost_basis(
                self.uid,
                self.id,
                alloc_record.info.stock_purchase_id,
                updated_costbasis,
            );

            // check if there have been any sales affected by this stock that would be affected by this split
            let sale_allocations_opt = self
                .db
                .check_and_get_stock_sale_allocation_record_matching_from_purchase_id(
                    self.uid,
                    self.id,
                    alloc_record.info.stock_purchase_id,
                )
                .unwrap();
            if sale_allocations_opt.is_some() {
                let sale_allocations = sale_allocations_opt.unwrap();
                for sale_allocation in sale_allocations {
                    let stock_sale_opt = self
                        .db
                        .check_and_get_stock_sale_record_matching_from_sale_id(
                            self.uid,
                            self.id,
                            sale_allocation.info.sale_id,
                        )
                        .unwrap();
                    if stock_sale_opt.is_none() {
                        panic!(
                            "Stock stale record not found for sale id: {}",
                            sale_allocation.info.sale_id
                        );
                    }
                    let stock_sale = stock_sale_opt.unwrap();
                    if stock_sale.txn_opt.is_none() {
                        panic!(
                            "Transaction is missing with sale transaction matching id: {}",
                            stock_sale.id
                        );
                    }
                    let sale_txn: LedgerInfo = stock_sale.txn_opt.unwrap();

                    // if the sale occured after the split, ignore it.
                    if sale_txn.date > split_txn.date {
                        continue;
                    }
                    self.db
                        .update_stock_sale_allocation_quantity(
                            self.uid,
                            self.id,
                            sale_allocation.id,
                            sale_allocation.info.quantity / record.info.split,
                        )
                        .unwrap();
                    sales_to_update.push((stock_sale.id, stock_sale.info.shares));
                }
            }
            self.db
                .remove_stock_split_allocation(self.uid, self.id, alloc_record.id)
                .unwrap();
        }
        if !sales_to_update.is_empty() {
            sales_to_update.sort_by(|a, b| (a.0).cmp(&b.0));
            sales_to_update.dedup_by(|a, b| a.0 == b.0);
            for sale in sales_to_update {
                self.db
                    .update_stock_shares_sold(self.uid, self.id, sale.0, sale.1 / record.info.split)
                    .unwrap();
            }
        }
        self.db
            .remove_stock_split(self.uid, self.id, record.info.ledger_id)
            .unwrap();
    }

    pub fn confirm_public_ticker(&self, ticker: String) -> bool {
        let rs = stocks::get_stock_at_close(ticker.clone());
        match rs {
            Ok(price) => true,
            Err(error) => {
                // panic!("Fetch failed for ticker '{}': {}!", ticker.clone(), error);
                false
            }
        }
    }

    pub fn get_current_value(&self) -> f32 {
        let today = Local::now().date_naive();
        return self
            .db
            .get_cumulative_total_of_ledger_on_date(self.uid, self.id, today)
            .unwrap()
            .unwrap()
            + self.get_value_of_positions_on_day(&today);
    }

    pub fn time_weighted_return(&self, period_start: NaiveDate, period_end: NaiveDate) -> f32 {
        let mut cf: f32 = 0.0;
        let mut hps: Vec<f32> = Vec::new();
        let mut hp: f32;
        let mut vf;
        let mut vi;
        let mut rate = 0.0;

        // println!("Beginning of analysis: {} - {}", period_start, period_end);

        let starting_fixed_value_opt = self
            .db
            .get_cumulative_total_of_ledger_on_date(self.uid, self.id, period_start)
            .unwrap();

        let starting_fixed_value;
        if starting_fixed_value_opt.is_some() {
            starting_fixed_value = starting_fixed_value_opt.unwrap();
        } else {
            return f32::NAN;
        }

        let starting_variable_value = self.get_value_of_positions_on_day(
            &period_start
                .checked_sub_days(Days::new(1))
                .expect("Invalid date!"),
        );

        vi = starting_fixed_value + starting_variable_value;

        // println!("Day: {}, Fixed: {}, Variable: {}", period_start.to_string(), starting_fixed_value, starting_variable_value);

        let external_transactions = Some(
            self.db
                .get_ledger_entries_within_timestamps(self.uid, self.id, period_start, period_end)
                .unwrap(),
        );
        if let Some(transactions) = external_transactions {
            if !transactions.is_empty() {
                vf = 0.0;
                for txn in transactions {
                    let end_period = NaiveDate::parse_from_str(&txn.info.date, "%Y-%m-%d")
                        .expect(format!("Invalid date format: {}", txn.info.date).as_str());
                    cf = match txn.info.transfer_type {
                        TransferType::DepositFromExternalAccount => txn.info.amount,
                        TransferType::WithdrawalToExternalAccount => -txn.info.amount,
                        _ => 0.0,
                    };
                    let final_fixed_value_opt = self
                        .db
                        .get_cumulative_total_of_ledger_on_date(self.uid, self.id, end_period)
                        .unwrap();
                    let final_fixed_value;
                    if final_fixed_value_opt.is_some() {
                        final_fixed_value = final_fixed_value_opt.unwrap();
                    } else {
                        return f32::NAN;
                    }

                    let final_variable_value = self.get_value_of_positions_on_day(&end_period);
                    // println!("Day: {}, Fixed: {}, Variable: {}", end_period.to_string(), final_fixed_value, final_variable_value);
                    vf = final_fixed_value + final_variable_value;
                    hp = (vf - (cf + vi)) / (cf + vi);
                    hps.push(hp);

                    vi = vf;
                }
            }
        }

        let final_fixed_value_opt = self
            .db
            .get_cumulative_total_of_ledger_on_date(self.uid, self.id, period_end)
            .unwrap();
        let final_fixed_value;
        if final_fixed_value_opt.is_some() {
            final_fixed_value = final_fixed_value_opt.unwrap();
        } else {
            return f32::NAN;
        }

        let final_variable_value = self.get_value_of_positions_on_day(&period_end);
        // println!("Day: {}, Fixed: {}, Variable: {}", period_end.to_string(), final_fixed_value, final_variable_value);
        vf = final_fixed_value + final_variable_value;
        hp = (vf - vi) / vi;
        hps.push(hp);

        let hp1 = hps.pop().expect("No valid cash flow periods!");
        let twr = hps.iter().fold(1.0 + hp1, |acc, hp| acc * (1.0 + hp)) - 1.0;
        rate = twr * 100.0;

        return rate;
    }

    pub fn get_positions(&self) -> Option<Vec<(String, f32)>> {
        return self.db.get_positions(self.uid, self.id).unwrap();
    }

    pub fn get_value_of_positions_on_day(&self, day: &NaiveDate) -> f32 {
        let mut value: f32 = 0.0;
        if let Some(buffer) = self.buffer.as_ref() {
            for e in buffer {
                let mut owned_shares = e
                    .history
                    .iter()
                    .filter(|x| x.date <= *day)
                    .collect::<Vec<&SharesOwned>>();
                if owned_shares.is_empty() {
                    // if no shares owned before date, then just continue 0
                    continue;
                }
                owned_shares.sort_by(|x, y| (x.date).cmp(&y.date));
                let most_recently_owned = owned_shares.last().unwrap();
                let quotes = e
                    .quotes
                    .iter()
                    .filter(|x| {
                        let date = OffsetDateTime::from_unix_timestamp(x.timestamp as i64)
                            .unwrap()
                            .date();
                        let ndate = NaiveDate::from_ymd_opt(
                            date.year(),
                            date.month() as u32,
                            date.day() as u32,
                        )
                        .unwrap();
                        ndate < *day
                    })
                    .collect::<Vec<&Quote>>();
                let quote_opt = quotes.last();
                if quote_opt.is_none() {
                    continue;
                }
                let quote = quote_opt.unwrap();
                let partial_value = (quote.close * most_recently_owned.shares as f64) as f32;
                value = value + partial_value
            }
        }
        return value;
    }

    pub fn get_account_value_on_day(&self, day: &NaiveDate) -> Option<f32> {
        let mut value: f32 = 0.0;
        if let Some(buffer) = self.buffer.as_ref() {
            for e in buffer {
                let mut owned_shares = e
                    .history
                    .iter()
                    .filter(|x| x.date <= *day)
                    .collect::<Vec<&SharesOwned>>();
                if owned_shares.is_empty() {
                    // if no shares owned before date, then just continue 0
                    continue;
                }
                owned_shares.sort_by(|x, y| (x.date).cmp(&y.date));
                let most_recently_owned = owned_shares.last().unwrap();
                let quote = e
                    .quotes
                    .iter()
                    .filter(|x| {
                        let date = OffsetDateTime::from_unix_timestamp(x.timestamp as i64)
                            .unwrap()
                            .date();
                        let ndate = NaiveDate::from_ymd_opt(
                            date.year(),
                            date.month() as u32,
                            date.day() as u32,
                        )
                        .unwrap();
                        ndate < *day
                    })
                    .last()
                    .expect(
                        format!("No quote matching date {}", most_recently_owned.date).as_str(),
                    );
                let partial_value = (quote.close * most_recently_owned.shares as f64) as f32;
                // println!("\tTicker: {}, Shares: {}, Price: {}, Total : {}", e.ticker, most_recently_owned.shares, quote.close, partial_value);
                // println!("\t\tMost recent date: {}", OffsetDateTime::from_unix_timestamp(quote.timestamp as i64).unwrap().date());
                value = value + partial_value
            }
        }
        let fixed_value = self
            .db
            .get_cumulative_total_of_ledger_on_date(self.uid, self.id, *day)
            .unwrap();
        if let Some(fixed) = fixed_value {
            value = value + fixed;
        } else {
            return None;
        }
        return Some(value);
    }

    pub fn manually_record_stock_close_price(&self) {
        let ticker = Text::new("What ticker are you recording for?")
            .with_autocomplete(ParticipantAutoCompleter {
                uid: self.uid,
                aid: self.id,
                db: self.db.clone(),
                ptype: ParticipantType::Payee,
                with_accounts: false,
                manually_recorded_only: true,
            })
            .prompt()
            .unwrap();

        let peer_id = self.db.check_and_add_participant(
            self.uid,
            self.id,
            ticker.clone(),
            ParticipantType::Payee,
            false,
        );

        let date = DateSelect::new("Enter date to record:").prompt().unwrap();
        let close_price = CustomType::<f32>::new(
            format!(
                "Enter close price per unit share on {}:",
                date.to_string().clone()
            )
            .as_str(),
        )
        .prompt()
        .unwrap();
        let info: StockPriceInfo = StockPriceInfo {
            stock_ticker_peer_id: peer_id,
            price_per_unit_share: close_price,
            date: date.to_string(),
        };
        self.db.add_stock_price(self.uid, self.id, info).unwrap();
    }

    fn convert_stock_price_record_to_quotes(
        stock_prices: &Vec<StockPriceRecord>,
    ) -> Vec<yahoo_finance_api::Quote> {
        let mut quotes: Vec<yahoo_finance_api::Quote> = Vec::new();
        for r in stock_prices {
            let timestamp = NaiveDate::parse_from_str(&r.info.date, "%Y-%m-%d")
                .unwrap()
                .and_time(NaiveTime::from_num_seconds_from_midnight_opt(0, 0).unwrap())
                .and_utc()
                .timestamp() as u64;

            quotes.push(Quote {
                timestamp: timestamp,
                // right now, the user doesn't store this information in the database
                // because it might not be available, so set it to the known value.
                open: r.info.price_per_unit_share as f64,
                high: r.info.price_per_unit_share as f64,
                low: r.info.price_per_unit_share as f64,
                close: r.info.price_per_unit_share as f64,
                volume: 0,
                adjclose: r.info.price_per_unit_share as f64,
            });
        }
        return quotes;
    }
}
