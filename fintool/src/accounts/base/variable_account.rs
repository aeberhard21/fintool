use core::alloc;
use std::backtrace;
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

use chrono::{Date, Days, Local, NaiveDate, NaiveDateTime};
use csv::DeserializeError;
use inquire::*;
use time::OffsetDateTime;
use yahoo_finance_api::YahooError;

use crate::accounts::base::{SharesOwned, StockData};
use crate::database::DbConn;
use crate::stocks::{self, get_stock_history};
use crate::types::investments::{
    SaleAllocationInfo, SaleAllocationRecord, StockInfo, StockRecord, StockSplitAllocationInfo,
    StockSplitInfo, StockSplitRecord,
};
use crate::types::ledger::{LedgerInfo, LedgerRecord};
use crate::types::participants::ParticipantType;
use shared_lib::{LedgerEntry, TransferType};

use super::fixed_account::FixedAccount;

pub struct VariableAccount {
    pub id: u32,
    pub uid: u32,
    pub db: DbConn,
    pub fixed: FixedAccount,
    pub buffer : Option<Vec<StockData>>,
}

impl VariableAccount {
    pub fn new(uid: u32, id: u32, db: &DbConn) -> Self {
        let mut acct: VariableAccount = Self {
            id: id,
            uid: uid,
            db: db.clone(),
            fixed: FixedAccount::new(uid, id, db.clone()),
            buffer : None
        };

        let mut ledger = acct.db.get_ledger(acct.uid, acct.id).unwrap();
        ledger.sort_by(|l1, l2| (&l1.info.date).cmp(&l2.info.date));
        let earliest_date = NaiveDate::parse_from_str(&ledger[0].info.date, "%Y-%m-%d").unwrap();
        let latest_date = Local::now().date_naive();

        let x = acct.db.get_positions_by_ledger(id, uid).unwrap();
        if x.is_some() { 
             let x = x.unwrap();
             let mut tickers = x.iter().map(|x| x.0.clone()).collect::<Vec<String>>();
             tickers.dedup();
             let mut data : Vec<StockData> = Vec::new();
             for ticker in tickers { 
                let date_shares = x.iter().filter(|data| data.0 == ticker).map(|x: &(String, String, f32)| (SharesOwned { date : NaiveDate::parse_from_str(&&x.1, "%Y-%m-%d").expect(format!("Unable to decode {}", &x.1).as_str()), shares :  x.2.clone()})).collect::<Vec<SharesOwned>>();
                let quotes = get_stock_history(ticker.clone(), earliest_date, latest_date).unwrap();
                data.push(StockData { ticker: ticker.clone(), quotes: quotes, history: date_shares });

             }
             acct.buffer = Some(data);
        }

        acct
    }

