// use super::DbConn;
use crate::database::DbConn;
use chrono::{Days, NaiveDate};
use rusqlite::{Error, Result};
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, UNIX_EPOCH};
use time::OffsetDateTime;
use yahoo_finance_api::Quote;

use super::ledger::{LedgerInfo, LedgerRecord};
use super::participants;

#[derive(Debug, Clone)]
pub struct StockInfo {
    pub shares: f32,
    pub costbasis: f32,
    pub remaining: f32,
    pub ledger_id: u32,
}

#[derive(Debug, Clone)]
pub struct StockRecord {
    pub id: u32,
    pub info: StockInfo,
    pub txn_opt: Option<LedgerInfo>
}

pub struct SaleAllocationInfo {
    pub purchase_id: u32,
    pub sale_id: u32,
    pub quantity: f32,
}

pub struct SaleAllocationRecord {
    pub id: u32,
    pub info: SaleAllocationInfo,
}

#[derive(Debug, Clone)]
pub struct StockSplitInfo {
    pub split : f32,
    pub ledger_id : u32
}

#[derive(Debug, Clone)]
pub struct StockSplitRecord {
    pub id: u32,
    pub info: StockSplitInfo,
    pub txn_opt : Option<LedgerInfo>
}

pub struct StockSplitAllocationInfo {
    pub stock_split_id : u32,
    pub stock_purchase_id : u32
}

pub struct StockSplitAllocationRecord {
    pub id : u32, 
    pub info: StockSplitAllocationInfo
}

