use rusqlite::{Result, Error};
use super::DbConn;

pub struct HsaRecord { 
    pub fixed: f32, 
    pub variable: f32, 
    pub date: i64
}

impl DbConn {
    pub fn create_hsa_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS hsa ( 
            date        INTEGER NOT NULL, 
            fixed       REAL NOT NULL,
            variable    REAL NOT NULL,
            aid         INTEGER,
            FOREIGN     KEY (aid) REFERENCES accounts(id)
        )";
        match self.conn.execute(sql, ()) {
            Ok(_) => {
                println!("Created hsa table!");
            }
            Err(error) => {
                panic!("Unable to create table 'hsa:' {}", error);
            }
        }
        Ok(())
    }

    pub fn record_hsa_account(&mut self, aid: u32, record: HsaRecord) -> Result<(), Error> {
        let sql: &str = "INSERT INTO hsa (date, fixed, variable, aid) VALUES (?1, ?2, ?3, ?4)";
        match self.conn.execute(sql, rusqlite::params!(record.date, record.fixed, record.variable, aid)) {
            Ok(_) => {
                println!("Added HSA record!");
            }
            Err(error) => {
                panic!("Unable to add HSA record: {}", error);
            }
        }
        Ok(())
    }  
}
