use core::alloc;
use std::io::Read;
use std::sync::Arc;

use chrono::{Date, Days, NaiveDate};
use inquire::*;

use crate::database::DbConn;
use crate::stocks;
use crate::types::investments::{SaleAllocationInfo, SaleAllocationRecord, StockInfo, StockRecord, StockSplitAllocationInfo, StockSplitInfo, StockSplitRecord};
use crate::types::ledger::{LedgerInfo, LedgerRecord};
use crate::types::participants::ParticipantType;
use shared_lib::{LedgerEntry, TransferType};

use super::fixed_account::FixedAccount;

pub struct VariableAccount {
    pub id: u32,
    pub uid: u32,
    pub db: DbConn,
    pub fixed: FixedAccount,
}

impl VariableAccount {
    pub fn new(uid: u32, id: u32, db: &mut DbConn) -> Self {
        let acct: VariableAccount = Self {
            id: id,
            uid: uid,
            db: db.clone(),
            fixed: FixedAccount::new(uid, id, db.clone()),
        };
        acct
    }

    pub fn purchase_stock(&mut self, initial_opt : Option<StockRecord>, overwrite_entry : bool) -> LedgerRecord {
        let purchase: LedgerInfo;
        let defaults_to_use : bool;
        let mut initial: StockRecord = StockRecord { id: 0, info: StockInfo { shares: 0.0, costbasis: 0.0, remaining: 0.0, ledger_id: 0 }, txn_opt: None };

        if initial_opt.is_some() { 
            defaults_to_use = true;
            initial = initial_opt.unwrap();
        } else { 
            defaults_to_use = false;
        }

        let ticker_msg = "Enter stock ticker:";
        let ticker = if defaults_to_use { 
            let pid = initial.clone().txn_opt.expect("Ledger information not populated!").participant;
            let initial_ticker = self.db.get_participant(self.uid, self.id, pid).unwrap();
            Text::new(ticker_msg)
                .with_default(initial_ticker.as_str())
                .prompt()
                .unwrap()
                .to_string()
                .to_ascii_uppercase()
        } else { 
            Text::new(ticker_msg)
                .prompt()
                .unwrap()
                .to_string()
                .to_ascii_uppercase()
        };

        let ticker_valid = self.confirm_valid_ticker(ticker.clone());
        if ticker_valid == false { 
            panic!("Invalid stock ticker entered!");
        }

        let date_msg =  "Enter date of purchase:";
        let date_input= if defaults_to_use {
            let initial_date = initial.clone().txn_opt.expect("Ledger information not populated!").date;
            DateSelect::new(date_msg)
                .with_default(NaiveDate::parse_from_str(&initial_date, "%Y-%m-%d").unwrap())
                .prompt()
                .unwrap().to_string()
        } else { 
            DateSelect::new(date_msg).prompt().unwrap().to_string()
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

        let pid =
            self.db
                .check_and_add_participant(self.uid, self.id, ticker.clone(), ParticipantType::Payee, false);
        let cid = self
            .db
            .check_and_add_category(self.uid,  self.id, "buy".to_string());

        purchase = LedgerInfo {
            date: date_input.clone(),
            amount: shares * costbasis,
            transfer_type: TransferType::WithdrawalToInternalAccount,
            participant: pid,
            category_id: cid,
            description: format!(
                "[Internal] Purchase {} shares of {} at ${} on {}",
                shares,
                ticker,
                costbasis,
                date_input.clone()
            ),
            ancillary_f32data : 0.0
        };

        let ledger_id = if defaults_to_use && overwrite_entry { 
            self.db.update_ledger_item(self.uid, self.id, LedgerRecord { id: initial.info.ledger_id , info: purchase.clone() }).unwrap()
        } else { 
            self.db.add_ledger_entry(self.uid, self.id, purchase.clone()).unwrap()
        };

        let stock_record = StockInfo {
            shares: shares,
            costbasis: costbasis,
            remaining: shares,
            ledger_id: ledger_id,
        };

        self.db.add_stock_purchase(self.uid, self.id, stock_record).unwrap();

        return LedgerRecord { id:  ledger_id, info: purchase.clone() };
    }

    pub fn sell_stock(&mut self, initial_opt : Option<StockRecord>, overwrite_entry : bool) -> LedgerRecord {

        let defaults_to_use : bool;
        let mut initial: StockRecord = StockRecord { id: 0, info: StockInfo { shares: 0.0, costbasis: 0.0, remaining: 0.0, ledger_id: 0 }, txn_opt: None };

        if initial_opt.is_some() { 
            defaults_to_use = true;
            initial = initial_opt.unwrap();
        } else { 
            defaults_to_use = false;
        }

        let ticker : String;
        let ticker_msg = "Select which stock you would like to record a sale of:";
        let tickers = self.db.get_stock_tickers(self.uid, self.id).unwrap();
        ticker = if defaults_to_use {  
            let pid = initial.clone().txn_opt.expect("Ledger information not populated!").participant;
            let initial_ticker = self.db.get_participant(self.uid, self.id, pid).unwrap();
            Select::new(ticker_msg, tickers)
                .with_starting_filter_input(initial_ticker.as_str())
                .prompt()
                .unwrap()
                .to_string()
                .to_ascii_uppercase()
        } else { 
            Select::new(ticker_msg,tickers)
                .prompt()
                .unwrap()
                .to_string()
                .to_ascii_uppercase()
        };

        let pid =
            self.db
                .check_and_add_participant(self.uid, self.id, ticker.clone(), ParticipantType::Payer, false);

        let date_msg = "Enter date of sale:";
        let sale_date = if defaults_to_use { 
            let initial_date = initial.clone().txn_opt.expect("Ledger information not populated!").date;
            DateSelect::new(date_msg)
                .with_default(NaiveDate::parse_from_str(&initial_date, "%Y-%m-%d").unwrap())
                .prompt()
                .unwrap()
        } else { 
            DateSelect::new(date_msg)
                .prompt()
                .unwrap()
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

        let shares_msg = "Enter quantity sale:";
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
        let stock_cid = self
            .db
            .check_and_add_category(self.uid, self.id, "sale".to_string());

        let sale = LedgerInfo {
            date: sale_date.to_string(),
            amount: value_received,
            transfer_type: TransferType::DepositFromInternalAccount,
            participant: pid,
            category_id: stock_cid,
            description: format!(
                "[Internal]: sale {} shares of {} at ${} on {}.",
                number_of_shares_sale,
                ticker,
                sale_price,
                sale_date.to_string()
            ),
            ancillary_f32data : 0.0
        };

        let ledger_id: u32 = if defaults_to_use && overwrite_entry {
            self.db.update_ledger_item(self.uid,self.id, LedgerRecord{id : initial.info.ledger_id, info : sale.clone()}).unwrap()
        } else { 
            self.db.add_ledger_entry(self.uid, self.id, sale.clone()).unwrap()
        };

        let sale_record = StockInfo {
            shares: number_of_shares_sale,
            costbasis: sale_price,
            remaining: 0.0,
            ledger_id: ledger_id,
        };

        let sale_id = self.db.add_stock_sale(self.uid, self.id, sale_record.clone()).unwrap();
        
        let sale_info = StockRecord {
            id: sale_id,
            info: sale_record.clone(),
            txn_opt: Some(sale.clone())
        };

        const SALE_METHOD_OPTIONS: [&'static str; 2] = ["LIFO", "FIFO"];
        let sell_method: String =
            Select::new("Select sale methodology:", SALE_METHOD_OPTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();

        self.allocate_sale_stock(sale_info, sell_method);

        return LedgerRecord { id: ledger_id, info: sale.clone() }
    }

    pub fn split_stock(&mut self, initial_opt : Option<StockSplitRecord>, overwrite_entry : bool) -> LedgerRecord {

        let defaults_to_use : bool;
        let mut initial: StockSplitRecord = StockSplitRecord { id: 0, info: StockSplitInfo { split: 0.0, ledger_id: 0 }, txn_opt: None };

        if initial_opt.is_some() { 
            defaults_to_use = true;
            initial = initial_opt.unwrap();
        } else { 
            defaults_to_use = false;
        }

        let tickers = self.db.get_stock_tickers(self.uid, self.id).unwrap();
        let ticker_msg = "Select which stock you would like to report a split of:";
        let ticker = if defaults_to_use {
                let pid = initial.clone().txn_opt.expect("Ledger information not populated!").participant;
                let initial_ticker: String = self.db.get_participant(self.uid, self.id, pid).unwrap();
                Select::new(ticker_msg,tickers)
                    .with_starting_filter_input(&initial_ticker.as_str())
                    .prompt()
                    .unwrap()
                    .to_string()
        } else { 
            Select::new(ticker_msg,tickers)
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
            let initial_date = initial.clone().txn_opt.expect("Ledger information not populated!").date;
            DateSelect::new(date_msg)
            .with_starting_date(NaiveDate::parse_from_str(initial_date.as_str(), "%Y-%m-%d").expect("Unable to convert date to NaiveDate"))
            .prompt()
            .unwrap()
            .to_string()
        } else {
            DateSelect::new(date_msg).prompt().unwrap().to_string()
        };

        let pid = self.db.check_and_add_participant(self.uid, self.id, ticker.clone(), ParticipantType::Both, false);
        let cid = self.db.check_and_add_category(self.uid, self.id, "stock dividend/split".to_string());

        let ledger_entry = LedgerInfo { 
            date : split_date.clone(), 
            amount : 0.0, 
            transfer_type : TransferType::ZeroSumChange, 
            participant : pid, 
            category_id : cid, 
            description : format!("[Internal]: Split of {} by factor of {} on {}.", ticker.clone(), split.clone(), split_date),
            ancillary_f32data : self.db.get_stocks(self.uid, self.id, ticker.clone()).unwrap().iter().map(|rcrd| rcrd.info.remaining).sum::<f32>() * split
        };

        let lid = if defaults_to_use && overwrite_entry { 
            self.db.update_ledger_item(self.uid, self.id, LedgerRecord { id: initial.info.ledger_id, info: ledger_entry.clone() }).unwrap()
        } else { 
            self.db.add_ledger_entry(self.uid, self.id, ledger_entry.clone()).unwrap()
        };

        let stock_split_id = self.db
            .add_stock_split(self.uid, self.id,  split.clone(), lid)
            .unwrap();

        let stock_split_record = StockSplitRecord { 
            id : stock_split_id, 
            info : StockSplitInfo { 
                split : split.clone(),
                ledger_id : lid.clone()
            },
            txn_opt : Some(ledger_entry.clone())
        };

        self.allocate_stock_split(stock_split_record);

        return LedgerRecord{ id : lid, info : ledger_entry};
    }

    pub fn modify(&mut self, record : LedgerRecord) -> LedgerRecord {

        println!("Record id: {}", record.clone().id);

        let was_stock_purchase_opt = self.db.check_and_get_stock_purchase_record_matching_from_ledger_id(self.uid, self.id, record.id).unwrap();
        let was_stock_sale_opt = self.db.check_and_get_stock_sale_record_matching_from_ledger_id(self.uid, self.id, record.id).unwrap();
        let was_stock_split_opt = self.db.check_and_get_stock_split_record_matching_from_ledger_id(self.uid, self.id, record.id).unwrap();

        let mut is_stock_purchase: bool = false;
        let mut is_stock_sale: bool = false;
        let mut is_stock_split: bool = false;
        let mut stock_record: StockRecord = StockRecord { id: 0, info: StockInfo { shares : 0.0, costbasis : 0.0, remaining : 0.0, ledger_id : 0 }, txn_opt: None };
        let mut split_record: StockSplitRecord = StockSplitRecord { id : 0, info : StockSplitInfo { split: 0.0, ledger_id: 0 }, txn_opt : None };
        if was_stock_purchase_opt.is_none() && was_stock_sale_opt.is_none() && was_stock_split_opt.is_none() { 
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

        const OPTIONS: [&'static str; 2] = ["Update", "Remove"];
        let modify_choice = Select::new("What would you like to do:", OPTIONS.to_vec()).prompt().unwrap();
        match modify_choice { 
            "Update" => {
                if is_stock_purchase {
                    self.db.remove_stock_purchase(self.uid, self.id, stock_record.id).unwrap();
                    return self.purchase_stock(Some(stock_record), true);
                } else if is_stock_sale {
                    self.deallocate_sale_stock(stock_record.id);
                    self.db.remove_stock_sale(self.uid, self.id, stock_record.info.ledger_id).unwrap();
                    return self.sell_stock(Some(stock_record), true);
                } else { 
                    // split stock
                    self.deallocate_stock_split(split_record.clone());
                    return self.split_stock(Some(split_record.clone()), true);
                }
            }
            "Remove" => {
                if is_stock_purchase { 
                    // TODO: check if part of any splits or sales before removing
                    self.db.remove_stock_purchase(self.uid, self.id, stock_record.info.ledger_id);
                    self.db.remove_ledger_item(self.uid, self.id, stock_record.info.ledger_id);
                } else if is_stock_sale { 
                    println!("{}", stock_record.clone().id);
                    self.deallocate_sale_stock(stock_record.clone().id);
                    self.db.remove_stock_sale(self.uid, self.id, stock_record.info.ledger_id);
                    self.db.remove_ledger_item(self.uid, self.id, stock_record.info.ledger_id);
                } else {
                    self.deallocate_stock_split(split_record.clone());
                    self.db.remove_stock_split(self.uid, self.id, split_record.info.ledger_id);
                    self.db.remove_ledger_item(self.uid, self.id, split_record.info.ledger_id);
                }
                return record;
            }
            _ => { 
                panic!("Input not recognized!");
            }
        }
        
    }

    fn allocate_sale_stock(&mut self, record: StockRecord, method: String) {
        let stocks: Vec<StockRecord>;
        let ticker = self.db.get_participant(self.uid, self.id, record.txn_opt.expect("Transaction required but not found!").participant).unwrap();
        match method.as_str() {
            "LIFO" => {
                stocks = self
                    .db
                    .get_stock_history_ascending(self.uid, self.id, ticker
                    )
                    .unwrap();
            }
            "FIFO" => {
                stocks = self
                    .db
                    .get_stock_history_descending(self.uid, self.id, ticker
                    )
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
                stock.info.remaining -= num_shares_remaining_to_allocate;
                num_shares_allocated = num_shares_remaining_to_allocate;
            } else {
                stock.info.remaining = 0.0;
                num_shares_allocated = stock.info.remaining;
            }
            self.db
                .update_stock_remaining(purchase_id.clone(), self.id, stock.id, stock.info.remaining)
                .unwrap();
            self.db
                .add_stock_sale_allocation(self.uid, self.id, purchase_id, record.id, num_shares_allocated)
                .unwrap();
            num_shares_remaining_to_allocate -= num_shares_allocated;

            // if there are no shares to allocate, we are done here and all sales
            // are accounted for
            if num_shares_remaining_to_allocate == 0.0 {
                break;
            }
        }
    }

    fn deallocate_sale_stock(&mut self, sale_id: u32) {
        let stock_allocation_records = self
            .db
            .get_stock_sale_allocation_for_sale_id(self.uid, self.id, sale_id)
            .unwrap();
        for record in stock_allocation_records {
            // add shares back to ledger
           let _ = self.db
                .add_to_stock_remaining(self.uid, self.id, record.info.purchase_id, record.info.quantity).unwrap();
            self.db.remove_stock_sale_allocation(self.uid, self.id,record.id);
        }
        // self.db.remove_stock_sale(sale_record.info.ledger_id);
    }

    pub fn allocate_stock_split(&mut self, record : StockSplitRecord) { 
        let ticker = self.db.get_participant(self.uid, self.id, record.txn_opt.expect("Corresponding ledger transaction not provided!").participant).unwrap();
        let stock_records = self.db.get_stocks(self.uid,self.id, ticker).unwrap();
        for stock in stock_records {
            self.db.update_stock_remaining(stock.id, self.id, stock.id, stock.info.remaining * record.info.split)
                .unwrap();
            self.db.update_cost_basis(self.uid, self.id, stock.id, stock.info.costbasis / record.info.split)
                .unwrap();
            self.db.add_stock_split_allocation(self.uid, self.id, StockSplitAllocationInfo { stock_split_id : record.id, stock_purchase_id : stock.id }).unwrap();
        }
    }

    fn deallocate_stock_split(&mut self, record : StockSplitRecord ) { 
        let stock_split_alloc_records = self
            .db
            .get_stock_split_allocation_for_stock_split_id(self.uid,self.id, record.id)
            .unwrap();
        for alloc_record in stock_split_alloc_records {
            // add shares back to ledger
            let stock_purchase = self.db.check_and_get_stock_purchase_record_matching_from_purchase_id(self.uid, self.id, alloc_record.info.stock_purchase_id).unwrap().expect("Stock record not returned");
            let updated_shares = stock_purchase.info.shares / record.info.split;
            let _ = self.db.update_stock_remaining(stock_purchase.id, self.id, alloc_record.info.stock_purchase_id, updated_shares);
            let updated_costbasis = stock_purchase.info.costbasis * record.info.split;
            let _ = self.db.update_cost_basis(self.uid, self.id, alloc_record.info.stock_purchase_id, updated_costbasis);
            self.db.remove_stock_split_allocation(self.uid, self.id, alloc_record.id);
        }
        self.db.remove_stock_split(self.uid, self.id, record.info.ledger_id);
    }

    pub fn confirm_valid_ticker(&mut self, ticker : String) -> bool { 
        let rs = stocks::get_stock_at_close(ticker.clone());
        match rs {
            Ok(price) => {true}
            Err(error) => {
                panic!("Fetch failed for ticker '{}': {}!", ticker.clone(), error);
            }
        }

    }

    pub fn get_current_value(&mut self) -> f32 {
        let fixed_value = self.fixed.get_current_value();
        let variable_value = self.db.get_stock_current_value(self.uid, self.id).unwrap();
        return fixed_value + variable_value;
    }

    pub fn time_weighted_return(&mut self, period_start: NaiveDate, period_end: NaiveDate) -> f32 {
        let mut cf: f32 = 0.0;
        let mut hps: Vec<f32> = Vec::new();
        let mut hp: f32;

        let fixed_transactions = self
            .db
            .get_ledger_entries_within_timestamps(self.uid, self.id, period_start, period_end)
            .unwrap();
        let mut iter = fixed_transactions.iter().peekable();

        // calculate value before date
        let fixed_value = self
            .db
            .get_cumulative_total_of_ledger_before_date(
                    self.uid, 
                self.id,
                period_start
                    .checked_sub_days(Days::new(1))
                    .expect("Invalid date!"),
            )
            .unwrap();
        let variable_value = self
            .db
            .get_portfolio_value_before_date(self.uid, self.id, period_start)
            .unwrap();
        let mut vi = fixed_value + variable_value;
        let mut vf: f32;

        let final_fixed_value = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.uid, self.id, period_end)
            .unwrap();
        let final_portfolio_value = self
            .db
            .get_portfolio_value_before_date(self.uid,self.id, period_end)
            .unwrap();
        let final_vf = final_fixed_value + final_portfolio_value;

        if iter.peek().is_none() {
            // no transactions during analyzed period, so
            // calculate regular growth rate
            hp = (final_vf - vi) / (vi);
            return ((1.0 + hp) - 1.0) * 100 as f32;
        }

        while let Some(txn) = iter.next() {
            let tt: &TransferType = &txn.transfer_type;
            cf += match tt {
                // positive cash flow
                TransferType::DepositFromExternalAccount => txn.amount,
                // all cash stays within account so cash flow is 0
                TransferType::DepositFromInternalAccount => 0.0,
                // withdrawing from account to cash flow is negative
                TransferType::WithdrawalToExternalAccount => -txn.amount,
                // all cash stays within account so cash flow is 0
                TransferType::WithdrawalToInternalAccount => 0.0,
                // Cash flow is zero on zero-sum change
                TransferType::ZeroSumChange => 0.0
            };

            if iter.peek().is_none() {
                // no more left so calculate with account value at end of analysis period
                vf = final_vf;
                hp = (vf - (cf + vi)) / (cf + vi);
                hps.push(hp);
            } else {
                let nxt = iter.peek().expect("Item found, but not available");
                // in the event that there is a second transaction in this period, we will add this all to the cash flow
                if nxt.date == txn.date {
                    continue;
                };

                // next transaction is for a new period, so we can calculate the Hp and reset
                let end_of_period: NaiveDate =
                    NaiveDate::parse_from_str(&txn.date.as_str(), "%Y-%m-%d")
                        .expect("Invalid date!");
                let vf_fixed = self
                    .db
                    .get_cumulative_total_of_ledger_before_date(self.uid, self.id, end_of_period)
                    .unwrap();
                let vf_variable = self
                    .db
                    .get_portfolio_value_before_date(self.uid, self.id, end_of_period)
                    .unwrap();
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
}