impl DbConn {
    pub fn create_investment_purchase_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS stock_purchases (
            id          INTEGER NOT NULL,
            shares      REAL NOT NULL,
            costbasis   REAL NOT NULL,
            remaining   REAL NOT NULL,
            aid         INTEGER NOT NULL, 
            lid         INTEGER NOT NULL,
            uid         INTEGER NOT NULL, 
            PRIMARY KEY (uid, aid, id),
            FOREIGN     KEY (uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN     KEY (uid, aid, lid) REFERENCES ledgers(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN     KEY (uid) REFERENCES users(id)
        )";
        match self.conn.execute(sql, ()) {
            Ok(_) => {}
            Err(error) => {
                panic!(
                    "Unable to create table 'stock_purchases' because: {}",
                    error
                );
            }
        }
        Ok(())
    }

    pub fn create_investment_sale_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS stock_sales (
            id          INTEGER NOT NULL,
            shares      REAL NOT NULL,
            price       REAL NOT NULL,
            aid         INTEGER NOT NULL,
            lid         INTEGER NOT NULL,
            uid         INTEGER NOT NULL,
            PRIMARY KEY (uid, aid, id),
            FOREIGN     KEY (uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN     KEY (uid, aid, lid) REFERENCES ledgers(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN     KEY (uid) REFERENCES users(id)
        )";
        match self.conn.execute(sql, ()) {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create table 'investment' because: {}", error);
            }
        }
        Ok(())
    }

    pub fn create_investment_sale_allocation_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS stock_sale_allocation (
            id          INTEGER NOT NULL,
            purchase_id INTEGER NOT NULL, 
            sale_id     INTEGER NOT NULL,
            quantity    REAL NOT NULL,
            uid         INTEGER NOT NULL,
            aid         INTEGER NOT NULL,
            PRIMARY KEY (uid, aid, id),
            FOREIGN KEY (uid, aid, purchase_id) REFERENCES stock_purchases(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN KEY (uid, aid, sale_id) REFERENCES stock_sales(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN     KEY (uid) REFERENCES users(id),
            FOREIGN     KEY (uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE
        )";
        match self.conn.execute(sql, ()) {
            Ok(_) => {}
            Err(error) => {
                panic!(
                    "Unable to create table 'stock_sale_allocation' because: {}",
                    error
                );
            }
        }
        Ok(())
    }

    pub fn create_stock_split_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS stock_splits (
            id          INTEGER NOT NULL,
            split       REAL NOT NULL,
            aid         INTEGER NOT NULL,
            lid         INTEGER NOT NULL,
            uid         INTEGER NOT NULL,
            PRIMARY KEY (uid, aid, id),
            FOREIGN KEY (uid, aid, lid) REFERENCES ledgers(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN KEY (uid) REFERENCES users(id),
            FOREIGN KEY (uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE
        )";
        match self.conn.execute(sql, ()) {
            Ok(_) => {}
            Err(error) => {
                panic!(
                    "Unable to create table 'stock_splits' because: {}",
                    error
                );
            }
        }
        Ok(())
    }

    pub fn create_stock_split_allocation_table(&mut self) -> Result<()> { 
        let sql: &str = "CREATE TABLE IF NOT EXISTS stock_split_allocations (
            id INTEGER NOT NULL, 
            stock_purchase_id INTEGER NOT NULL, 
            stock_split_id INTEGER NOT NULL, 
            uid INTEGER NOT NULL,
            aid INTEGER NOT NULL,
            PRIMARY KEY (uid, aid, id),
            FOREIGN KEY (uid, aid, stock_purchase_id) REFERENCES stock_purchases(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN KEY (uid, aid, stock_split_id) REFERENCES stock_splits(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN KEY (uid) REFERENCES users(id),
            FOREIGN KEY (uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE
        )";
        match self.conn.execute(sql, ())  {
            Ok(_) => {}
            Err(error) => { 
                panic!(
                    "Unable to create 'stock_split_allocations_table' because :{}",
                    error
                );
            }
        }
        Ok(())
    }

    pub fn add_stock_purchase(&mut self, uid: u32, aid: u32, record: StockInfo) -> Result<u32> {
        let id = self.get_next_stock_purchase_id(uid, aid).unwrap();
        let p = rusqlite::params!(
            id,
            record.shares,
            record.costbasis,
            record.remaining,
            aid,
            record.ledger_id,
            uid,
        );
        let sql = "INSERT INTO stock_purchases (id, shares, costbasis, remaining, aid, lid, uid) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add purchase of stock for account {}: {}",
                    aid,
                    error
                );
            }
        }
    }

    pub fn check_and_get_stock_purchase_record_matching_from_ledger_id(
        &mut self,
        uid : u32,
        aid : u32,
        ledger_id: u32,
    ) -> rusqlite::Result<Option<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let sql = "SELECT id, shares, costbasis, remaining, lid FROM stock_purchases WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(StockRecord {
                        id: row.get(0)?,
                        info: StockInfo {
                            shares: row.get(1)?,
                            costbasis: row.get(2)?,
                            remaining: row.get(3)?,
                            ledger_id: row.get(4)?,
                        },
                        txn_opt: None,
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }

    pub fn check_and_get_stock_purchase_record_matching_from_purchase_id(
        &mut self,
        uid: u32,
        aid : u32,
        ledger_id: u32,
    ) -> rusqlite::Result<Option<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let sql = "SELECT id, shares, costbasis, remaining, lid FROM stock_purchases WHERE id = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(StockRecord {
                        id: row.get(0)?,
                        info: StockInfo {
                            shares: row.get(1)?,
                            costbasis: row.get(2)?,
                            remaining: row.get(3)?,
                            ledger_id: row.get(4)?,
                        },
                        txn_opt: None,
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }

    pub fn check_and_get_stock_sale_record_matching_from_ledger_id(
        &mut self,
        uid : u32, 
        aid : u32,
        ledger_id: u32,
    ) -> rusqlite::Result<Option<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let sql = "SELECT id, shares, price, lid FROM stock_sales WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(StockRecord {
                        id: row.get(0)?,
                        info: StockInfo {
                            shares: row.get(1)?,
                            costbasis: row.get(2)?,
                            remaining: 0.0,
                            ledger_id: row.get(3)?,
                        },
                        txn_opt: None
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }

