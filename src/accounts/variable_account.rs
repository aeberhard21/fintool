use chrono::format::Fixed;
use chrono::{Days, NaiveDate};
use inquire::*;
use rusqlite::Transaction;
use tokio::signal;
use std::collections::HashSet;
use std::{collections::HashMap, fmt::Formatter};
use std::time::{Duration, UNIX_EPOCH};
use time::OffsetDateTime;
use yahoo_finance_api::Quote;

use crate::{database::DbConn, types::transfer_types};
use crate::types::investments::StockRecord;
use crate::types::ledger::LedgerEntry;
use crate::types::participants::ParticipantType;
use crate::types::transfer_types::TransferType;
use crate::stocks;

use super::fixed_account::{self, FixedAccount};

pub struct VariableAccount {
    pub id : u32,
    pub db : DbConn,
    pub fixed : FixedAccount
}

impl VariableAccount {

    pub fn new(id : u32, db : &mut DbConn) -> Self {
        let acct : VariableAccount = Self { 
            id : id, 
            db : db.clone(), 
            fixed : FixedAccount::new(id, db.clone())
        };
        acct
    }

    pub fn purchase_stock(&mut self) {

        let purchase : LedgerEntry;

        let mut ticker: String = String::new();
        ticker = Text::new("Enter stock ticker: ")
            .prompt()
            .unwrap()
            .to_string();
        let rs = stocks::get_stock_at_close(ticker.clone());
        match rs {
            Ok(price) => {
            }
            Err(error) => {
                panic!("Fetch failed for ticker '{}': {}!", ticker.clone(), error);
            }
        }
        
        let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date of purchase").prompt();
    
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

        let stock_record = StockRecord {
            date: date_input.unwrap().to_string(),
            ticker: ticker,
            shares: shares,
            costbasis: costbasis,
        };

        let pid = self.db.check_and_add_participant(self.id, stock_record.ticker.clone(), ParticipantType::Payee);
        let cid = self.db.check_and_add_category(self.id, "Bought".to_string());

        purchase = LedgerEntry {
            date : stock_record.date.to_string(),
            amount :  stock_record.shares * stock_record.costbasis,
            transfer_type : TransferType::WidthdrawalToInternalAccount,
            participant_id : pid,
            category_id : cid,
            description : format!("Purchase {} shares of {} at ${} on {}",
                stock_record.shares,
                stock_record.ticker,
                stock_record.costbasis,
                stock_record.date)
        };

        self.db.add_stock(self.id, stock_record).unwrap();
        self.db.add_ledger_entry(self.id, purchase);

    }

