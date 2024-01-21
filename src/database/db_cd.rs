use rusqlite;
use super::DbConn;

pub struct CdRecord { 
    pub principal: f32,
    pub apy: f32, 
    pub open_date: i64, 
    pub length: u32,
}

impl DbConn { 
    pub fn create_cd_table(&mut self) -> rusqlite::Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS cds (
            principal   REAL NOT NULL, 
            apy         REAL NOT NULL, 
            open_date   INTEGER NOT NULL, 
            length      INTEGER NOT NULL,
            aid         INTEGER NOT NULL, 
            FOREIGN     KEY (aid) REFERENCES accounts(id)
        )";
        match self.conn.execute(sql, ()) {
            Ok(_) => {
                println!("Created CDs table!");
            }
            Err(error) => {
                panic!("Unable to create table 'CDs' because {}", error);            
            }
        }
        Ok(())
    }

    pub fn add_cd(&mut self, aid: u32, record : CdRecord) -> Result<(), rusqlite::Error> {
        let p = rusqlite::params!(record.principal, record.apy, record.open_date, record.length, aid );
        let sql = "INSERT INTO cds (principal, apy, open_date, length, aid) VALUES (?1, ?2, ?3, ?4, ?5)";
        self.conn.execute(sql, p)?;
        return Ok(());
    }

    pub fn get_cds(&mut self, aid : u32) -> Result<CdRecord, rusqlite::Error> {
        let p = rusqlite::params![aid];
        let sql = "SELECT * FROM cds WHERE aid = (?1)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let account = stmt.query_row(p, |row| {
                    Ok( CdRecord {
                            principal : row.get(0)?, 
                            apy : row.get(1)?, 
                            open_date : row.get(2)?, 
                            length : row.get(3)?
                            }
                        )
                    }
                );
                return account;
            }
            false => {
                panic!("Unable to retrieve certificate of deposit accounts for {}!", aid);
            }
        }
    }
}