    pub fn check_and_get_stock_split_record_matching_from_ledger_id(
        &mut self,
        uid : u32,
        aid : u32,
        ledger_id: u32,
    ) -> rusqlite::Result<Option<StockSplitRecord>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let sql = "SELECT id, split, lid FROM stock_splits WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(StockSplitRecord {
                        id: row.get(0)?,
                        info: StockSplitInfo {
                            split : row.get(1)?,
                            ledger_id : row.get(2)?
                            
                        },
                        txn_opt : None
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }


    pub fn add_stock_sale(&mut self, uid: u32, aid: u32, sale_record: StockInfo) -> Result<u32> {
        let id = self.get_next_stock_sale_id(uid, aid).unwrap();
        let p = rusqlite::params!(
            id,
            sale_record.shares,
            sale_record.costbasis,
            aid,
            sale_record.ledger_id,
            uid, 
        );
        let sql = "INSERT INTO stock_sales (id, shares, price, aid, lid, uid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";

        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add sale of stock for account {}: {}",
                    aid,
                    error
                );
            }
        }
    }

    pub fn add_stock_sale_allocation(
        &mut self,
        uid : u32, 
        aid : u32, 
        buy_id: u32,
        sell_id: u32,
        shares_allocated: f32,
    ) -> Result<u32> {
        let id = self.get_next_stock_sale_allocation_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, buy_id, sell_id, shares_allocated, uid, aid);
        let sql = "INSERT INTO stock_sale_allocation (id, purchase_id, sale_id, quantity, uid, aid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add allocation of stock sale {}", error);
            }
        }
    }

    pub fn add_stock_split(
        &mut self,
        uid: u32,
        aid: u32,
        split: f32,
        lid:  u32, 
    ) -> Result<u32, rusqlite::Error> {
        let split_id = self.get_next_stock_split_id(uid, aid).unwrap();
        let p = rusqlite::params!(split_id, split, aid, lid, uid);
        let sql =
            "INSERT INTO stock_splits (id, split, aid, lid, uid) VALUES (?1, ?2, ?3, ?4, ?5)";
        let row = match self.conn.execute(sql, p) {
            Ok(_) => split_id,
            Err(error) => {
                panic!("Unable to add allocation of stock sale {}", error)
            }
        };
        Ok(row)
    }

    pub fn add_stock_split_allocation(
        &mut self,
        uid : u32,
        aid : u32,
        allocation: StockSplitAllocationInfo
    ) -> Result<u32> {
        let id = self.get_next_stock_split_allocation_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, allocation.stock_purchase_id, allocation.stock_split_id, uid, aid);
        let sql = "INSERT INTO stock_split_allocations (id, stock_purchase_id, stock_split_id, uid, aid) VALUES (?1, ?2, ?3, ?4, ?5)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add allocation of stock sale {}", error);
            }
        }
    }


    pub fn drop_stock_by_id(&mut self, uid: u32, aid : u32, id: u32) {
        let p = rusqlite::params![id,  uid, aid];
        let sql = "DELETE FROM stock_purchases WHERE id = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        if exists {
            stmt.execute(p).unwrap();
        } else {
            panic!("Stock id {} does not exist!", id);
        }
    }

    pub fn remove_stock_sale(&mut self, uid : u32, aid :u32, ledger_id: u32) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let id_sql = "SELECT id FROM stock_sales WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = self.conn.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }

        let rm_sql = "DELETE FROM stock_sales WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        stmt = self.conn.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql = "UPDATE stock_sales SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock sale ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET ssid = ssid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update 'ssid' value in 'user_account_info': {}", error);
            }
        }

        return Ok(Some(id));
    }

    pub fn remove_stock_purchase(
        &mut self,
        uid  : u32,
        aid : u32,
        ledger_id: u32,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let id_sql = "SELECT id FROM stock_purchases WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = self.conn.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let rm_sql = "DELETE FROM stock_purchases WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        stmt = self.conn.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql = "UPDATE stock_purchases SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock purchase ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET spid = spid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update 'spid' value in 'user_account_info': {}", error);
            }
        }

        return Ok(Some(id));
    }

    pub fn remove_stock_split(
        &mut self,
        uid : u32, 
        aid : u32,
        ledger_id: u32,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let id_sql = "SELECT id FROM stock_splits WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = self.conn.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let rm_sql = "DELETE FROM stock_splits WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        stmt = self.conn.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql = "UPDATE stock_splits SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock split ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET splid = splid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update 'splid' value in 'user_account_info': {}", error);
            }
        }

        return Ok(Some(id));
    }

    pub fn remove_stock_sale_allocation(
        &mut self,
        uid : u32,
        aid : u32, 
        id: u32,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![id, uid, aid];
        let id_sql = "SELECT id FROM stock_sale_allocation WHERE id = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = self.conn.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let rm_sql = "DELETE FROM stock_sale_allocation WHERE id = (?1) and uid = (?2) and aid = (?3)";
        stmt = self.conn.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql = "UPDATE stock_sale_allocation SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock split ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET said = said - 1 WHERE uid = ?1 and aid = ?2";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update 'said' value in 'user_account_info': {}", error);
            }
        }

        return Ok(Some(id));
    }

    pub fn remove_stock_split_allocation(
        &mut self,
        uid: u32,
        aid : u32,
        id: u32,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![id, uid, aid];
        let id_sql = "SELECT id FROM stock_split_allocations WHERE id = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = self.conn.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let rm_sql = "DELETE FROM stock_split_allocations WHERE id = (?1) and uid = (?2) and aid = (?3)";
        stmt = self.conn.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql = "UPDATE stock_split_allocations SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock sale ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET stock_split_allocation_id = stock_split_allocation_id - 1 WHERE uid = ?1 and aid = ?2";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update 'ssid' value in 'stock_split_allocation_id': {}", error);
            }
        }

        return Ok(Some(id));
    }


    pub fn update_stock_purchase(
        &mut self,
        uid : u32,
        aid : u32,
        updated_info: StockInfo,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![updated_info.ledger_id,  uid, aid];
        let id_sql = "SELECT id FROM stock_purchases WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = self.conn.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let p = rusqlite::params![
            updated_info.ledger_id,
            updated_info.shares,
            updated_info.costbasis,
            updated_info.remaining, 
            uid,
            aid
        ];
        let update_sql = "UPDATE stock_purchases SET shares = (?2), costbasis = (?3), remaining = (?4) WHERE lid = (?1) and uid = (?5) and aid = (?6)";
        stmt = self.conn.prepare(update_sql)?;
        stmt.execute(p)?;
        return Ok(Some(id));
    }

    pub fn update_stock_sale(
        &mut self,
        uid :  u32,
        aid : u32, 
        updated_info: StockInfo,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![updated_info.ledger_id, uid, aid];
        let id_sql = "SELECT id FROM stock_sales WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = self.conn.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let p = rusqlite::params![
            updated_info.ledger_id,
            updated_info.shares,
            updated_info.costbasis,
            uid, 
            aid
        ];
        let update_sql = "UPDATE stock_sales SET shares = (?2), price = (?3) WHERE lid = (?1) and uid =(?4) and aid = (?5)";
        stmt = self.conn.prepare(update_sql)?;
        stmt.execute(p)?;
        return Ok(Some(id));
    }

    pub fn update_stock_remaining(&mut self, uid : u32, aid :u32, id: u32, updated_shares: f32) -> Result<u32> {
        let p = rusqlite::params![id, updated_shares, uid, aid];
        let sql = "UPDATE stock_purchases SET remaining = (?2) WHERE id = (?1) and uid = (?3) and aid = (?4)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to update shares: {}", error);
            }
        }
    }

    pub fn add_to_stock_remaining(&mut self, uid: u32, aid : u32, id: u32, shares_to_add: f32) -> Result<u32> {
        let p = rusqlite::params![id, shares_to_add, uid, aid];
        let sql = "UPDATE stock_purchases SET remaining = remaining + (?2) WHERE id = (?1) and uid = (?3) and aid = (?4)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to update shares: {}", error);
            }
        }
    }

    pub fn update_cost_basis(&mut self, uid : u32, aid : u32, id: u32, updated_costbasis: f32) -> Result<u32> {
        let p = rusqlite::params![id, updated_costbasis, uid, aid];
        let sql = "UPDATE stock_purchases SET costbasis = (?2) WHERE id = (?1) and uid = (?3) and aid = (?4)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to update shares: {}", error);
            }
        }
    }

    pub fn get_stock_tickers(&mut self, uid: u32, aid: u32) -> Result<Vec<String>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT pid FROM ledgers INNER JOIN stock_purchases ON stock_purchases.lid = ledgers.id WHERE ledgers.aid = (?1) and ledgers.uid = (?2)";
        let mut pids = Vec::new();
        {
            let mut stmt = self.conn.prepare(sql)?;
            let exists = stmt.exists(p)?;
            match exists {
                true => {
                    stmt = self.conn.prepare(sql)?;
                    pids = stmt
                        .query_map(p, |row| Ok(row.get(0)?))
                        .unwrap()
                        .collect::<Vec<_>>();
                }
                false => {
                    panic!("A list of stocks do not exist for account: {}", aid);
                }
            }
        }

        let mut stocks : Vec<String> = Vec::new();
        for pid in pids {
            let ticker = self.get_participant(uid,aid, pid.unwrap()).unwrap();
            stocks.push(ticker);
        }

        Ok(stocks)

    }

    pub fn get_stock_sale_allocation_for_sale_id(
        &mut self,
        uid : u32, 
        aid : u32,
        sale_id: u32,
    ) -> Result<Vec<SaleAllocationRecord>, rusqlite::Error> {
        let p = rusqlite::params![sale_id, uid, aid];
        let sql = "SELECT * FROM stock_sale_allocation WHERE sale_id = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut records: Vec<SaleAllocationRecord> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let wrapped_records: Vec<Result<SaleAllocationRecord, Error>> = stmt
                    .query_map(p, |row| {
                        Ok(SaleAllocationRecord {
                            id: row.get(0)?,
                            info: SaleAllocationInfo {
                                purchase_id: row.get(1)?,
                                sale_id: row.get(2)?,
                                quantity: row.get(3)?,
                            },
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();
                for wrapped_record in wrapped_records {
                    records.push(wrapped_record.unwrap());
                }
                Ok(records)
            }
            false => {
                panic!(
                    "An allocation of stock sale does not exist for: {}",
                    sale_id
                );
            }
        }
    }

    pub fn get_stock_split_allocation_for_stock_split_id(
        &mut self,
        uid : u32,
        aid : u32, 
        split_id: u32,
    ) -> Result<Vec<StockSplitAllocationRecord>, rusqlite::Error> {
        let p = rusqlite::params![split_id, uid, aid];
        let sql = "SELECT id, stock_split_id, stock_purchase_id FROM stock_split_allocations WHERE stock_split_id = (?1) and uid = (?2) and aid = (?3)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut records: Vec<StockSplitAllocationRecord> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let wrapped_records: Vec<Result<StockSplitAllocationRecord, Error>> = stmt
                    .query_map(p, |row| {
                        Ok(StockSplitAllocationRecord {
                            id: row.get(0)?,
                            info: StockSplitAllocationInfo {
                                stock_split_id: row.get(1)?,
                                stock_purchase_id: row.get(2)?,
                            },
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();
                for wrapped_record in wrapped_records {
                    records.push(wrapped_record.unwrap());
                }
                Ok(records)
            }
            false => {
                panic!(
                    "An allocation of stock splits does not exist for: {}",
                    split_id
                );
            }
        }
    }

    pub fn get_stocks(
        &mut self,
        uid : u32,
        aid: u32,
        ticker: String,
    ) -> Result<Vec<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, ticker, uid];
        let sql = "
            SELECT
                stock_purchases.id, shares, costbasis, remaining, lid 
            FROM stock_purchases 
            INNER JOIN ledgers, people ON 
                stock_purchases.lid = ledgers.id AND
                stock_purchases.uid = ledgers.uid AND
                ledgers.pid = people.id
            WHERE 
                ledgers.aid = (?1) and 
                ledgers.uid = (?3) and
                people.name LIKE (?2)
            ";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt
                    .query_map(p, |row| {
                        Ok(StockRecord {
                            id: row.get(0)?,
                            info: StockInfo {
                                shares: row.get(1)?,
                                costbasis: row.get(2)?,
                                remaining: row.get(3)?,
                                ledger_id: row.get(4)?,
                            },
                            txn_opt: None
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

    pub fn get_stock_history(
        &mut self,
        aid: u32,
        ticker: String,
        start: NaiveDate,
        end: NaiveDate,
    ) -> rusqlite::Result<(Vec<StockInfo>, StockInfo), rusqlite::Error> {
        let p = rusqlite::params![aid, ticker, start.to_string(), end.to_string()];
        let sql = "
            SELECT
                shares, costbasis, remaining, lid 
            FROM stock_purchases 
            INNER JOIN ledgers, people ON 
                stock_purchases.lid = ledgers.id AND
                stock_purchases.uid = ledgers.uid AND
                ledgers.pid = people.id
            WHERE 
                ledgers.aid = (?1) and people.name LIKE (?2) and ledgers.date >= (3) and ledgers.date <= (?4)
            ORDER BY
                ledgers.date ASC
            ";
    
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();
        let mut initial: StockInfo = StockInfo {
            shares: 0.0,
            costbasis: 0.0,
            remaining: 0.0,
            ledger_id: 0,
        };

        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt
                    .query_map(p, |row| {
                        Ok(StockInfo {
                            shares: row.get(1)?,
                            costbasis: row.get(2)?,
                            remaining: row.get(3)?,
                            ledger_id: row.get(4)?,
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

        let final_stocks = VecDeque::from(stocks);

        let sql: &str = "
            SELECT
                shares, costbasis, remaining, lid 
            FROM stock_purchases 
            INNER JOIN ledgers, people ON 
                stock_purchases.lid = ledgers.id AND
                stock_purchases.uid = ledgers.uid AND
                ledgers.pid = people.id
            WHERE 
                ledgers.aid = (?1) and people.name LIKE (?2) and ledgers.date < (?3)
            ORDER BY
                ledgers.date ASC
            ";
        let p = rusqlite::params![aid, ticker, start.to_string()];
        match self.conn.prepare(sql) {
            Ok(mut stmt) => {
                let exists = stmt.exists(p)?;
                if exists {
                    let previously_purchased_stock = stmt
                        .query_map(p, |row| {
                            Ok(StockInfo {
                                shares: row.get(1)?,
                                costbasis: row.get(2)?,
                                remaining: row.get(3)?,
                                ledger_id: row.get(4)?,
                            })
                        })
                        .unwrap()
                        .collect::<Vec<_>>();

                    let mut num_shares: f32 = 0.0;
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

    pub fn get_stock_history_ascending(
        &mut self,
        uid : u32, 
        aid: u32,
        ticker: String,
    ) -> rusqlite::Result<Vec<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, ticker, uid];
        let sql = "
            SELECT
                stock_purchases.id, shares, costbasis, remaining, lid 
            FROM stock_purchases 
            INNER JOIN ledgers, people ON 
                stock_purchases.lid = ledgers.id AND
                stock_purchases.uid = ledgers.uid AND
                ledgers.pid = people.id
            WHERE 
                ledgers.aid = (?1) and 
                ledgers.uid = (?3) and 
                people.name LIKE (?2)
            ORDER BY
                ledgers.date ASC
            ";
        // let quotes = stocks::get_stock_history(ticker.clone(), start, end).unwrap();
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();

        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt
                    .query_map(p, |row| {
                        Ok(StockRecord {
                            id: row.get(0)?,
                            info: StockInfo {
                                shares: row.get(1)?,
                                costbasis: row.get(2)?,
                                remaining: row.get(3)?,
                                ledger_id: row.get(4)?,
                            },
                            txn_opt: None
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
        Ok(stocks)
    }

    pub fn get_stock_history_descending(
        &mut self,
        uid : u32, 
        aid: u32,
        ticker: String,
    ) -> rusqlite::Result<Vec<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, ticker, uid];
        let sql = "SELECT * FROM stock_purchases WHERE aid = (?1) and ticker LIKE (?2) ORDER BY date DESC";
        let sql = "
            SELECT
                stock_purchases.id, shares, costbasis, remaining, lid 
            FROM stock_purchases 
            INNER JOIN ledgers, people ON 
                stock_purchases.lid = ledgers.id AND
                ledgers.pid = people.id
            WHERE 
                ledgers.aid = (?1) and 
                ledgers.uid = (?3) and
                people.name LIKE (?2)
            ORDER BY
                ledgers.date DESC
            ";

        // let quotes = stocks::get_stock_history(ticker.clone(), start, end).unwrap();
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();

        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let tickers = stmt
                    .query_map(p, |row| {
                        Ok(StockRecord {
                            id: row.get(0)?,
                            info: StockInfo {
                                shares: row.get(1)?,
                                costbasis: row.get(2)?,
                                remaining: row.get(3)?,
                                ledger_id: row.get(4)?,
                            },
                            txn_opt: None
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();
                for ticker in tickers {
                    stocks.push(ticker.unwrap());
                }
            }
            false => {
                panic!(
                    "A list of stocks do not exist for account {} and ticker {}",
                    aid, ticker
                );
            }
        }
        Ok(stocks)
    }

    pub fn get_stock_current_value(&mut self, uid: u32, aid: u32) -> rusqlite::Result<f32, rusqlite::Error> {
        let sum: f32;
        let p = rusqlite::params![aid,  uid];
        let sql = "
            SELECT SUM(get_stock_value(ticker) * remaining) as total_value
            FROM stock_purchases WHERE aid = (?1) and uid = (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        if stmt.exists(p)? {
            sum = stmt.query_row(p, |row| row.get(0))?;
        } else {
            panic!("Not found!");
        }
        Ok(sum)
    }

    pub fn get_portfolio_value_before_date(
        &mut self,
        uid: u32,
        aid: u32,
        date: NaiveDate,
    ) -> rusqlite::Result<f32, rusqlite::Error> {
        let mut sum: f32 = 0.0;
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
                    FROM stock_purchases p WHERE aid = (?1) and uid = (?2)
                ),
                
                -- Sales (converting sales into negative amounts)
                sales AS (
                    SELECT 
                        s.ticker as ticker, 
                        s.date AS transaction_date, 
                        -s.shares AS shares, 
                        'sale' AS transaction_type
                    FROM stock_sales s WHERE aid = (?1) and uid = (?2)
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
}
