// use super::DbConn;
use crate::database::DbConn;
use chrono::{Days, NaiveDate};
use rusqlite::{Error, Result};
use shared_lib::stocks::get_stock_at_close;
use shared_lib::TransferType;
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
    pub txn_opt: Option<LedgerInfo>,
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
    pub split: f32,
    pub ledger_id: u32,
}

#[derive(Debug, Clone)]
pub struct StockSplitRecord {
    pub id: u32,
    pub info: StockSplitInfo,
    pub txn_opt: Option<LedgerInfo>,
}

pub struct StockSplitAllocationInfo {
    pub stock_split_id: u32,
    pub stock_purchase_id: u32,
}

pub struct StockSplitAllocationRecord {
    pub id: u32,
    pub info: StockSplitAllocationInfo,
}

impl DbConn {
    pub fn create_investment_purchase_table(&self) -> Result<()> {
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
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, ()) {
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

    pub fn create_investment_sale_table(&self) -> Result<()> {
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
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, ()) {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create table 'investment' because: {}", error);
            }
        }
        Ok(())
    }

    pub fn create_investment_sale_allocation_table(&self) -> Result<()> {
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
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, ()) {
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

    pub fn create_stock_split_table(&self) -> Result<()> {
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
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, ()) {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create table 'stock_splits' because: {}", error);
            }
        }
        Ok(())
    }

    pub fn create_stock_split_allocation_table(&self) -> Result<()> {
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
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, ()) {
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

    pub fn add_stock_purchase(&self, uid: u32, aid: u32, record: StockInfo) -> Result<u32> {
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
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add purchase of stock for account {}: {}",
                    aid, error
                );
            }
        }
    }

    pub fn check_and_get_stock_purchase_record_matching_from_ledger_id(
        &self,
        uid: u32,
        aid: u32,
        ledger_id: u32,
    ) -> rusqlite::Result<Option<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let sql = "SELECT id, shares, costbasis, remaining, lid FROM stock_purchases WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;

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
        &self,
        uid: u32,
        aid: u32,
        ledger_id: u32,
    ) -> rusqlite::Result<Option<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let sql = "
            SELECT 
                p.id, 
                p.shares, 
                p.costbasis, 
                p.remaining, 
                p.lid, 
                l.date, 
                l.amount, 
                l.transfer_type,
                l.pid, 
                l.cid,
                l.desc,
            FROM stock_purchases p 
            INNER JOIN ledgers l ON 
                p.aid = l.aid and  
                p.uid = l.uid and
                p.lid = l.id
            WHERE p.id = (?1) and p.uid = (?2) and p.aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(StockRecord {
                        id: row.get(0)?,
                        info: StockInfo {
                            shares: row.get(1)?,
                            costbasis: row.get(2)?,
                            remaining: row.get(3)?,
                            ledger_id: row.get(4)?,
                        },
                        txn_opt: Some(LedgerInfo {
                            date: row.get(5)?,
                            amount: row.get(6)?,
                            transfer_type: TransferType::from(row.get::<_, u32>(7)? as u32),
                            participant: row.get(8)?,
                            category_id: row.get(9)?,
                            description: row.get(10)?,
                        }),
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }

    pub fn check_and_get_stock_sale_record_matching_from_ledger_id(
        &self,
        uid: u32,
        aid: u32,
        ledger_id: u32,
    ) -> rusqlite::Result<Option<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let sql = "SELECT id, shares, price, lid FROM stock_sales WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(StockRecord {
                        id: row.get(0)?,
                        info: StockInfo {
                            shares: row.get(1)?,
                            costbasis: row.get(2)?,
                            remaining: 0.0,
                            ledger_id: row.get(3)?,
                        },
                        txn_opt: None,
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }

    pub fn check_and_get_stock_sale_record_matching_from_sale_id(
        &self,
        uid: u32,
        aid: u32,
        sale_id: u32,
    ) -> rusqlite::Result<Option<StockRecord>, rusqlite::Error> {
        let p = rusqlite::params![sale_id, uid, aid];
        let sql = "
            SELECT 
                s.id, 
                s.shares, 
                s.price, 
                s.lid, 
                l.date, 
                l.amount, 
                l.transfer_type,
                l.pid, 
                l.cid,
                l.desc,
            FROM stock_sales s 
            INNER JOIN ledgers l ON 
                s.aid = l.aid and  
                s.uid = l.uid and
                s.lid = l.id
            WHERE s.id = (?1) and s.uid = (?2) and s.aid = (?3)
        ";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(StockRecord {
                        id: row.get(0)?,
                        info: StockInfo {
                            shares: row.get(1)?,
                            costbasis: row.get(2)?,
                            remaining: 0.0,
                            ledger_id: row.get(3)?,
                        },
                        txn_opt: Some(LedgerInfo {
                            date: row.get(4)?,
                            amount: row.get(5)?,
                            transfer_type: TransferType::from(row.get::<_, u32>(6)? as u32),
                            participant: row.get(7)?,
                            category_id: row.get(8)?,
                            description: row.get(9)?,
                        }),
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }

    pub fn check_and_get_stock_split_record_matching_from_ledger_id(
        &self,
        uid: u32,
        aid: u32,
        ledger_id: u32,
    ) -> rusqlite::Result<Option<StockSplitRecord>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let sql = "
        SELECT 
                s.id, 
                s.split, 
                s.lid, 
                l.date, 
                l.amount, 
                l.transfer_type,
                l.pid, 
                l.cid,
                l.desc,
            FROM stock_splits s 
            INNER JOIN ledgers l ON 
                s.aid = l.aid and  
                s.uid = l.uid and
                s.lid = l.id
            WHERE s.lid = (?1) and s.uid = (?2) and s.aid = (?3)
        ";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(StockSplitRecord {
                        id: row.get(0)?,
                        info: StockSplitInfo {
                            split: row.get(1)?,
                            ledger_id: row.get(2)?,
                        },
                        txn_opt: Some(LedgerInfo {
                            date: row.get(3)?,
                            amount: row.get(4)?,
                            transfer_type: TransferType::from(row.get::<_, u32>(5)? as u32),
                            participant: row.get(6)?,
                            category_id: row.get(7)?,
                            description: row.get(8)?,
                        }),
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }

    pub fn check_and_get_stock_sale_allocation_record_matching_from_purchase_id(
        &self,
        uid: u32,
        aid: u32,
        purchase_id: u32,
    ) -> rusqlite::Result<Option<Vec<SaleAllocationRecord>>, rusqlite::Error> {
        let p = rusqlite::params![purchase_id, uid, aid];
        let sql = "SELECT id, purchase_id, sale_id, quantity FROM stock_sale_allocation WHERE purchase_id = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut sale_allocation_records = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;

                let records = stmt
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
                for record in records {
                    sale_allocation_records.push(record.unwrap());
                }
                Ok(Some(sale_allocation_records))
            }
            false => Ok(None),
        }
    }

    pub fn add_stock_sale(&self, uid: u32, aid: u32, sale_record: StockInfo) -> Result<u32> {
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
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add sale of stock for account {}: {}", aid, error);
            }
        }
    }

    pub fn add_stock_sale_allocation(
        &self,
        uid: u32,
        aid: u32,
        buy_id: u32,
        sell_id: u32,
        shares_allocated: f32,
    ) -> Result<u32> {
        let id = self.get_next_stock_sale_allocation_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, buy_id, sell_id, shares_allocated, uid, aid);
        let sql = "INSERT INTO stock_sale_allocation (id, purchase_id, sale_id, quantity, uid, aid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add allocation of stock sale {}", error);
            }
        }
    }

    pub fn add_stock_split(
        &self,
        uid: u32,
        aid: u32,
        split: f32,
        lid: u32,
    ) -> Result<u32, rusqlite::Error> {
        let split_id = self.get_next_stock_split_id(uid, aid).unwrap();
        let p = rusqlite::params!(split_id, split, aid, lid, uid);
        let sql = "INSERT INTO stock_splits (id, split, aid, lid, uid) VALUES (?1, ?2, ?3, ?4, ?5)";
        let conn_lock = self.conn.lock().unwrap();
        let row = match conn_lock.execute(sql, p) {
            Ok(_) => split_id,
            Err(error) => {
                panic!("Unable to add allocation of stock sale {}", error)
            }
        };
        Ok(row)
    }

    pub fn add_stock_split_allocation(
        &self,
        uid: u32,
        aid: u32,
        allocation: StockSplitAllocationInfo,
    ) -> Result<u32> {
        let id = self.get_next_stock_split_allocation_id(uid, aid).unwrap();
        let p = rusqlite::params!(
            id,
            allocation.stock_purchase_id,
            allocation.stock_split_id,
            uid,
            aid
        );
        let sql = "INSERT INTO stock_split_allocations (id, stock_purchase_id, stock_split_id, uid, aid) VALUES (?1, ?2, ?3, ?4, ?5)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add allocation of stock sale {}", error);
            }
        }
    }

    pub fn drop_stock_by_id(&self, uid: u32, aid: u32, id: u32) {
        let p = rusqlite::params![id, uid, aid];
        let sql = "DELETE FROM stock_purchases WHERE id = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        if exists {
            stmt.execute(p).unwrap();
        } else {
            panic!("Stock id {} does not exist!", id);
        }
    }

    pub fn remove_stock_sale(
        &self,
        uid: u32,
        aid: u32,
        ledger_id: u32,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let id_sql = "SELECT id FROM stock_sales WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = conn_lock.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }

        let rm_sql = "DELETE FROM stock_sales WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        stmt = conn_lock.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql = "UPDATE stock_sales SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock sale ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET ssid = ssid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'ssid' value in 'user_account_info': {}",
                    error
                );
            }
        }

        return Ok(Some(id));
    }

    pub fn remove_stock_purchase(
        &self,
        uid: u32,
        aid: u32,
        ledger_id: u32,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let id_sql =
            "SELECT id FROM stock_purchases WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = conn_lock.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let rm_sql = "DELETE FROM stock_purchases WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        stmt = conn_lock.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql = "UPDATE stock_purchases SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock purchase ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET spid = spid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'spid' value in 'user_account_info': {}",
                    error
                );
            }
        }

        return Ok(Some(id));
    }

    pub fn remove_stock_split(
        &self,
        uid: u32,
        aid: u32,
        ledger_id: u32,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid, aid];
        let id_sql = "SELECT id FROM stock_splits WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = conn_lock.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let rm_sql = "DELETE FROM stock_splits WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        stmt = conn_lock.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql = "UPDATE stock_splits SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock split ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET splid = splid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'splid' value in 'user_account_info': {}",
                    error
                );
            }
        }

        return Ok(Some(id));
    }

    pub fn remove_stock_sale_allocation(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![id, uid, aid];
        let id_sql =
            "SELECT id FROM stock_sale_allocation WHERE id = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = conn_lock.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let rm_sql =
            "DELETE FROM stock_sale_allocation WHERE id = (?1) and uid = (?2) and aid = (?3)";
        stmt = conn_lock.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql =
            "UPDATE stock_sale_allocation SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock split ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET said = said - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'said' value in 'user_account_info': {}",
                    error
                );
            }
        }

        return Ok(Some(id));
    }

    pub fn remove_stock_split_allocation(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![id, uid, aid];
        let id_sql =
            "SELECT id FROM stock_split_allocations WHERE id = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = conn_lock.prepare(id_sql)?;
                id = stmt.query_row(p, |row| row.get(0))?;
            }
            false => {
                return Ok(None);
            }
        }
        let rm_sql =
            "DELETE FROM stock_split_allocations WHERE id = (?1) and uid = (?2) and aid = (?3)";
        stmt = conn_lock.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql =
            "UPDATE stock_split_allocations SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock sale ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET stock_split_allocation_id = stock_split_allocation_id - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'ssid' value in 'stock_split_allocation_id': {}",
                    error
                );
            }
        }

        return Ok(Some(id));
    }

    pub fn update_stock_purchase(
        &self,
        uid: u32,
        aid: u32,
        updated_info: StockInfo,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![updated_info.ledger_id, uid, aid];
        let id_sql =
            "SELECT id FROM stock_purchases WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = conn_lock.prepare(id_sql)?;
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
        stmt = conn_lock.prepare(update_sql)?;
        stmt.execute(p)?;
        return Ok(Some(id));
    }

    pub fn update_stock_sale(
        &self,
        uid: u32,
        aid: u32,
        updated_info: StockInfo,
    ) -> Result<Option<u32>, rusqlite::Error> {
        let p = rusqlite::params![updated_info.ledger_id, uid, aid];
        let id_sql = "SELECT id FROM stock_sales WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(id_sql).unwrap();
        let exists = stmt.exists(p).unwrap();
        let id: u32;
        match exists {
            true => {
                stmt = conn_lock.prepare(id_sql)?;
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
        stmt = conn_lock.prepare(update_sql)?;
        stmt.execute(p)?;
        return Ok(Some(id));
    }

    pub fn update_stock_remaining(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
        updated_shares: f32,
    ) -> Result<u32> {
        let p = rusqlite::params![id, updated_shares, uid, aid];
        let sql = "UPDATE stock_purchases SET remaining = (?2) WHERE id = (?1) and uid = (?3) and aid = (?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to update shares: {}", error);
            }
        }
    }

    pub fn update_stock_shares_purchased(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
        updated_shares: f32,
    ) -> Result<u32> {
        let p = rusqlite::params![id, updated_shares, uid, aid];
        let sql = "UPDATE stock_purchases SET shares = (?2) WHERE id = (?1) and uid = (?3) and aid = (?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to update shares: {}", error);
            }
        }
    }

    pub fn update_stock_shares_sold(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
        updated_shares: f32,
    ) -> Result<u32> {
        let p = rusqlite::params![id, updated_shares, uid, aid];
        let sql =
            "UPDATE stock_sales SET shares = (?2) WHERE id = (?1) and uid = (?3) and aid = (?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to update shares: {}", error);
            }
        }
    }

    pub fn add_to_stock_remaining(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
        shares_to_add: f32,
    ) -> Result<u32> {
        let p = rusqlite::params![id, shares_to_add, uid, aid];
        let sql = "UPDATE stock_purchases SET remaining = remaining + (?2) WHERE id = (?1) and uid = (?3) and aid = (?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to update shares: {}", error);
            }
        }
    }

    pub fn update_cost_basis(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
        updated_costbasis: f32,
    ) -> Result<u32> {
        let p = rusqlite::params![id, updated_costbasis, uid, aid];
        let sql = "UPDATE stock_purchases SET costbasis = (?2) WHERE id = (?1) and uid = (?3) and aid = (?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to update shares: {}", error);
            }
        }
    }

    pub fn update_stock_sale_allocation_quantity(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
        updated_quantity: f32,
    ) -> Result<u32> {
        let p = rusqlite::params![id, updated_quantity, uid, aid];
        let sql = "UPDATE stock_sale_allocation SET quantity = (?2) WHERE id = (?1) and uid = (?3) and aid = (?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to update shares: {}", error);
            }
        }
    }

    pub fn get_stock_tickers(&self, uid: u32, aid: u32) -> Result<Vec<String>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT pid FROM ledgers INNER JOIN stock_purchases ON stock_purchases.lid = ledgers.id WHERE ledgers.aid = (?1) and ledgers.uid = (?2)";
        let mut pids = Vec::new();
        {
            let conn_lock = self.conn.lock().unwrap();
            let mut stmt = conn_lock.prepare(sql)?;
            let exists = stmt.exists(p)?;
            match exists {
                true => {
                    stmt = conn_lock.prepare(sql)?;
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

        let mut stocks: Vec<String> = Vec::new();
        for pid in pids {
            let ticker = self.get_participant(uid, aid, pid.unwrap()).unwrap();
            stocks.push(ticker);
        }

        Ok(stocks)
    }

    pub fn get_stock_sale_allocation_for_sale_id(
        &self,
        uid: u32,
        aid: u32,
        sale_id: u32,
    ) -> Result<Vec<SaleAllocationRecord>, rusqlite::Error> {
        let p = rusqlite::params![sale_id, uid, aid];
        let sql = "SELECT * FROM stock_sale_allocation WHERE sale_id = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut records: Vec<SaleAllocationRecord> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
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
        &self,
        uid: u32,
        aid: u32,
        split_id: u32,
    ) -> Result<Vec<StockSplitAllocationRecord>, rusqlite::Error> {
        let p = rusqlite::params![split_id, uid, aid];
        let sql = "SELECT id, stock_split_id, stock_purchase_id FROM stock_split_allocations WHERE stock_split_id = (?1) and uid = (?2) and aid = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut records: Vec<StockSplitAllocationRecord> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
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
        &self,
        uid: u32,
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
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
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
                            txn_opt: None,
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
        &self,
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

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
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
                stmt = conn_lock.prepare(sql)?;
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
        match conn_lock.prepare(sql) {
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
        &self,
        uid: u32,
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
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();

        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
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
                            txn_opt: None,
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
        &self,
        uid: u32,
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
                ledgers.pid = people.id
            WHERE 
                ledgers.aid = (?1) and 
                ledgers.uid = (?3) and
                people.name LIKE (?2)
            ORDER BY
                ledgers.date DESC
            ";

        // let quotes = stocks::get_stock_history(ticker.clone(), start, end).unwrap();
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut stocks = Vec::new();

        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
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
                            txn_opt: None,
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

    pub fn get_stock_current_value(
        &self,
        uid: u32,
        aid: u32,
    ) -> rusqlite::Result<f32, rusqlite::Error> {
        let mut sum: f32;
        let p = rusqlite::params![uid, aid];
        let sql = "
            SELECT SUM(get_stock_value(ticker) * shares) as total_value
            FROM (
                SELECT 
                    p.name as ticker,
                    SUM(sp.remaining) as shares 
                FROM 
                    stock_purchases as sp
                INNER JOIN ledgers as l ON 
                    sp.lid = l.id and
                    sp.aid = l.aid and 
                    sp.uid = l.uid 
                INNER JOIN people as p ON 
                    l.pid = p.id and 
                    l.aid = p.aid and 
                    l.uid = p.uid 
                WHERE 
                    sp.uid = ?1 and
                    sp.aid = ?2
                GROUP BY
                    ticker
            )
        ";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        if stmt.exists(p)? {
            sum = stmt.query_row(p, |row| row.get(0))?;
        } else {
            panic!("Not found!");
        }
        Ok(sum)
    }

    pub fn get_portfolio_value_before_date(
        &self,
        uid: u32,
        aid: u32,
        date: NaiveDate,
    ) -> rusqlite::Result<Option<f32>, rusqlite::Error> {
        let mut sum: f32 = 0.0;
        let p = rusqlite::params![aid, uid, date.format("%Y-%m-%d").to_string()];
        let sql =
            "WITH 
                -- Purchases (converting purchases into positive amounts)
                purchases AS (
                    SELECT 
                        p.name as ticker, 
                        l.date AS transaction_date, 
                        sp.shares, 
                        'purchase' AS transaction_type
                    FROM stock_purchases AS sp 
                    INNER JOIN ledgers AS l ON 
                        sp.lid = l.id and 
                        sp.aid = l.aid and
                        sp.uid = l.uid 
                    INNER JOIN people AS p ON
                        l.pid = p.id and 
                        l.aid = p.aid and 
                        l.uid = p.uid 
                    WHERE 
                        sp.aid = (?1) and 
                        sp.uid = (?2)
                ),
                
                -- Sales (converting sales into negative amounts)
                sales AS (
                    SELECT 
                        p.name as ticker, 
                        l.date AS transaction_date, 
                        -ss.shares AS shares, 
                        'sale' AS transaction_type
                    FROM stock_sales AS ss
                    INNER JOIN ledgers AS l ON 
                        ss.lid = l.id and 
                        ss.aid = l.aid and
                        ss.uid = l.uid 
                    INNER JOIN people AS p ON
                        l.pid = p.id and 
                        l.aid = p.aid and 
                        l.uid = p.uid 
                    WHERE 
                        ss.aid = (?1) and 
                        ss.uid = (?2)
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
                    WHERE t.transaction_date <= (?3)
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
                SUM(get_stock_value_on_day(ticker, (?3)) * final_shares_owned)
            FROM residual";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                sum = stmt.query_row(p, |row| row.get(0))?;
                return Ok(Some(sum));
            }
            false => {
                return Ok(None);
            }
        }
    }

    pub fn get_positions(
        &self,
        uid: u32,
        aid: u32,
    ) -> rusqlite::Result<Option<Vec<(String, f32)>>, rusqlite::Error> {
        let p = rusqlite::params![uid, aid];
        let sql = "
            SELECT 
                p.name, SUM(sp.remaining) 
            FROM 
                stock_purchases as sp
            INNER JOIN ledgers as l ON 
                sp.lid = l.id and 
                sp.aid = l.aid and 
                sp.uid = l.uid 
            INNER JOIN people as p ON 
                l.pid = p.id and 
                l.aid = p.aid and 
                l.uid = p.uid
            WHERE 
                sp.uid = (?1) and 
                sp.aid = (?2)
            GROUP BY p.name;
        ";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut positions: Vec<(String, f32)> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let rows = stmt
                    .query_map(p, |row| Ok((row.get(0)?, row.get(1)?)))
                    .unwrap()
                    .collect::<Vec<_>>();

                for row in rows {
                    positions.push(row.unwrap());
                }
                return Ok(Some(positions));
            }
            false => {
                return Ok(None);
            }
        }
    }

    pub fn get_positions_by_ledger(
        &self,
        aid: u32,
        uid: u32,
    ) -> Result<Option<Vec<(String, String, f32)>>, rusqlite::Error> {
        let p = rusqlite::params![uid, aid];
        let sql = "
            WITH stock_ledger AS (
                SELECT *
                FROM ledgers l
                WHERE EXISTS (
                    SELECT 1 
                    FROM stock_purchases sp 
                    WHERE sp.lid = l.id 
                    AND sp.uid = l.uid 
                    AND sp.aid = l.aid
                    AND l.uid = (?1)
                    AND l.aid = (?2)
                )
                OR EXISTS (
                    SELECT 1 
                    FROM stock_sales ss 
                    WHERE ss.lid = l.id 
                    AND ss.uid = l.uid 
                    AND ss.aid = l.aid
                    AND l.uid = (?1)
                    AND l.aid = (?2)
                )
            ),
            ledger_changes AS (
                SELECT
                    sl.date,
                    p.name AS ticker,
                    COALESCE(
                        CASE
                            WHEN sl.transfer_type IN (0, 2) THEN sp.shares
                            WHEN sl.transfer_type IN (1, 3) THEN -ss.shares
                            ELSE 0
                        END,
                        0
                    ) AS share_change
                FROM stock_ledger sl
                JOIN people p 
                ON p.id = sl.pid 
                AND p.uid = sl.uid 
                AND p.aid = sl.aid
                LEFT JOIN stock_purchases sp 
                ON sp.lid = sl.id 
                AND sp.uid = sl.uid 
                AND sp.aid = sl.aid
                LEFT JOIN stock_sales ss 
                ON ss.lid = sl.id 
                AND ss.uid = sl.uid 
                AND ss.aid = sl.aid
            ),
            running_total AS (
                SELECT
                    lc.ticker,
                    lc.date,
                    lc.share_change,
                    SUM(lc.share_change) OVER (
                        PARTITION BY lc.ticker ORDER BY lc.date
                        ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
                    ) AS shares_owned
                FROM ledger_changes lc
            )
            SELECT *
            FROM running_total
            ORDER BY ticker, date;
        ";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut positions: Vec<(String, String, f32)> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let rows = stmt
                    .query_map(p, |row| Ok((row.get(0)?, row.get(1)?, row.get(3)?)))
                    .unwrap()
                    .collect::<Vec<_>>();

                for row in rows {
                    positions.push(row.unwrap());
                }
                return Ok(Some(positions));
            }
            false => {
                return Ok(None);
            }
        }
    }
}
