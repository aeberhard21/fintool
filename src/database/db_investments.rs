use super::DbConn;
use chrono::{NaiveDate, NaiveDateTime, Days};
use rusqlite::{Error, Result, ToSql, Transaction};
use time::OffsetDateTime;
use core::num;
use std::array::IntoIter;
use std::borrow::BorrowMut;
use std::collections::VecDeque;
use std::ops::Deref;
use std::time::{Duration, UNIX_EPOCH};
use std::{collections::{HashMap, HashSet}};
use crate::stocks::{self, get_stock_history};
use yahoo_finance_api::Quote;

pub struct StockRecord {
    pub ticker: String,
    pub shares: f32,
    pub costbasis: Option<f32>,
    pub date: Option<String>,
}

impl DbConn {
    pub fn create_investment_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS investments (
            id          INTEGER NOT NULL,
            date        TEXT NOT NULL, 
            ticker      TEXT NOT NULL,
            shares      REAL NOT NULL,
            costbasis   REAL NOT NULL,
            aid         INTEGER NOT NULL, 
            FOREIGN     KEY (aid) REFERENCES accounts(id)
        )";
        match self.conn.execute(sql, ()) {
            Ok(_) => {
                println!("Created stocks table!");
            }
            Err(error) => {
                panic!("Unable to create table 'investment' because: {}", error);
            }
        }
        Ok(())
    }

    pub fn add_stock(&mut self, aid: u32, record: StockRecord) -> Result<u32> {
        let id = self.get_next_stock_id().unwrap();
        let p = rusqlite::params!(
            id,
            record.date,
            record.ticker.as_str(),
            record.shares,
            record.costbasis,
            aid
        );
        let sql = "INSERT INTO investments (id, date, ticker, shares, costbasis, aid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add stock {} for account {}: {}",
                    record.ticker.as_str(),
                    aid,
                    error
                );
            }
        }
    }

    pub fn get_stock_tickers(&mut self, aid: u32) -> Result<Vec<String>, rusqlite::Error> {
        let p = rusqlite::params![aid];
        let sql = "SELECT ticker FROM investments WHERE aid = (?1)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks: HashSet<String> = HashSet::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers: Vec<Result<String, Error>> = stmt
                    .query_map(p, |row| Ok(row.get(0)?))
                    .unwrap()
                    .collect::<Vec<_>>();
                for ticker in tickers {
                    stocks.insert(ticker.unwrap());
                }
                Ok(Vec::from_iter(stocks))
            }
            false => {
                panic!("A list of stocks do not exist for account: {}", aid);
            }
        }
    }

    pub fn get_stocks(
        &mut self,
        aid: u32,
        ticker: String,
    ) -> Result<Vec<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, ticker];
        let sql = "SELECT * FROM investments WHERE aid = (?1) and ticker LIKE (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt
                    .query_map(p, |row| {
                        Ok(StockRecord {
                            date: row.get(1)?,
                            ticker: row.get(2)?,
                            shares: row.get(3)?,
                            costbasis: row.get(4)?,
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();
                for ticker in tickers {
                    stocks.push(ticker.unwrap());
                }
                Ok(stocks)
            }
            false => {
                panic!(
                    "A list of stocks do not exist for account {} and ticker {}",
                    aid, ticker
                );
            }
        }
    }

    pub fn get_stock_history(&mut self, aid: u32, ticker: String, start: NaiveDate, end : NaiveDate) ->rusqlite::Result<(Vec<StockRecord>, StockRecord), rusqlite::Error> {
        let p = rusqlite::params![aid, ticker, start.to_string(), end.to_string()];
        let sql = "SELECT * FROM investments WHERE aid = (?1) and ticker LIKE (?2) and date >= (?3) and date <= (?4) ORDER BY date ASC";

        // let quotes = stocks::get_stock_history(ticker.clone(), start, end).unwrap();
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();
        let mut initial: StockRecord = StockRecord{shares : 0.0, ticker : ticker.clone(), costbasis : Some(0.0), date : Some(start.to_string())};

        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt
                    .query_map(p, |row| {
                        Ok(StockRecord {
                            date: row.get(1)?,
                            ticker: row.get(2)?,
                            shares: row.get(3)?,
                            costbasis: row.get(4)?,
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();
                for ticker in tickers {
                    stocks.push(ticker.unwrap());
                }
                // Ok(stocks)
            }
            false => {
                panic!(
                    "A list of stocks do not exist for account {} and ticker {}",
                    aid, ticker
                );
            }
        }

        let mut final_stocks = VecDeque::from(stocks);

        let sql: &str = "SELECT * FROM investments WHERE aid = (?1) and ticker LIKE (?2) and date < (?3) ORDER BY date ASC";
        let p = rusqlite::params![aid, ticker, start.to_string()];
        match self.conn.prepare(sql) {
            Ok(mut stmt) => {
                let exists = stmt.exists(p)?;
                if exists {
                    let previously_purchased_stock = stmt .query_map(p, |row| {
                        Ok(StockRecord {
                            date: row.get(1)?,
                            ticker: row.get(2)?,
                            shares: row.get(3)?,
                            costbasis: row.get(4)?,
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();

                    let mut num_shares : f32 = 0.0;
                    for purchase in previously_purchased_stock {
                        num_shares += purchase.unwrap().shares;
                    }

                    initial.shares = num_shares;
                } else {
                    // in this case, the user has requested to analyze stock history with a date
                    // prior to any ownership of the requested stock
                    //
                    // so, we will return first owned stock and update the period start date
                    // to be the first date that ownership started
                    let first_stock = final_stocks.pop_front().expect("Stocks not returned!");
                    initial.shares = first_stock.shares;
                    initial.date = first_stock.date;
                }
 
            }
            Err(error) => {
                panic!("Unable to retrieve stock account information: {}", error);
            }
        }

        Ok((Vec::from(final_stocks), initial))

        // return stocks;
    }

    pub fn cumulate_stocks(self: &mut DbConn, aid: u32, ticker: String) -> Vec<StockRecord> {
        let err_str = format!("Unable to retrieve stock information for account {}.", aid);
        let stocks = self.get_stocks(aid, ticker).expect(err_str.as_str());
        let mut map = HashMap::new();
        let mut cumulated_stocks = Vec::new();
        for stock in stocks {
            match map.insert(stock.ticker.clone(), stock.shares) {
                None => {
                    continue;
                }
                Some(shares) => {
                    map.insert(stock.ticker, stock.shares + shares);
                }
            }
        }
        for kv in map {
            cumulated_stocks.push(StockRecord {
                ticker: kv.0,
                shares: kv.1,
                costbasis: None,
                date: None,
            });
        }
        return cumulated_stocks;
    }

    pub fn time_weighted_return(self : &mut DbConn, quotes : Vec<Quote>, transactions : Vec<StockRecord>, initial: StockRecord) -> f32
    {
        let mut quote_lookup : HashMap<String, Quote> = HashMap::new();
        for quote in quotes {
            let date_and_time = OffsetDateTime::from(UNIX_EPOCH + Duration::from_secs(quote.timestamp));
            let date = date_and_time.date();
            quote_lookup.insert( date.to_string(), quote.clone());
        }

        let mut total_shares : f32 = initial.shares;
        let ts = VecDeque::from(transactions);
        
        let mut vi;
        let mut first_open_date = initial.date.expect("Date must be populated!");
        loop {
            if quote_lookup.get(&first_open_date.clone()).is_none() {
                first_open_date = 
                    NaiveDate::parse_from_str(&first_open_date
                    .as_str(), "%Y-%m-%d")
                    .unwrap()
                    .checked_add_days(Days::new(1))
                    .expect("Date could not be added!")
                    .to_string();
                continue;
            }
            vi = quote_lookup.get(&first_open_date.clone()).expect("I am here!").clone().open as f32 * total_shares;
            // println!("Found: {}, {}", quote_lookup.get(&first_open_date.clone()).expect("I am here!").clone().open, total_shares);
            break;
        }
        // let mut vi = quote_lookup.get(&initial.date.expect("Not found!")).expect("I am here!").clone() as f32 * total_shares;
        // println!("Initial: {}, Shares: {}", vi, total_shares);
        let mut sub_period_return = Vec::new();

        let mut cf: f32 = 0.0;

        for (i, t) in ts.iter().enumerate() { 
            
            let mut ve : f32;
            let hp : f32;
            let next_vi = t;
            let date = next_vi.date.clone().expect("Date must be populated!");
            
            // Check is found within the quote lookup. If not, this may suggest that the 
            // stock market was closed that day, e.g., weekends or holidays.
            if quote_lookup.get(&date.clone()).is_none() {

                cf += next_vi.costbasis.expect("Cost basis must be populated!") * next_vi.shares;
                total_shares += next_vi.shares;

                if i == ts.len()-1 {
                    // if this is the last entry (sorted), than there will not be
                    // any susequent period to lump the buy into so we will just 
                    // lump thte cost basis and value as one, (no growth)
                    ve = cf + vi;
                }
                else {
                    continue;

                }      
            } else {
                // add in the costs and we will consider this in the next period when 
                // the market is open
                cf += next_vi.costbasis.expect("Requires costbasis!") * next_vi.shares;

                // let fail_msg = format!("Date {} not found!", &next_vi.date.clone().expect("not populated!"));
                let fail_msg = "Not here!";
                
                total_shares += next_vi.shares;
                ve = quote_lookup.get(&date.clone()).expect(&fail_msg).clone().close as f32 * total_shares;
            }
            

            hp = (ve - (vi+cf))/(vi + cf);
            sub_period_return.push(hp);
            
            // println!("vi: {}, ve: {}, cf: {}, shares: {}, return: {}", vi, ve, cf, total_shares, hp);
            
            cf = 0.0;
            vi = ve;
        } 

        let mut twr: f32 = 1.0;
        for hp in sub_period_return {
            twr = twr * (1.0 as f32+ hp)
        }
        (twr-1.0)*100.0
    }

    pub fn get_stock_growth(self : &mut DbConn, aid : u32, ticker : String, period_start : NaiveDate, period_end : NaiveDate) -> f32 {
        let (transactions, initial) = self.get_stock_history(aid, ticker.clone(), period_start, period_end).unwrap();
        let quotes = crate::stocks::get_stock_history(ticker, NaiveDate::parse_from_str(initial.date.clone().expect("Date not populated!").as_str(), "%Y-%m-%d").unwrap(), period_end).unwrap();
        self.time_weighted_return(quotes, transactions, initial)
    }
}