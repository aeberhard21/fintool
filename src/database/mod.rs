use rusqlite::{Connection, Error, Rows};
use std::path::{Path, PathBuf};

use crate::{ledger::Ledger, tui::tui_user::create_user};

mod db_ledger;
mod db_user;
pub mod db_accounts;
pub mod db_banks;
pub mod db_hsa;
mod statements;

const CURRENT_DATABASE_SCHEMA_VERSION: i32 = 0;

pub struct DbConn {
    pub conn: Connection,
}

impl DbConn {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        // the ? returns early if error, otherwise ok
        let rs = Connection::open(db_path);
        let mut conn;
        match rs {
            Ok(rs_conn) => {
                conn = Self { conn: rs_conn };
                conn.initialize_database();
            }
            Err(error) => {
                panic!("unable to open db: {}", error)
            }
        };
        Ok(conn)
    }

    fn initialize_database(&mut self) -> Result<(), rusqlite::Error> {
        // self.conn.execute(statements::CREATE_LEDGER, ())?;
        Self::allow_foreign_keys(&self.conn);
        Self::create_db_info_table(self);
        Self::create_user_table(self);
        Self::create_accounts_table(self);
        Self::create_ledger_table(self);
        Self::create_bank_table(self);
        Self::create_hsa_table(self);
        Self::set_schema_version(&self.conn, CURRENT_DATABASE_SCHEMA_VERSION);
        Ok(())
    }

    fn allow_foreign_keys(conn: &Connection) -> rusqlite::Result<()> {
        conn.pragma_update(None, "foreign_keys", "on")
    }

    fn get_schema_version(conn: &Connection) -> rusqlite::Result<i32> {
        conn.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))
    }

    fn set_schema_version(conn: &Connection, schema_version: i32) -> rusqlite::Result<()> {
        conn.pragma_update(None, "user_version", schema_version)
    }

    fn create_db_info_table(&mut self) -> rusqlite::Result<()> {
        let sql = "CREATE TABLE IF NOT EXISTS info (
            uid    INTEGER NOT NULL,
            aid    INTEGER NOT NULL
        )";
        self.conn.execute(sql, ())?;
        let sql = "SELECT uid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        if !exists {
            let sql: &str = "INSERT INTO info (uid, aid) VALUES ( ?1,  ?2)";
            match self.conn.execute(sql,(0, 0)) {
                Ok(rows_inserted) => {
                    println!("Initialized info table: {}!", rows_inserted);
                }
                Err(error) => {
                    panic!("Unable to initialize info table: {}", error);
                }
            }
        }
        println!("Created info table!");
        Ok(())
    }

    pub fn get_next_user_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT uid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?; 
                let sql = "UPDATE info SET uid = uid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next user ID within table 'info' does not exist.");
            }
        }
    }
    pub fn get_next_account_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT aid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?; 
                let sql = "UPDATE info SET aid = aid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next account ID within table 'info' does not exist.");
            }
        }
    }

    pub fn close(self) {
        self.conn.close().unwrap();
    }
}
