use std::usize;

// use crate::database::statements::CREATE_LEDGER;
use crate::ledger::LedgerEntry;
use crate::ledger::Ledger;
use super::DbConn;
use rusqlite::ffi::sqlite3;
use rusqlite::{Connection, Result};


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
            Err(Error) => {panic!("unable to create: {}", Error)}
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

    // fn get_ledger_entry(self, )
}