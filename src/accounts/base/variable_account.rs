use chrono::{Days, NaiveDate};
use inquire::*;

use crate::stocks;
use crate::types::investments::{StockInfo, StockRecord};
use crate::types::ledger::LedgerInfo;
use crate::types::participants::ParticipantType;
use crate::types::transfer_types::TransferType;
use crate::database::DbConn;

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
            DateSelect::new("Enter date of purchase").prompt();

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
            transfer_type: TransferType::WidthdrawalToInternalAccount,
            participant: pid,
            category_id: cid,
            description: format!(
                "Purchase {} shares of {} at ${} on {}",
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

        let sale_date = DateSelect::new("Enter date of purchase").prompt().unwrap();

        let sale_price: f32 = CustomType::<f32>::new("Enter sale price (per share): ")
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let mut number_of_shares_sold: f32 = CustomType::<f32>::new("Enter quantity sold: ")
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let value_received = number_of_shares_sold * sale_price;
        let stock_cid = self
            .db
            .get_category_id(self.id, "Sold".to_string())
            .unwrap();

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

        let sale_id = self.db.sell_stock(self.id, sale_record).unwrap();

        let sell_method: String =
            Select::new("Select sale methodology:", vec!["LIFO", "FIFO", "Custom"])
                .prompt()
                .unwrap()
                .to_string();

        let mut stocks: Vec<StockRecord> = Vec::new();
        match sell_method.as_str() {
            "LIFO" => {
                stocks = self
                    .db
                    .get_stock_history_ascending(self.id, ticker.clone())
                    .unwrap();
            }
            "FIFO" => {
                stocks = self
                    .db
                    .get_stock_history_descending(self.id, ticker.clone())
                    .unwrap();
            }
            "Custom" => {
                panic!("Not implemented!");
            }
            _ => {
                panic!("Unrecognized input!");
            }
        }

        let mut num_shares_remaining_to_allocate = number_of_shares_sold;
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
                .add_stock_sale_allocation(purchase_id, sale_id, num_shares_allocated)
                .unwrap();
            num_shares_remaining_to_allocate -= num_shares_allocated;

            // if there are no shares to allocate, we are done here and all sales
            // are accounted for
            if num_shares_remaining_to_allocate == 0.0 {
                break;
            }
        }
    }

    pub fn get_current_value(&mut self) -> f32 {
        let fixed_value = self.fixed.get_current_value();
        let variable_value = self.db.get_stock_current_value(self.id).unwrap();
        return fixed_value + variable_value;
    }

    pub fn time_weighted_return(&mut self, period_start: NaiveDate, period_end: NaiveDate) -> f32 {
        let rate: f32 = 0.0;
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
        let mut vf: f32 = 0.0;
        println!("Initial: {}", vi);

        let final_fixed_value = self
            .db
            .get_cumulative_total_of_ledger_before_date(self.id, period_end)
            .unwrap();
        let final_portfolio_value = self
            .db
            .get_portfolio_value_before_date(self.id, period_end)
            .unwrap();
        let final_vf = final_fixed_value + final_portfolio_value;
        println!("Final: {}", final_vf);

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
                TransferType::WidthdrawalToExternalAccount => -txn.amount,
                // all cash stays within account so cash flow is 0
                TransferType::WidthdrawalToInternalAccount => 0.0,
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

    // pub fn time_weighted_return(&mut self, period_start : NaiveDate, period_end : NaiveDate, ticker : Option<String>) -> f32 {
    //     let mut rate : f32 = 0.0;
    //     let mut owned_shares : HashMap<String, f32> = HashMap::new();
    //     let mut stock_values : HashMap<String, HashMap<String, Quote>> = HashMap::new();
    //     let mut share_txns   : HashMap<String, Vec<StockInfo>> = HashMap::new();
    //     let mut vi: f32 = 0.0;
    //     let mut single_ticker : bool = false;
    //     let mut tickers = Vec::new();
    //     // get fixed account trsanctions within the time period
    //     let mut fixed_transactions = self.db.get_ledger_entries_within_timestamps(self.id, period_start, period_end).unwrap();

    //     let mut fixed_vi = 0.0;
    //     if ticker.is_none() {
    //         // determining time-weighted rate of return for the entire account
    //         tickers = self.db.get_stock_tickers(self.id).unwrap();
    //         // get initial value of fixed account
    //         fixed_vi = self.db.get_cumulative_total_of_ledger_before_date(self.id, period_start).unwrap();
    //     } else {
    //         // determining time-weighted rate of return for the one stock
    //         //  - will not consider initial value (fixed_vi = 0.0) of, movements into our out of fixed account because we are only looking at one ticker

    //         // filter transactions that move money into and out of account
    //         fixed_transactions.retain(|transaction| transaction.transfer_type != TransferType::WidthdrawalToExternalAccount);
    //         fixed_transactions.retain(|transaction| transaction.transfer_type != TransferType::DepositFromExternalAccount);
    //         // filter transactions not including the selected ticker
    //         fixed_transactions.retain(|transaction| transaction.participant == ticker.clone().expect("Ticker not provided!"));

    //         tickers.push(ticker.clone().expect("Ticker not provided!"));
    //         single_ticker = true;
    //     }

    //     vi = fixed_vi;

    //     // get history for all stocks within the account
    //     for ticker in tickers.clone() {
    //         let (transactions, initial) =
    //             self.db.get_stock_history(self.id, ticker.clone(), period_start, period_end).unwrap();

    //         // get stock history starting a week before requested date so that we can find last open date if necessary
    //         let quotes = crate::stocks::get_stock_history(ticker.clone(), period_start.checked_sub_days(Days::new(7)).unwrap(), period_end).unwrap();

    //         // create a map of quotes to dates for quick lookup
    //         let mut quote_lookup : HashMap<String, Quote> = HashMap::new();
    //         for quote in quotes {
    //             let date_and_time = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
    //             let date = date_and_time.date();
    //             quote_lookup.insert( date.to_string(), quote.clone());
    //         }

    //         owned_shares.insert(ticker.clone(), initial.shares.clone());
    //         stock_values.insert(ticker.clone(), quote_lookup.clone());
    //         share_txns.insert(ticker.clone(), transactions);

    //         // ------------
    //         // sum up vi_s
    //         // ------------
    //         // check to see if the analyzed start date
    //         // occurred when the stock market was open.
    //         // choose the closing value of the stocks
    //         let mut first_open_stock_market_date = period_start
    //             .checked_sub_days(Days::new(1))
    //             .unwrap()
    //             .to_string();
    //         loop {
    //             // does quote exist for that date?
    //             if quote_lookup.get(&first_open_stock_market_date.clone()).is_none() {
    //                 first_open_stock_market_date =
    //                     NaiveDate::parse_from_str(
    //                         &first_open_stock_market_date.as_str(),
    //                         "%Y-%m-%d")
    //                         .unwrap()
    //                         .checked_sub_days(Days::new(1))
    //                         .expect("Invalid date range!")
    //                         .to_string();
    //                 continue;
    //             }

    //             // add initial value of owned stock
    //             vi += quote_lookup.get(&first_open_stock_market_date.clone()).expect("Stock market date not found!")
    //                 .clone().open as f32 * initial.shares;
    //             break;
    //         }
    //     }

    //     let mut sub_period_return: Vec<f32> = Vec::new();
    //     let mut cash_flow : f32 = 0.0;

    //     // if fixed_transactions.is_empty() {
    //     //     cash_flow = 0.0;
    //     //     let quotes =

    //     // }
    //     for txn in fixed_transactions {
    //         let date = txn.date;
    //         let cash_flow =
    //             // adding cash to account
    //             if txn.transfer_type == TransferType::DepositFromExternalAccount { txn.amount } else
    //             // removing cash from account
    //             if txn.transfer_type == TransferType::WidthdrawalToExternalAccount  { -txn.amount }  else
    //             // stock sale (for single stock analysis)
    //             if single_ticker && txn.transfer_type == TransferType::DepositFromInternalAccount { -txn.amount } else
    //             // stock purchase (for single stock analysis)
    //             if single_ticker && txn.transfer_type == TransferType::WidthdrawalToInternalAccount { txn.amount } else
    //             // otherwise, do not consider
    //             { 0.0 } ;

    //         // update shares of stock
    //         let re = Regex::new("([0-9.]+) shares").unwrap();
    //         let captures = re.captures(&txn.description).unwrap();
    //         let updated_shares = captures[1].to_string().parse().unwrap() as f32;

    //         owned_shares.entry(txn.participant).and_modify(|shares|
    //                 { *shares += updated_shares });

    //         for ticker in tickers {
    //             // determine account wealth on date of transaction
    //             let stock_quote =
    //                 stock_values.get(&ticker).expect(format!("Unable to find quote history for {}", ticker.clone()).as_str());

    //         //     if let Some(quote) = stock_quote.get(&date.clone()) {
    //         //         // check to ensure that transaction occured when the market was open
    //         //         // otherwise, we will just consider in the next period
    //         //         ve += quote.close *
    //         //     }
    //         //     let quote = .expect(format!("Unable to find quote for {} and date {}"))

    //         // }
    //     }
    //     return rate;
    // }
}
