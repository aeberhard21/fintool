use chrono::{Date, Days, NaiveDate};
use inquire::*;

use crate::database::DbConn;
use crate::stocks;
use crate::types::investments::{SaleAllocationInfo, SaleAllocationRecord, StockInfo, StockRecord};
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

    pub fn purchase_stock(&mut self) {
        let purchase: LedgerInfo;

        let mut ticker: String = String::new();
        ticker = Text::new("Enter stock ticker: ")
            .prompt()
            .unwrap()
            .to_string();
        let rs = stocks::get_stock_at_close(ticker.clone());
        match rs {
            Ok(price) => {}
            Err(error) => {
                panic!("Fetch failed for ticker '{}': {}!", ticker.clone(), error);
            }
        }

        let date_input: Result<NaiveDate, InquireError> =
            DateSelect::new("Enter date of purchase:").prompt();

        let shares: f32 = CustomType::<f32>::new("Enter number of shares purchased: ")
            .with_placeholder("0.00")
            .with_default(0.00)
            .with_error_message("Please enter a valid amount!")
            .prompt()
            .unwrap();

        let costbasis: f32 = CustomType::<f32>::new("Enter cost basis of shares purchased: ")
            .with_placeholder("0.00")
            .with_default(0.00)
            .with_error_message("Please enter a valid amount!")
            .prompt()
            .unwrap();

        let pid =
            self.db
                .check_and_add_participant(self.id, ticker.clone(), ParticipantType::Payee);
        let cid = self
            .db
            .check_and_add_category(self.id, "Bought".to_string());

        let date = date_input.unwrap().to_string();

        purchase = LedgerInfo {
            date: date.clone(),
            amount: shares * costbasis,
            transfer_type: TransferType::WithdrawalToInternalAccount,
            participant: pid,
            category_id: cid,
            description: format!(
                "[Internal] Purchase {} shares of {} at ${} on {}",
                shares,
                ticker,
                costbasis,
                date.clone()
            ),
        };

        let ledger_id = self.db.add_ledger_entry(self.id, purchase).unwrap();

        let stock_record = StockInfo {
            date: date.clone(),
            ticker: ticker.clone(),
            shares: shares,
            costbasis: costbasis,
            remaining: shares,
            ledger_id: ledger_id,
        };

        self.db.add_stock(self.id, stock_record).unwrap();
    }

    pub fn sell_stock(&mut self) {
        let tickers = self.db.get_stock_tickers(self.id).unwrap();
        let ticker = Select::new(
            "\nSelect which stock you would like to record a sale of:",
            tickers,
        )
        .prompt()
        .unwrap()
        .to_string();
        let pid =
            self.db
                .check_and_add_participant(self.id, ticker.clone(), ParticipantType::Payer);

        // let owned_stocks: Vec<StockEntry> = self.db.get_stocks(self.id, ticker.clone()).unwrap();

        let sale_date = DateSelect::new("Enter date of sale: ").prompt().unwrap();

        let sale_price: f32 = CustomType::<f32>::new("Enter sale price (per share): ")
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let number_of_shares_sold: f32 = CustomType::<f32>::new("Enter quantity sold: ")
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let value_received = number_of_shares_sold * sale_price;
        let stock_cid = self
            .db
            .check_and_add_category(self.id, "Sold".to_string());

        let sale = LedgerInfo {
            date: sale_date.to_string(),
            amount: value_received,
            transfer_type: TransferType::DepositFromInternalAccount,
            participant: pid,
            category_id: stock_cid,
            description: format!(
                "[Internal]: Sold {} shares of {} at ${} on {}.",
                number_of_shares_sold,
                ticker,
                sale_price,
                sale_date.to_string()
            ),
        };

        let ledger_id = self.db.add_ledger_entry(self.id, sale).unwrap();

        let sale_record = StockInfo {
            ticker: ticker.clone(),
            shares: number_of_shares_sold,
            costbasis: sale_price,
            date: sale_date.to_string(),
            remaining: 0.0,
            ledger_id: ledger_id,
        };

        let sale_id = self.db.sell_stock(self.id, sale_record.clone()).unwrap();
        let sale_info = StockRecord {
            id: sale_id,
            info: sale_record.clone(),
        };

        const SALE_METHOD_OPTIONS: [&'static str; 2] = ["LIFO", "FIFO"];
        let sell_method: String =
            Select::new("Select sale methodology:", SALE_METHOD_OPTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();

        self.allocate_sold_stock(sale_info, sell_method);
    }

    pub fn modify(&mut self, record: LedgerRecord) -> LedgerRecord {
        let updated_record;

        // does this correlate with purchase or sale of stocks
        let was_stock_purchase = self
            .db
            .check_and_get_stock_purchase_record_matching_from_ledger_id(record.id)
            .unwrap();
        let was_stock_sale = self
            .db
            .check_and_get_stock_sale_record_matching_from_ledger_id(record.id)
            .unwrap();

        let original_record;
        let original_txn;

        // if not just update in place
        if was_stock_purchase.is_none() && was_stock_sale.is_none() {
            updated_record = self.fixed.modify(record);
            return updated_record;
        }
        if was_stock_purchase.is_some() {
            original_record = was_stock_purchase.unwrap();
            original_txn = "purchase";
        } else {
            original_record = was_stock_sale.unwrap();
            original_txn = "sale";
        }

        const OPTIONS: [&'static str; 4] = ["Purchase", "Sale", "Stock Split", "None"];
        let purchase_or_sale = Select::new("Purchase, Sale, or Stock Split:", OPTIONS.to_vec())
            .with_starting_filter_input(original_txn)
            .prompt()
            .unwrap();

        let mut tickers = self.db.get_stock_tickers(self.id).unwrap();

        let ticker_msg = format!(
            "Select which stock would like to record the {} of:",
            purchase_or_sale
        );
        let updated_ticker = Select::new(ticker_msg.as_str(), tickers)
            .with_starting_filter_input(&original_record.info.ticker)
            .prompt()
            .unwrap();

        let stock_valid = stocks::get_stock_at_close(updated_ticker.clone());
        match stock_valid {
            Ok(price) => {}
            Err(error) => {
                panic!(
                    "Fetch failed for ticker '{}': {}!",
                    updated_ticker.clone(),
                    error
                );
            }
        }

        let date_msg = format!("Enter date of {}", purchase_or_sale);
        let updated_date = DateSelect::new(date_msg.as_str())
            .with_default(
                NaiveDate::parse_from_str(original_record.info.date.as_str(), "%Y-%m-%d").unwrap(),
            )
            .prompt()
            .unwrap();

        let price_msg = format!("Enter {} price (per share):", purchase_or_sale);
        let updated_price: f32 = CustomType::<f32>::new(price_msg.as_str())
            .with_default(original_record.info.costbasis)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let shares_msg = format!("Enter shares transacted in {}:", purchase_or_sale);
        let updated_shares: f32 = CustomType::<f32>::new(shares_msg.as_str())
            .with_default(original_record.info.shares)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let (updated_ptype, updated_category, updated_ttype) = match purchase_or_sale {
            "Purchase" => (
                ParticipantType::Payee,
                "Bought",
                TransferType::WithdrawalToInternalAccount,
            ),
            "Sale" => (
                ParticipantType::Payer,
                "Sold",
                TransferType::DepositFromInternalAccount,
            ),
            _ => {
                panic!("Not implemented!")
            }
        };

        let updated_pid =
            self.db
                .check_and_add_participant(self.id, updated_ticker.clone(), updated_ptype);
        let updated_cid = self
            .db
            .check_and_add_category(self.id, updated_category.to_string());

        let updated_amount = updated_shares * updated_price;
        let updated_description = if original_txn == "Purchase" {
            format!(
                "[Internal] Purchase {} shares of {} at ${} on {}",
                updated_shares,
                updated_ticker,
                updated_price,
                updated_date.to_string()
            )
        } else {
            format!(
                "[Internal] Sold {} shares of {} at ${} on {}.",
                updated_shares,
                updated_ticker,
                updated_price,
                updated_date.to_string()
            )
        };

        updated_record = LedgerRecord {
            id: original_record.info.ledger_id,
            info: LedgerInfo {
                date: updated_date.to_string(),
                amount: updated_amount,
                transfer_type: updated_ttype,
                participant: updated_pid,
                category_id: updated_cid,
                description: updated_description,
            },
        };

        let updated_stock_info = StockInfo {
            date: updated_date.to_string(),
            ticker: updated_ticker,
            shares: updated_shares,
            costbasis: updated_price,
            remaining: updated_shares,
            ledger_id: original_record.info.ledger_id,
        };
        let updated_stock_record;
        let stock_id;

        if "Purchase" == purchase_or_sale.to_string() {
            // Check if previously entered as a sale and now needs to
            // be a purchase.
            if "sale" == original_txn.to_string() {
                let sale_id_or_none = self
                    .db
                    .remove_stock_sale(original_record.info.ledger_id)
                    .unwrap();
                if sale_id_or_none.is_none() {
                    println!("Sale could not be found associated with the modified ledger item!");
                    return updated_record;
                }

                // Once done, remove stock allocation for this ledger id
                let sale_id = sale_id_or_none.unwrap();
                self.deallocate_sold_stock(sale_id);

                stock_id = self.db.add_stock(self.id, updated_stock_info).unwrap();
            } else {
                let stock_id_or_none = self
                    .db
                    .update_stock_purchase(updated_stock_info.clone())
                    .unwrap();
                if stock_id_or_none.is_none() {
                    println!("Purchase could not be associated with the modified ledger item!");
                    return updated_record;
                }
                stock_id = stock_id_or_none.unwrap();
            }
        } else {
            const SALE_METHOD_OPTIONS: [&'static str; 2] = ["LIFO", "FIFO"];
            let sell_method: String =
                Select::new("Select sale methodology:", SALE_METHOD_OPTIONS.to_vec())
                    .prompt()
                    .unwrap()
                    .to_string();

            // Check if previously entered as purchase and now needs to
            // a sale.
            if "purchase" == original_txn.to_string() {
                let _purchase_id = self
                    .db
                    .remove_stock_purchase(original_record.info.ledger_id)
                    .unwrap();
                let stock_id = self
                    .db
                    .sell_stock(self.id, updated_stock_info.clone())
                    .unwrap();
                updated_stock_record = StockRecord {
                    id: stock_id,
                    info: updated_stock_info,
                };
                self.allocate_sold_stock(updated_stock_record, sell_method);
            } else {
                let sale_id_or_none = self
                    .db
                    .update_stock_sale(updated_stock_info.clone())
                    .unwrap();
                if sale_id_or_none.is_none() {
                    println!("Sale could not be associated with the modified ledger item!");
                    return updated_record;
                }
                stock_id = sale_id_or_none.unwrap();
                updated_stock_record = StockRecord {
                    id: stock_id,
                    info: updated_stock_info,
                };
                // TODO: deallocate sold stock, remove entries
                self.deallocate_sold_stock(stock_id);
            
                // reallocate
                self.allocate_sold_stock(updated_stock_record, sell_method);
            }
        }

        let _updated_id = self.db.update_ledger_item(updated_record.clone()).unwrap();
        return updated_record;
    }

    fn allocate_sold_stock(&mut self, record: StockRecord, method: String) {
        let stocks: Vec<StockRecord>;
        match method.as_str() {
            "LIFO" => {
                stocks = self
                    .db
                    .get_stock_history_ascending(self.id, record.info.ticker)
                    .unwrap();
            }
            "FIFO" => {
                stocks = self
                    .db
                    .get_stock_history_descending(self.id, record.info.ticker)
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
                .update_stock_remaining(purchase_id.clone(), stock.info.remaining)
                .unwrap();
            self.db
                .add_stock_sale_allocation(purchase_id, record.id, num_shares_allocated)
                .unwrap();
            num_shares_remaining_to_allocate -= num_shares_allocated;

            // if there are no shares to allocate, we are done here and all sales
            // are accounted for
            if num_shares_remaining_to_allocate == 0.0 {
                break;
            }
        }
    }

    fn deallocate_sold_stock(&mut self, sale_id: u32) {
        let stock_allocation_records = self
            .db
            .get_stock_sale_allocation_for_sale_id(sale_id)
            .unwrap();
        for record in stock_allocation_records {
            // add shares back to ledger
           let _ = self.db
                .add_to_stock_remaining(record.info.purchase_id, record.info.quantity).unwrap();
            self.db.remove_stock_sale_allocation(record.id);
        }
    }

    pub fn split_stock(&mut self) {
        let tickers = self.db.get_stock_tickers(self.id).unwrap();
        let ticker = Select::new(
            "\nSelect which stock you would like to report a split of:",
            tickers,
        )
        .prompt()
        .unwrap()
        .to_string();
        let split: f32 = CustomType::<f32>::new("Enter split factor:")
            .with_placeholder("2.0")
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();
        let split_date = DateSelect::new("Enter date of split:").prompt().unwrap();
        self.db
            .add_stock_split(self.id, split_date.to_string(), ticker, split)
            .unwrap();
    }

    pub fn get_current_value(&mut self) -> f32 {
        let fixed_value = self.fixed.get_current_value();
        let variable_value = self.db.get_stock_current_value(self.id).unwrap();
        return fixed_value + variable_value;
    }

    pub fn time_weighted_return(&mut self, period_start: NaiveDate, period_end: NaiveDate) -> f32 {
        let mut cf: f32 = 0.0;
        let mut hps: Vec<f32> = Vec::new();
        let mut hp: f32;

        let fixed_transactions = self
            .db
            .get_ledger_entries_within_timestamps(self.id, period_start, period_end)
            .unwrap();
        let mut iter = fixed_transactions.iter().peekable();

        // calculate value before date
        let fixed_value = self
            .db
            .get_cumulative_total_of_ledger_before_date(
                self.id,
                period_start
                    .checked_sub_days(Days::new(1))
                    .expect("Invalid date!"),
            )
            .unwrap();
        let variable_value = self
            .db
            .get_portfolio_value_before_date(self.id, period_start)
            .unwrap();
        let mut vi = fixed_value + variable_value;
        let mut vf: f32;

        let final_fixed_value = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.id, period_end)
            .unwrap();
        let final_portfolio_value = self
            .db
            .get_portfolio_value_before_date(self.id, period_end)
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
                    .get_cumulative_total_of_ledger_before_date(self.id, end_of_period)
                    .unwrap();
                let vf_variable = self
                    .db
                    .get_portfolio_value_before_date(self.id, end_of_period)
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
