use std::usize;

// use crate::database::statements::CREATE_LEDGER;
use crate::ledger::LedgerEntry;
use crate::ledger::Ledger;
use rusqlite::Rows;
use rusqlite::Statement;
use rusqlite::ffi::sqlite3;
use rusqlite::{Connection, Result};

use super::DbConn;

impl DbConn {
    pub fn create_ledger(&mut self, name: String) -> Result<()>{
        let sql : String;
        sql = format!(
            "CREATE TABLE {} (
                date    TEXT NOT NULL, 
                amount  REAL NOT NULL, 
                deposit INTEGER NOT NULL, 
                payee   TEXT NOT NULL, 
                desc    TEXT
            )", 
            name
        );
        println!("{}", sql);
        let rs = self.conn.execute(sql.as_str(), ());
        match rs {
            Ok(_) => { println!("created") }
            Err(error) => {panic!("unable to create: {}", error)}
        }
        Ok(())
    }

    pub fn add_ledger_entry(&mut self, ledger: String, entry: LedgerEntry) -> Result<()> {
        let sql : String;
        sql = format!(
            "INSERT INTO {} 
                ( date, amount, deposit, payee, desc )
                VALUES ( ?1, ?2, ?3, ?4, ?5)
            ", 
            ledger
        );
        let rs = self.conn.execute(sql.as_str(), (entry.date.to_string(), entry.amount, entry.deposit, entry.payee, entry.description));   
        match rs {
            Ok(usize) => {
                println!("Added statement");
            }
            Err(Error) => {
                println!("Unable to add ledger: {}", Error);
            }
        }     
        Ok(())
    }

    pub fn read_ledger(&mut self, name: String) -> Result<Ledger> {
        let mut ledger : Ledger = Ledger::new(name.as_str());
        let sql : String;
        sql = format!(
            "SELECT * FROM {}",
            name
        );
        let mut rs = self.conn.prepare(sql.as_str()).unwrap();
        let mut rows;
        rows = rs.query_map([], |row| Ok(
            LedgerEntry {
                date: row.get(0)?,
                amount: row.get(1)?, 
                deposit: row.get(2)?, 
                payee: row.get(3)?, 
                description: row.get(4)?,
            }
        )
        ).unwrap();
        for row in rows {
            ledger.add(row.unwrap());
        }
        ledger.print();
        return Ok(ledger)
    }

    // fn get_ledger_entry(self, )
}