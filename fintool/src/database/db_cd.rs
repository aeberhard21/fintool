/* ------------------------------------------------------------------------
    Copyright (C) 2025  Andrew J. Eberhard

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
  -----------------------------------------------------------------------*/
use super::DbConn;
use rusqlite;

pub struct CdRecord {
    pub principal: f32,
    pub apy: f32,
    pub open_date: String,
    pub length: u32,
}

impl DbConn {
    pub fn create_cd_table(&self) -> rusqlite::Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS cds (
            principal   REAL NOT NULL, 
            apy         REAL NOT NULL, 
            open_date   STRINT NOT NULL, 
            length      INTEGER NOT NULL,
            aid         INTEGER NOT NULL, 
            uid         INTEGER NOT NULL,
            PRIMARY KEY (uid, aid)
            FOREIGN     KEY (uid, aid) REFERENCES accounts(uid, id) ON DELETE CASCADE ON UPDATE CASCADE
        )";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, ()) {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create table 'CDs' because {}", error);
            }
        }
        Ok(())
    }

    pub fn add_cd(&self, aid: u32, record: CdRecord) -> Result<(), rusqlite::Error> {
        let p = rusqlite::params!(
            record.principal,
            record.apy,
            record.open_date,
            record.length,
            aid
        );
        let sql =
            "INSERT INTO cds (principal, apy, open_date, length, aid) VALUES (?1, ?2, ?3, ?4, ?5)";
        let conn_lock = self.conn.lock().unwrap();
        conn_lock.execute(sql, p)?;
        return Ok(());
    }

    pub fn get_cds(&self, aid: u32) -> Result<CdRecord, rusqlite::Error> {
        let p = rusqlite::params![aid];
        let sql = "SELECT * FROM cds WHERE aid = (?1)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let account = stmt.query_row(p, |row| {
                    Ok(CdRecord {
                        principal: row.get(0)?,
                        apy: row.get(1)?,
                        open_date: row.get(2)?,
                        length: row.get(3)?,
                    })
                });
                return account;
            }
            false => {
                panic!(
                    "Unable to retrieve certificate of deposit accounts for {}!",
                    aid
                );
            }
        }
    }
}
