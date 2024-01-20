use rusqlite::{Result, Error};
use super::DbConn;
use std::collections::{HashSet, HashMap};
// use crate::database::db_banks::BankRecord;

pub struct StockRecord {
    pub ticker: String,
    pub shares: f32,
    pub costbasis: Option<f32>,
    pub date: Option<i64>
}

impl DbConn {
    pub fn create_investment_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS investments (
            id          INTEGER NOT NULL,
            date        INTEGER NOT NULL, 
            ticker      TEXT NOT NULL,
            shares      REAL NOT NULL,
            costbasis   REAL NOT NULL,
            aid         INTEGER NOT NULL, 
            FOREIGN     KEY (aid) REFERENCES accounts(id)
        )";
        match self.conn.execute(sql, ()){
            Ok(_) => {
                println!("Created stocks table!");
            }
            Err(error) => {
                panic!("Unable to create table 'investment' because: {}", error);
            }
        }
        Ok(())
    }

    pub fn add_stock(&mut self, aid : u32, record: StockRecord) -> Result<u32> {
        let id = self.get_next_stock_id().unwrap();
        let p = rusqlite::params!(id, record.date, record.ticker.as_str(), record.shares, record.costbasis, aid);
        let sql = "INSERT INTO investments (id, date, ticker, shares, costbasis, aid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
        match self.conn.execute(sql, p) {
            Ok(_) => {
                Ok(id)
            }
            Err(error) => {
                panic!("Unable to add stock {} for account {}: {}", record.ticker.as_str(), aid, error);
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
                let tickers: Vec<Result<String, Error>> = stmt.query_map(p, |row| {Ok(row.get(0)?)}).unwrap().collect::<Vec<_>>();
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

    pub fn get_stocks(&mut self, aid: u32, ticker: String) -> Result<Vec<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, ticker];
        let sql = "SELECT * FROM investments WHERE aid = (?1) and ticker LIKE (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt.query_map(p, |row| {Ok(StockRecord {date: row.get(1)?, ticker: row.get(2)?, shares: row.get(3)?, costbasis: row.get(4)? })}).unwrap().collect::<Vec<_>>();
                for ticker in tickers {
                    stocks.push(ticker.unwrap());
                }
                Ok(stocks)
            } 
            false => {
                panic!("A list of stocks do not exist for account {} and ticker {}", aid, ticker);
            }
        }
    }

    pub fn cumulate_stocks(self: &mut DbConn, aid: u32, ticker :String) -> Vec<StockRecord> {
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
            cumulated_stocks.push(StockRecord { ticker: kv.0, shares : kv.1, costbasis: None, date: None } );
        }
        return cumulated_stocks;
    }

    // pub fn get_stock_info(&mut self, aid: u32, ticker: String) -> Result<Vec<StockRecord>, Error> {
    //     let p = rusqlite::params![aid, ticker.as_str()];
    //     let sql = "SELECT * FROM investments WHERE aid = (?1) and ticker = (?2)";
    //     let mut stmt = self.conn.prepare(sql).expect("Unable to prepare SQL statement!");
    //     let exists = stmt.exists(p).expect("Unable to determine if query exists!");
    //     let mut stocks:  Vec<StockRecord> = Vec::new();
    //     match exists {
    //         true => {
    //             stmt = self.conn.prepare(sql).expect("Unable to prepare sql statement!");

    //             let rows: Vec<Result<StockRecord, Error>> = stmt.query_map(p, |row: &rusqlite::Row<'_>| 
    //                 {
    //                     Ok( StockRecord {date: row.get(1)?, ticker: row.get(2)?, shares: row.get(3)?, costbasis: row.get(4)? } )
    //                 }                    
    //             ).unwrap().collect::<Vec<_>>();

    //             for row in rows {
    //                 stocks.push(row.unwrap());
    //             }

    //             return Ok(stocks);
    //         }
    //         false => {
    //             panic!("A list of stock information does not exist for the selected stock: {}", ticker);
    //         }
    //     }

    // }
}