    pub fn sell_stock(&mut self) {
        let tickers = self.db.get_stock_tickers(self.id).unwrap();
        let ticker = Select::new("\nSelect which stock you would like to record a sale of:", tickers)
            .prompt()
            .unwrap()
            .to_string();
        let owned_stocks = self.db.get_stocks(self.id, ticker.clone()).unwrap();
        
        let sale_date: Result<NaiveDate, InquireError> = DateSelect::new("Enter date of purchase").prompt();

        let sale_price : f32 = CustomType::<f32>::new("Enter sale price: ")
            .with_placeholder("00000.00")
            .with_default(00000.00)
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();

        let all_or_partial_sale: String = Select::new("Sell all or partial:", vec!["All", "Partial"])
            .prompt()
            .unwrap()
            .to_string();

        let mut number_of_shares_sold : f32 = 0.0;
        let mut cost_basis_of_shares_sold : f32 = 0.0;

        match all_or_partial_sale.as_str() { 
            "All" => {
                number_of_shares_sold = owned_stocks.iter().map(|stock_entry| stock_entry.record.shares).sum();
                cost_basis_of_shares_sold = owned_stocks.iter().map(|stock_entry| stock_entry.record.costbasis).sum();
                self.db.drop_stock(self.id, ticker.clone());
            }
            "Partial" => {
                let mut entry_map : HashMap<String, u32> = HashMap::new();
                let mut record_map: HashMap<u32, StockRecord> = HashMap::new();
                let mut commands : Vec<String> = Vec::new();
                for entries in owned_stocks {

                    let key = format!(
                        "{}",
                        [entries.record.ticker.clone(), entries.record.date.clone(), entries.record.costbasis.to_string().clone()].join("\t")
                    );

                    entry_map.insert(key, entries.id.clone());
                }
                commands = entry_map.keys().cloned().collect();
                let selected_entry: String = Select::new("\nWhat stock would you like to sell", commands)
                    .prompt()
                    .unwrap()
                    .to_string();
                let stock_id_to_update: u32 = entry_map.get(&selected_entry).expect("Stock not found!").to_owned();
                
                number_of_shares_sold = CustomType::<f32>::new("Enter number of shares sold: ")
                    .with_placeholder("00000.00")
                    .with_default(00000.00)
                    .with_error_message("Please type a valid amount!")
                    .prompt()
                    .unwrap();

                let selected_record = record_map.get(&stock_id_to_update).expect("Stock record not found");
                let updated_shares = selected_record.shares - number_of_shares_sold;

                if updated_shares == 0.0 { 
                    self.db.drop_stock_by_id(stock_id_to_update);
                } else {
                    self.db.update_stock_shares(stock_id_to_update, updated_shares);
                }
            }
            _ => {
                panic!("Unrecognized input!");
            }
        }

        let value_received = number_of_shares_sold * cost_basis_of_shares_sold;
        let stock_cid = self.db.get_category_id(self.id, "Sold".to_string()).unwrap();
        let stock_pid = self.db.check_and_add_participant(self.id, ticker.clone(), ParticipantType::Payer);

        let sale = LedgerEntry { 
            date: sale_date.as_ref().unwrap().to_string(), 
            amount : value_received, 
            transfer_type: TransferType::DepositFromInternalAccount,
            participant_id : stock_pid,
            category_id: stock_cid, 
            description : format!("[Internal]: Sold {} shares of {} at ${} on {}.", number_of_shares_sold, ticker, sale_price, sale_date.as_ref().unwrap().clone().to_string())
        };

        self.db.add_ledger_entry(self.id, sale);

    }

