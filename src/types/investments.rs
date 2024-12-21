// use super::DbConn;
use chrono::{Days, Month, NaiveDate, NaiveDateTime};
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
use crate::database::DbConn;
use yahoo_finance_api::Quote;

pub struct StockRecord {
    pub ticker: String,
    pub shares: f32,
    pub costbasis: f32,
    pub date: String,
    pub remaining : f32,
    pub ledger_id : u32
}

pub struct StockEntries {
    pub id: u32,
    pub record: StockRecord, 
}

impl DbConn {

    pub fn create_investment_purchase_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS stock_purchases (
            id          INTEGER NOT NULL PRIMARY KEY,
            date        TEXT NOT NULL, 
            ticker      TEXT NOT NULL,
            shares      REAL NOT NULL,
            costbasis   REAL NOT NULL,
            remaining   REAL NOT NULL,
            aid         INTEGER NOT NULL, 
            lid         INTEGER NOT NULL,
            FOREIGN     KEY (aid) REFERENCES accounts(id)
            FOREIGN     KEY (lid) REFERENCES ledgers(lid)
        )";
        match self.conn.execute(sql, ()) {
            Ok(_) => {
                println!("Created stocks table!");
            }
            Err(error) => {
                panic!("Unable to create table 'stock_purchases' because: {}", error);
            }
        }
        Ok(())
    }

    pub fn create_investment_sale_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS stock_sales (
            id          INTEGER NOT NULL PRIMARY KEY,
            date        TEXT NOT NULL, 
            ticker      TEXT NOT NULL,
            shares      REAL NOT NULL,
            price       REAL NOT NULL,
            aid         INTEGER NOT NULL,
            lid         INTEGER NOT NULL,
            FOREIGN     KEY (aid) REFERENCES accounts(id)
            FOREIGN     KEY (lid) REFERENCES ledgers(id)
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

    pub fn create_investment_sale_allocation_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS stock_sale_allocation (
            id          INTEGER NOT NULL PRIMARY KEY,
            purchase_id INTEGER NOT NULL, 
            sale_id     INTEGER NOT NULL,
            quantity    REAL NOT NULL,
            FOREIGN KEY (purchase_id) REFERENCES stock_purchases(id), 
            FOREIGN KEY (sale_id) REFERENCES stock_sales(id)
        )";
        match self.conn.execute(sql, ()) {
            Ok(_) => {
                println!("Created stocks allocation table!");
            }
            Err(error) => {
                panic!("Unable to create table 'stock_sale_allocation' because: {}", error);
            }
        }
        Ok(())
    }

    pub fn add_stock(&mut self, aid: u32, record: StockRecord) -> Result<u32> {
        let id = self.get_next_stock_purchase_id().unwrap();
        let p = rusqlite::params!(
            id,
            record.date,
            record.ticker.as_str(),
            record.shares,
            record.costbasis,
            record.remaining,
            aid
        );
        let sql = "INSERT INTO stock_purchases (id, date, ticker, shares, costbasis, remaining, aid) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add purchase of stock {} for account {}: {}",
                    record.ticker.as_str(),
                    aid,
                    error
                );
            }
        }
    }

    pub fn sell_stock(&mut self, aid : u32, sale_record : StockRecord ) -> Result<u32> {
        let id = self.get_next_stock_sale_id().unwrap();
        let p = rusqlite::params!(
            id,
            sale_record.date,
            sale_record.ticker.as_str(),
            sale_record.shares,
            sale_record.costbasis,
            aid
        );
        let sql = "INSERT INTO stock_sales (id, date, ticker, shares, price, aid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

        let mut stmt = self.conn.prepare(sql).unwrap();
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add sale of stock {} for account {}: {}",
                    sale_record.ticker.as_str(),
                    aid,
                    error
                );
            }
        }
    }

    pub fn add_stock_sale_allocation(&mut self, buy_id : u32, sell_id : u32, shares_allocated : f32) -> Result<u32> {
        let id = self.get_next_stock_sale_allocation_id().unwrap();
        let p = rusqlite::params!(
            id,
            buy_id, 
            sell_id, 
            shares_allocated
        );
        let sql = "INSERT INTO stock_sale_allocation (id, purchase_id, sale_id, quantity) VALUES (?1, ?2, ?3, ?4)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add allocation of stock sale {}", error
                );
            }
        }
    }

    pub fn drop_stock_by_id(&mut self, id : u32) {
        let p = rusqlite::params![id];
        let sql = "DELETE FROM stock_purchases WHERE id = (?1)";
        let mut stmt = self.conn.prepare(sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        if exists { 
            stmt.execute(p);
        } else {
            panic!("Stock id {} does not exist!", id);
        } 
    }

    pub fn update_stock_remaining(&mut self, id : u32, updated_shares : f32 ) -> Result<u32> {
        let p = rusqlite::params![id,updated_shares];
        let sql = "UPDATE stock_purchases SET remaining = (?2) WHERE id = (?1)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to update shares: {}", error
                );
            }
        }
    }
    
    pub fn get_stock_tickers(&mut self, aid: u32) -> Result<Vec<String>, rusqlite::Error> {
        let p = rusqlite::params![aid];
        let sql = "SELECT ticker FROM stock_purchases WHERE aid = (?1)";
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
    ) -> Result<Vec<StockEntries>, rusqlite::Error> {
        let p = rusqlite::params![aid, ticker];
        let sql = "SELECT * FROM stock_purchases WHERE aid = (?1) and ticker LIKE (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt
                    .query_map(p, |row| {
                        Ok(StockEntries{
                            id: row.get(0)?, 
                            record: StockRecord {
                                date: row.get(1)?,
                                ticker: row.get(2)?,
                                shares: row.get(3)?,
                                costbasis: row.get(4)?,
                                remaining: row.get(5)?,
                                ledger_id : row.get(7)?
                        }})
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
        let sql = "SELECT * FROM stock_purchases WHERE aid = (?1) and ticker LIKE (?2) and date >= (?3) and date <= (?4) ORDER BY date ASC";

        // let quotes = stocks::get_stock_history(ticker.clone(), start, end).unwrap();
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();
        let mut initial: StockRecord = StockRecord{shares : 0.0, ticker : ticker.clone(), costbasis : 0.0, date : start.checked_sub_days(Days::new(1)).unwrap().to_string(), remaining : 0.0, ledger_id : 0};

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
                            remaining : row.get(5)?,
                            ledger_id : row.get(7)?
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

        let sql: &str = "SELECT * FROM stock_purchases WHERE aid = (?1) and ticker LIKE (?2) and date < (?3) ORDER BY date ASC";
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
                            remaining: row.get(5)?,
                            ledger_id : row.get(7)?
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
                    // so, we will return 0 and set just before start date.
                    initial.shares = 0.0 as f32;
                }
 
            }
            Err(error) => {
                panic!("Unable to retrieve stock account information: {}", error);
            }
        }

        Ok((Vec::from(final_stocks), initial))

        // return stocks;
    }

    pub fn get_stock_history_ascending(&mut self, aid: u32, ticker: String) ->rusqlite::Result<Vec<StockEntries>, rusqlite::Error> {
        let p = rusqlite::params![aid, ticker];
        let sql = "SELECT * FROM stock_purchases WHERE aid = (?1) and ticker LIKE (?2) ORDER BY date ASC";

        // let quotes = stocks::get_stock_history(ticker.clone(), start, end).unwrap();
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();

        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt
                    .query_map(p, |row| {
                        Ok(StockEntries { 
                            id : row.get(0)?, 
                            record : StockRecord {
                                date: row.get(1)?,
                                ticker: row.get(2)?,
                                shares: row.get(3)?,
                                costbasis: row.get(4)?,
                                remaining : row.get(5)?,
                                ledger_id : row.get(7)?
                        }})
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

        // let mut final_stocks = VecDeque::from(stocks);
        // Ok((Vec::from(final_stocks), initial))
        Ok(stocks)
        // return stocks;
    }

    pub fn get_stock_history_descending(&mut self, aid: u32, ticker: String) ->rusqlite::Result<Vec<StockEntries>, rusqlite::Error> {
        let p = rusqlite::params![aid, ticker];
        let sql = "SELECT * FROM stock_purchases WHERE aid = (?1) and ticker LIKE (?2) ORDER BY date DESC";

        // let quotes = stocks::get_stock_history(ticker.clone(), start, end).unwrap();
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();

        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt
                    .query_map(p, |row| {
                        Ok(StockEntries { 
                            id : row.get(0)?,
                            record : StockRecord {
                                date: row.get(1)?,
                                ticker: row.get(2)?,
                                shares: row.get(3)?,
                                costbasis: row.get(4)?,
                                remaining : row.get(5)?,
                                ledger_id : row.get(7)?
                        }})
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

        // let mut final_stocks = VecDeque::from(stocks);
        // Ok((Vec::from(final_stocks), initial))
        Ok(stocks)
        // return stocks;
    }

    pub fn get_stock_current_value(&mut self, aid : u32) -> rusqlite::Result<f32,rusqlite::Error> { 
        let mut sum : f32 = 0.0;
        let p = rusqlite::params![aid];
        let sql = "
            SELECT SUM(get_stock_value(ticker) * remaining) as total_value
            FROM stock_purchases WHERE aid = (?1)";
        let mut stmt = self.conn.prepare(sql)?;
        if stmt.exists(p)? { 
            sum = stmt.query_row(p, |row| row.get(0))?;
        } else { 
            panic!("Not found!");
        }
        Ok(sum)
    }

    pub fn get_portfolio_value_before_date(&mut self, aid : u32, date : NaiveDate) -> rusqlite::Result<f32, rusqlite::Error> { 
        let mut sum : f32 = 0.0;
        let p = rusqlite::params![aid, date.to_string()];
        let sql = 
            "WITH 
                -- Purchases (converting purchases into positive amounts)
                purchases AS (
                    SELECT 
                        p.ticker as ticker, 
                        p.date AS transaction_date, 
                        p.shares, 
                        'purchase' AS transaction_type
                    FROM stock_purchases p WHERE aid = (?1)
                ),
                
                -- Sales (converting sales into negative amounts)
                sales AS (
                    SELECT 
                        s.ticker as ticker, 
                        s.date AS transaction_date, 
                        -s.shares AS shares, 
                        'sale' AS transaction_type
                    FROM stock_sales s WHERE aid = (?1)
                ),
                
                -- Combining Purchases and Sales for all tickers
                transactions AS (
                    SELECT ticker, transaction_date, shares, transaction_type
                    FROM purchases
                    UNION ALL
                    SELECT ticker, transaction_date, shares, transaction_type
                    FROM sales
                ),
                
                -- Cumulative Ownership Calculation per ticker (window function)
                cumulative AS (
                    SELECT 
                        t.ticker as ticker, 
                        t.transaction_date,
                        t.shares,
                        SUM(t.shares) OVER (PARTITION BY t.ticker ORDER BY t.transaction_date) AS cumulative_shares_owned
                    FROM transactions t
                    -- Only include transactions up to the specific date
                    WHERE t.transaction_date <= (?2)
                ),

                -- returns last recorded amount of stocks owned
                residual as (
                    SELECT
                        c.ticker,
                        MAX(transaction_date) AS final_transaction_date,
                        LAST_VALUE(cumulative_shares_owned) OVER (PARTITION BY c.ticker ORDER BY c.transaction_date ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) AS final_shares_owned
                    FROM cumulative as c 
                    GROUP BY c.ticker
                )

            -- Final query to get the cumulative shares owned for each ticker by the target date
            SELECT
                SUM(get_stock_value_on_day(ticker, (?2)) * final_shares_owned)
            FROM residual";

        
        let mut stmt = self.conn.prepare(sql)?;
        if stmt.exists(p)? { 
            sum = stmt.query_row(p, |row| row.get(0)).unwrap();
        } else { 
            panic!("Not found!");
        }
        Ok(sum)
    }

    pub fn cumulate_stocks(self: &mut DbConn, aid: u32, ticker: String) -> Vec<StockRecord> {
        let err_str = format!("Unable to retrieve stock information for account {}.", aid);
        let stocks = self.get_stocks(aid, ticker).expect(err_str.as_str());
        let mut map = HashMap::new();
        let mut cumulated_stocks = Vec::new();
        for stock in stocks {
            match map.insert(stock.record.ticker.clone(), stock.record.shares) {
                None => {
                    continue;
                }
                Some(shares) => {
                    map.insert(stock.record.ticker, stock.record.shares + shares);
                }
            }
        }
        for kv in map {
            cumulated_stocks.push(StockRecord {
                ticker: kv.0,
                shares: kv.1,
                costbasis: 0.0,
                date: "1970-01-01".to_string(),
                remaining : kv.1,
                ledger_id : 0
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
        let mut first_open_date = initial.date;
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
            let date = next_vi.date.clone();
            
            // Check is found within the quote lookup. If not, this may suggest that the 
            // stock market was closed that day, e.g., weekends or holidays.
            if quote_lookup.get(&date.clone()).is_none() {

                cf += next_vi.costbasis * next_vi.shares;
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
                cf += next_vi.costbasis * next_vi.shares;

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
        let quotes = crate::stocks::get_stock_history(ticker, NaiveDate::parse_from_str(initial.date.clone().as_str(), "%Y-%m-%d").unwrap(), period_end).unwrap();
        self.time_weighted_return(quotes, transactions, initial)
    }
}