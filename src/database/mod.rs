use rusqlite::{Connection, Error};
use std::path::{Path, PathBuf};

mod statements;

const CURRENT_DATABASE_SCHEMA_VERSION: i32 = 0;

pub struct DbConn { 
    conn: Connection
}

impl DbConn {
    pub fn new(db_path: impl AsRef<Path>) -> Result<(), rusqlite::Error> {
        // the ? returns early if error, otherwise ok
        let conn = Connection::open(db_path)?;

        // need to get schema version for the database

        let mut conn = Self { conn } ;
        conn.initialize_database();
        Ok(())
    } 

    fn initialize_database(&self) {
        self.conn.execute(statements::CREATE_LEDGER, ());

        Self::set_schema_version(&self.conn, CURRENT_DATABASE_SCHEMA_VERSION);
    }

    fn get_schema_version(conn: &Connection) -> rusqlite::Result<i32> {
        conn.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))
    }
    
    fn set_schema_version(conn: &Connection, schema_version: i32) -> rusqlite::Result<()> {
        conn.pragma_update(None, "user_version", schema_version)
    }
}