use std::usize;
use std::sync::atomic::{AtomicU32, Ordering};
use rusqlite::{Connection, Result, Error};

use super::DbConn;

pub enum AccountType {
    Ledger, 
    Investment,
}

impl DbConn {
    pub fn create_accounts_table(&mut self) -> Result<()> {
        let sql: &str = 
            "CREATE TABLE IF NOT EXISTS accounts (
                aid  INTEGER NOT NULL PRIMARY KEY, 
                type INTEGER NOT NULL, 
                name TEXT NOT NULL,
                uid  INTEGER,
                FOREIGN KEY (uid) REFERENCES users(id)
            )";
        let rs = self.conn.execute(sql, ());
        match rs {
            Ok(_) => {
                println!("Created accounts table!")
            }
            Err(error) => {
                panic!("Unable to create: {}", error)
            }
        }
        Ok(())
    }

    pub fn add_account(&mut self, uid: u32, name: String, atype: AccountType) -> Result<()> {
        static AID: AtomicU32 = AtomicU32::new(0); 
        let sql: &str = "INSERT INTO accounts (aid, type, name, uid) VALUES (?1, ?2, ?3, ?4)";
        let rs = self.conn.execute(sql, 
            (AID.fetch_add(1, Ordering::Relaxed), 
            atype as u64, 
            &name, 
            uid));
        match rs {
            Ok(usize) => {
                Ok(())
            }
            Err(error) => {
                panic!("Unable to add account {} for user {}: {}!", &name, &uid, error);
            }
        }
    }

    pub fn get_user_accounts(&mut self, uid: u32) -> rusqlite::Result<Vec<String>, Error> {
        let sql: &str = "SELECT name FROM accounts WHERE uid = (?1)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(rusqlite::params![uid])?;
        let mut accounts: Vec<String> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let names: Vec<Result<String, Error>> = 
                    stmt.query_map(
                        rusqlite::params![uid], 
                        |row| { 
                            Ok(row.get(0)?)
                        }).unwrap().collect::<Vec<_>>();
                for name in names {
                    accounts.push(name.unwrap())
                }
                return Ok(accounts);
            }
            false => {
                return Ok(accounts);
            }
        }
    }

    pub fn get_account_id(&mut self, aname: String) -> rusqlite::Result<u32, Error> {
        let sql: &str = "SELECT aid from accounts WHERE name = (?1)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists((&aname,),)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let id = stmt.query_row((&aname,), |row| row.get::<_,u32>(0));
                match id {
                    Ok(id) => {
                        return Ok(id);
                    }
                    Err(err) => {
                        panic!("Unable to retrieve id for account {}: {}", &aname, err);
                    }
                }
            }
            false => {
                panic!("Unable to find account matching {}", aname);
            }
        }
    }  
}