use chrono::NaiveDate;
use rusqlite::{Error, Result};

use crate::database::DbConn;

#[derive(Debug, Clone)]
pub struct StockPriceInfo { 
    pub stock_ticker_peer_id : u32, 
    pub price_per_unit_share : f32,
    pub date : String,
}

#[derive(Debug, Clone)]
pub struct StockPriceRecord { 
    pub id : u32, 
    pub info : StockPriceInfo,
}

impl DbConn { 
    pub fn create_stock_prices_table(&self) -> Result<()> { 
        let sql = "CREATE TABLE IF NOT EXISTS stock_prices (
            id              INTEGER NOT NULL, 
            stock_ticker_peer_id INTEGER NOT NULL, 
            price           REAL NOT NULL, 
            date            STRING NOT NULL, 
            aid             INTEGER NOT NULL, 
            uid             INTEGER NOT NULL,
            PRIMARY KEY (uid, aid, id), 
            FOREIGN KEY (uid, aid) REFERENCES accounts(uid, id) ON DELETE CASCADE ON UPDATE CASCADE, 
            FOREIGN KEY (uid, aid, stock_ticker_peer_id) REFERENCES people(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE, 
            FOREIGN KEY (uid) REFERENCES users(id)
        )";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, ()) {
            Ok(_) => {}
            Err(error) => {
                panic!(
                    "Unable to create table 'stock_prices' because: {}",
                    error
                );
            }
        }
        Ok(())
    }

    pub fn add_stock_price(&self, uid : u32, aid : u32, info : StockPriceInfo) -> Result<u32> { 
        let id = self.get_next_stock_price_id(uid, aid).unwrap();
        let p = rusqlite::params!(
            id,
            info.stock_ticker_peer_id, 
            info.price_per_unit_share, 
            info.date,
            uid,
            aid, 
        );
        let sql = "INSERT INTO stock_prices (id, stock_ticker_peer_id, price, date, uid, aid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add stock price record for account {}: {}",
                    aid, error
                );
            }
        }
    }

    pub fn check_and_get_stock_price_record_matching_from_participant_id(
        &self,
        uid: u32,
        aid: u32,
        participant_id: u32,
    ) -> rusqlite::Result<Vec<StockPriceRecord>, rusqlite::Error> {
        let p = rusqlite::params![participant_id, uid, aid];
        let sql = "
            SELECT id, stock_ticker_peer_id, price, date 
            FROM stock_prices 
            WHERE 
                stock_ticker_peer_id = (?1) and 
                uid = (?2) and 
                aid = (?3)
        ";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut entries : Vec<StockPriceRecord> = Vec::new();
        match exists {
            true => {
                let found_entries = stmt.query_map(p, |row| {
                    Ok(StockPriceRecord {
                        id: row.get(0)?,
                        info: StockPriceInfo { 
                            stock_ticker_peer_id: row.get(1)?, 
                            price_per_unit_share: row.get(2)?,
                            date: row.get(3)? 
                        },
                    })
                })
                .unwrap()
                .collect::<Vec<_>>();

                for entry in found_entries {
                    entries.push(entry.unwrap());
                }
                Ok(entries)
            }
            false => Ok(entries),
        }
    }

    pub fn remove_stock_price(&self, uid : u32, aid : u32, stock_price_id : u32) -> Result<Option<u32>, rusqlite::Error> { 
        let p = rusqlite::params![stock_price_id, uid, aid];
        let id_sql = "SELECT id FROM stock_prices WHERE lid = (?1) and uid = (?2) and aid = (?3)";
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

        let rm_sql = "DELETE FROM stock_prices WHERE lid = (?1) and uid = (?2) and aid = (?3)";
        stmt = conn_lock.prepare(rm_sql).unwrap();
        stmt.execute(p)?;

        let sql = "UPDATE stock_prices SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to stock sale ids: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET stock_price_id = stock_price_id - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'stock_price_id' value in 'user_account_info': {}",
                    error
                );
            }
        }

        return Ok(Some(id));
    }

    pub fn update_stock_price_record( &self, uid : u32, aid : u32, price_record : StockPriceRecord ) -> u32 { 
        let p = rusqlite::params![uid, aid, price_record.id, price_record.info.date, price_record.info.price_per_unit_share];
        let sql = "UPDATE stock_prices SET date = (?4) and price = (?5) WHERE uid = (?1) and aid = (?2) and id = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs { 
            Ok(_usize) => {price_record.id}, 
            Err(error) => { 
                panic!("Unable to update stock price record: {}!",error);
            }
        }
    }

    pub fn apply_stock_split_to_stock_prices( &self, uid : u32, aid : u32, peer_id : u32, split : f32 ) -> u32 { 
        let p = rusqlite::params![uid, aid, peer_id, split];
        let sql = "UPDATE stock_prices SET price = price / (?4) WHERE uid = (?1) and aid = (?2) and stock_ticker_peer_id = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs { 
            Ok(_usize) => {peer_id}, 
            Err(error) => { 
                panic!("Unable to update stock price record: {}!",error);
            }
        }
    }

}