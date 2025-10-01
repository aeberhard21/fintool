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
use rusqlite::Result;

use crate::database::DbConn;

#[derive(Clone)]
pub struct RothIraRecord {
    pub id: u32,
    pub info: RothIraInfo,
}

#[derive(Clone)]
pub struct RothIraInfo {
    pub contribution_limit: f32,
}

impl DbConn {
    pub fn create_roth_ira_accounts_table(&self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS roth_iras ( 
                id          INTEGER NOT NULL,
                contribution_limit REAL NOT NULL,
                uid  INTEGER NOT NULL, 
                aid INTEGER NOT NULL,
                PRIMARY KEY (uid, aid, id),
                FOREIGN KEY(uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE,
                FOREIGN KEY(uid) REFERENCES users(id)
            )";

        let conn_lock = self.conn.lock().unwrap();
        conn_lock
            .execute(sql, ())
            .expect("Unable to initialize credit_cards table!");
        Ok(())
    }

    pub fn add_roth_ira_account(&self, uid: u32, aid: u32, info: RothIraInfo) -> Result<u32> {
        let id = self.get_next_roth_ira_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, aid, uid, info.contribution_limit);
        let sql =
            "INSERT INTO roth_iras (id, aid, uid, contribution_limit) VALUES (?1, ?2, ?3, ?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add roth ira account {}: {}", aid, error);
            }
        }
    }

    pub fn update_roth_ira_contribution_limit(
        &self,
        uid: u32,
        aid: u32,
        new_contribution_lmit: f32,
    ) -> Result<f32> {
        let p = rusqlite::params!(uid, aid, new_contribution_lmit);
        let sql = "UPDATE roth_iras SET contribution_limit = (?3) WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(new_contribution_lmit),
            Err(error) => {
                panic!(
                    "Unable to update contribution limit for roth ira {}: {}!",
                    aid, error
                );
            }
        }
    }

    pub fn get_roth_ira(&self, uid: u32, aid: u32) -> Result<RothIraRecord, rusqlite::Error> {
        let p = rusqlite::params![uid, aid];
        let sql = "SELECT id, contribution_limit FROM roth_iras WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let cc_wrap = stmt.query_row(p, |row| {
                    Ok(RothIraRecord {
                        id: row.get(0)?,
                        info: RothIraInfo {
                            contribution_limit: row.get(1)?,
                        },
                    })
                });
                match cc_wrap {
                    Ok(cc) => return Ok(cc),
                    Err(error) => {
                        panic!(
                            "Unable to retrieve roth ira info for account {}: {}",
                            aid, error
                        )
                    }
                }
            }
            false => {
                panic!("Unable to find roth ira matching account id: {}!", aid);
            }
        }
    }
}
