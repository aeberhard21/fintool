use rusqlite::{Connection, Error};
use std::path::{Path, PathBuf};

use crate::ledger::Ledger;

mod statements;
mod db_ledger;

const CURRENT_DATABASE_SCHEMA_VERSION: i32 = 0;

pub struct DbConn { 
    pub conn: Connection
}

impl DbConn {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        // the ? returns early if error, otherwise ok
        let rs = Connection::open(db_path);
        let conn : Connection = match rs {
            Ok(conn) => conn,
            Err(_) => { panic!("unable to open db")}
        };
        Ok(Self { conn: conn })
    } 

    fn initialize_database(&self) -> Result<(), rusqlite::Error>{
        // self.conn.execute(statements::CREATE_LEDGER, ())?;

        Self::set_schema_version(&self.conn, CURRENT_DATABASE_SCHEMA_VERSION);
        Ok(())
    }

    fn save_ledger(_ledger: &mut Ledger) {
        
    }

    fn get_schema_version(conn: &Connection) -> rusqlite::Result<i32> {
        conn.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))
    }
    
    fn set_schema_version(conn: &Connection, schema_version: i32) -> rusqlite::Result<()> {
        conn.pragma_update(None, "user_version", schema_version)
    }

    pub fn close(self) {
        self.conn.close().unwrap();
    }
}