    pub fn time_weighted_return(&mut self, period_start : NaiveDate, period_end : NaiveDate, ticker : Option<String>) -> f32 {
        let mut rate : f32 = 0.0;
        let mut owned_shares : HashMap<String, f32> = HashMap::new();
        let mut stock_values : HashMap<String, HashMap<String, Quote>> = HashMap::new();
        let mut vi: f32 = 0.0;
        let mut single_ticker : bool = false;
        let mut tickers = Vec::new();
        // get fixed account trsanctions within the time period
        let mut fixed_transactions = self.db.get_ledger_entries_within_timestamps(self.id, period_start, period_end).unwrap();

        let mut fixed_vi = 0.0;
        if ticker.is_none() { 
            // determining time-weighted rate of return for the entire account

            tickers = self.db.get_stock_tickers(self.id).unwrap();
            // get initial value of fixed account
            fixed_vi = self.db.get_cumulative_total_of_ledger_before_date(self.id, period_start).unwrap();
        } else {
            // determining time-weighted rate of return for the one stock
            //  - will not consider initial value (fixed_vi = 0.0) of, movements into our out of fixed account because we are only looking at one ticker

            // filter transactions that move money into and out of account
            fixed_transactions.retain(|transaction| transaction.transfer_type != TransferType::WidthdrawalToExternalAccount);
            fixed_transactions.retain(|transaction| transaction.transfer_type != TransferType::DepositFromExternalAccount);

            tickers.push(ticker.clone().expect("Ticker not provided!"));
            single_ticker = true;

            // sort out transactions if single ledger - need transactions where
            // the stock may be both the payer (when selling) and the payee (when buying)
            let payee_pid_query = self.db.get_participant_id(self.id, ticker.clone().unwrap().clone(), ParticipantType::Payee);
            if payee_pid_query.is_none() { 
                // if no payee, filter out all transactions where stock was purchased
                fixed_transactions.retain(|transaction| transaction.transfer_type != TransferType::WidthdrawalToInternalAccount);
            } else { 
                let payee_pid = payee_pid_query.unwrap();
                fixed_transactions.retain(|transaction| transaction.participant_id == payee_pid && transaction.transfer_type == TransferType::WidthdrawalToInternalAccount);
            }

            let payer_pid_query = self.db.get_participant_id(self.id, ticker.clone().unwrap().clone(), ParticipantType::Payer);
            if payer_pid_query.is_none() { 
                // if no payer, filter out all transactions stock is sold
                fixed_transactions.retain(|transaction| transaction.transfer_type != TransferType::DepositFromInternalAccount);
            } else { 
                let payer_pid = payer_pid_query.unwrap();
                fixed_transactions.retain(|transaction| transaction.participant_id == payer_pid && transaction.transfer_type == TransferType::DepositFromInternalAccount);

            }
        }

        vi = fixed_vi;

        // get history for all stocks within the account
        for ticker in tickers.clone() {
            let (transactions, initial) = 
                self.db.get_stock_history(self.id, ticker.clone(), period_start, period_end).unwrap();

            // get stock history starting a week before requested date so that we can find last open date if necessary
            let quotes = crate::stocks::get_stock_history(ticker.clone(), period_start.checked_sub_days(Days::new(7)).unwrap(), period_end).unwrap();
            
            // create a map of quotes to dates for quick lookup
            let mut quote_lookup : HashMap<String, Quote> = HashMap::new();
            for quote in quotes {
                let date_and_time = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
                let date = date_and_time.date();
                quote_lookup.insert( date.to_string(), quote.clone());
            }

            owned_shares.insert(ticker.clone(), initial.shares.clone()); 
            stock_values.insert(ticker.clone(), quote_lookup.clone());

            // ------------
            // sum up vi_s
            // ------------
            // check to see if the analyzed start date 
            // occurred when the stock market was open. 
            // choose the closing value of the stocks
            let mut first_open_stock_market_date = period_start
                .checked_sub_days(Days::new(1))
                .unwrap()
                .to_string();
            loop { 
                // does quote exist for that date? 
                if quote_lookup.get(&first_open_stock_market_date.clone()).is_none() {
                    first_open_stock_market_date = 
                        NaiveDate::parse_from_str( 
                            &first_open_stock_market_date.as_str(),
                            "%Y-%m-%d")
                            .unwrap()
                            .checked_sub_days(Days::new(1))
                            .expect("Invalid date range!")
                            .to_string();
                    continue;
                }

                // add initial value of owned stock
                vi += quote_lookup.get(&first_open_stock_market_date.clone()).expect("Stock market date not found!")
                    .clone().open as f32 * initial.shares;
                break;
            }
        }

        let mut sub_period_return: Vec<f32> = Vec::new();
        let mut cash_flow : f32 = 0.0;

        // if fixed_transactions.is_empty() { 
        //     cash_flow = 0.0;
        //     let quotes = 

        // }
        for txn in fixed_transactions {
            let date = txn.date;
            let cash_flow =
                // adding cash to account
                if txn.transfer_type == TransferType::DepositFromExternalAccount { txn.amount } else 
                // removing cash from account
                if txn.transfer_type == TransferType::WidthdrawalToExternalAccount  { -txn.amount }  else 
                // stock sale (for single stock analysis)
                if single_ticker && txn.transfer_type == TransferType::DepositFromInternalAccount { -txn.amount } else 
                // stock purchase (for single stock analysis)
                if single_ticker && txn.transfer_type == TransferType::WidthdrawalToInternalAccount { txn.amount } else 
                // otherwise, do not consider
                { 0.0 } ;

            // update shares of stock 
            if 

            for ticker in tickers { 
                // determine account wealth on date of transaction
                let stock_quote = 
                    stock_values.get(&ticker).expect(format!("Unable to find quote history for {}", ticker.clone()).as_str());

                
                
                if let Some(quote) = stock_quote.get(&date.clone()) { 
                    // check to ensure that transaction occured when the market was open
                    // otherwise, we will just consider in the next period
                    ve += quote.close * 
                }
                let quote = .expect(format!("Unable to find quote for {} and date {}"))

            }
        }
        return rate;
    }

}