    pub fn purchase_stock(
        &self,
        initial_opt: Option<StockRecord>,
        overwrite_entry: bool,
    ) -> LedgerRecord {
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
        let ticker = if defaults_to_use {
            let pid = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .participant;
            let initial_ticker = self.db.get_participant(self.uid, self.id, pid).unwrap();
            Text::new(ticker_msg)
                .with_default(initial_ticker.as_str())
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string()
        } else {
            Text::new(ticker_msg)
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string()
        };

        let ticker_valid = self.confirm_valid_ticker(ticker.clone());
        if ticker_valid == false {
            panic!("Invalid stock ticker entered!");
        }

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

        let pid = self.db.check_and_add_participant(
            self.uid,
            self.id,
            ticker.clone(),
            ParticipantType::Payee,
            false,
        );
        let cid = self
            .db
            .check_and_add_category(self.uid, self.id, "buy".to_ascii_uppercase());

        purchase = LedgerInfo {
            date: date_input.clone(),
            amount: shares * costbasis,
            transfer_type: TransferType::WithdrawalToInternalAccount,
            participant: pid,
            category_id: cid,
            description: format!(
                "[Internal] Purchase {} shares of {} at ${} on {}.",
                shares,
                ticker,
                costbasis,
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

        self.db
            .add_stock_purchase(self.uid, self.id, stock_record)
            .unwrap();

        return LedgerRecord {
            id: ledger_id,
            info: purchase.clone(),
        };
    }

    pub fn sell_stock(
        &self,
        initial_opt: Option<StockRecord>,
        overwrite_entry: bool,
    ) -> LedgerRecord {
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

        return LedgerRecord {
            id: ledger_id,
            info: sale.clone(),
        };
    }

    pub fn split_stock(
        &self,
        initial_opt: Option<StockSplitRecord>,
        overwrite_entry: bool,
    ) -> LedgerRecord {
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
        let tickers = tickers_undup;
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

        return LedgerRecord {
            id: lid,
            info: ledger_entry,
        };
    }

    pub fn modify(&self, record: LedgerRecord) -> LedgerRecord {
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
            return self.fixed.modify(record);
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
                return record;
            }
            "None" => {
                return record;
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

    pub fn confirm_valid_ticker(&self, ticker: String) -> bool {
        let rs = stocks::get_stock_at_close(ticker.clone());
        match rs {
            Ok(price) => true,
            Err(error) => {
                panic!("Fetch failed for ticker '{}': {}!", ticker.clone(), error);
            }
        }
    }

    pub fn get_current_value(&self) -> f32 {
        let fixed_value = self.fixed.get_current_value();
        let variable_value = self.db.get_stock_current_value(self.uid, self.id).unwrap();
        return fixed_value + variable_value;
    }

    pub fn time_weighted_return(&self, period_start: NaiveDate, period_end: NaiveDate) -> f32 {
        let mut cf: f32 = 0.0;
        let mut hps: Vec<f32> = Vec::new();
        let mut hp: f32;

        let fixed_transactions = self
            .db
            .get_ledger_entries_within_timestamps(self.uid, self.id, period_start, period_end)
            .unwrap();
        let mut iter = fixed_transactions.iter().peekable();

        // calculate value before date
        let fixed_value_opt = self
            .db
            .get_cumulative_total_of_ledger_before_date(
                self.uid,
                self.id,
                period_start
                    .checked_sub_days(Days::new(1))
                    .expect("Invalid date!"),
            )
            .unwrap();
        let fixed_value;
        if fixed_value_opt.is_some() { 
            fixed_value = fixed_value_opt.unwrap();
        } else { 
            return f32::NAN;
        }
        let variable_value_opt = self
            .db
            .get_portfolio_value_before_date(self.uid, self.id, period_start)
            .unwrap();
        let variable_value;
        if variable_value_opt.is_some() { 
            variable_value = variable_value_opt.unwrap();
        } else { 
            return f32::NAN;
        }
        let mut vi = fixed_value + variable_value;
        let mut vf_variable: f32 = 0.0;
        let mut vf: f32;

        let final_fixed_value_opt = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.uid, self.id, period_end)
            .unwrap();
        let final_fixed_value;
        if final_fixed_value_opt.is_some() { 
            final_fixed_value = final_fixed_value_opt.unwrap();
        } else { 
            return f32::NAN;
        }
        let final_portfolio_value_opt = self
            .db
            .get_portfolio_value_before_date(self.uid, self.id, period_end)
            .unwrap();
        let final_portfolio_value;
        if final_portfolio_value_opt.is_some() { 
            final_portfolio_value = final_portfolio_value_opt.unwrap();
        } else { 
            return f32::NAN;
        }
        let final_vf = final_fixed_value + final_portfolio_value;
        

        if iter.peek().is_none() {
            // no transactions during analyzed period, so
            // calculate regular growth rate
            hp = (final_vf - vi) / (vi);
            return ((1.0 + hp) - 1.0) * 100 as f32;
        }

        while let Some(txn) = iter.next() {
            let tt: &TransferType = &txn.info.transfer_type;
            cf += match tt {
                // positive cash flow
                TransferType::DepositFromExternalAccount => txn.info.amount,
                // all cash stays within account so cash flow is 0
                TransferType::DepositFromInternalAccount => 0.0,
                // withdrawing from account to cash flow is negative
                TransferType::WithdrawalToExternalAccount => -txn.info.amount,
                // all cash stays within account so cash flow is 0
                TransferType::WithdrawalToInternalAccount => 0.0,
                // Cash flow is zero on zero-sum change
                TransferType::ZeroSumChange => 0.0,
            };

            if iter.peek().is_none() {
                // no more left so calculate with account value at end of analysis period
                vf = final_vf;
                hp = (vf - (cf + vi)) / (cf + vi);
                hps.push(hp);
            } else {
                let nxt = iter.peek().expect("Item found, but not available");
                // in the event that there is a second transaction in this period, we will add this all to the cash flow
                if nxt.info.date == txn.info.date {
                    continue;
                };

                // next transaction is for a new period, so we can calculate the Hp and reset
                let end_of_period: NaiveDate =
                    NaiveDate::parse_from_str(&txn.info.date.as_str(), "%Y-%m-%d")
                        .expect("Invalid date!");
                let vf_fixed;
                let vf_fixed_opt = self
                    .db
                    .get_cumulative_total_of_ledger_before_date(self.uid, self.id, end_of_period)
                    .unwrap(); 
                if vf_fixed_opt.is_some() { 
                    vf_fixed = vf_fixed_opt.unwrap();
                } else { 
                    return f32::NAN;
                }
                let vf_variable_wrap =
                    self.db
                        .get_portfolio_value_before_date(self.uid, self.id, end_of_period);
                vf_variable = match vf_variable_wrap {
                    Ok(Some(amt)) => amt,
                    Ok(None) => {return f32::NAN;}
                    Err(error) => {
                        // if an error was returned we will just skip this day and move on
                        vf_variable
                    }
                };
                vf = vf_fixed + vf_variable;

                hp = (vf - (cf + vi)) / (cf + vi);
                hps.push(hp);

                cf = 0.0;
            }

            vi = vf;
        }
        let hp1 = hps.pop().expect("No valid cash flow periods!");
        let twr = hps.iter().fold(1.0 + hp1, |acc, hp| acc * (1.0 + hp)) - 1.0;
        return twr * 100.0;
    }

    pub fn get_positions(&self) -> Option<Vec<(String, f32)>> {
        return self.db.get_positions(self.uid, self.id).unwrap();
    }

    pub fn get_value_of_positions_on_day(&self, day : &String) -> f32 { 
        let mut value: f32 = 0.0;
        if let Some(buffer) = self.buffer.as_ref() { 
            for e in buffer { 
                // println!("Ticker: {}", e.ticker);
                let mut owned_shares = e.history.iter().filter(|x| { x.date <= NaiveDate::parse_from_str(day, "%Y-%m-%d").unwrap() }).collect::<Vec<&SharesOwned>>(); 
                if owned_shares.is_empty() { 
                    // if no shares owned before date, then just continue 0
                    continue;
                }
                // println!("Unsorted ---- {:?}", owned_shares);
                owned_shares.sort_by(|x,y| { (x.date).cmp(&y.date) });
                // println!("Sorted ---- {:?}", owned_shares);
                let most_recently_owned = owned_shares.last().unwrap();
                let quote = e.quotes.iter().find(|x| {
                    let date = OffsetDateTime::from_unix_timestamp(x.timestamp as i64).unwrap().date();
                    let ndate = NaiveDate::from_ymd_opt(date.year(), date.month() as u32, date.day() as u32).unwrap();
                    ndate == most_recently_owned.date
                }).expect(format!("No quote matching date {}", most_recently_owned.date).as_str());
                value = value + (quote.close * most_recently_owned.shares as f64) as f32;
            }
        }
        return value;
    }
}
