use rusqlite::Result;

use crate::database::DbConn;

#[derive(Clone)]
pub struct HsaRecord {
    pub id: u32,
    pub info: HsaInfo,
}

#[derive(Clone)]
pub struct HsaInfo {
    pub contribution_limit: f32,
}

impl DbConn {
    pub fn create_hsa_accounts_table(&self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS hsas ( 
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
            .expect("Unable to initialize HSAs table!");
        Ok(())
    }

    pub fn add_hsa_account(&self, uid: u32, aid: u32, info: HsaInfo) -> Result<u32> {
        let id = self.get_next_hsa_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, aid, uid, info.contribution_limit);
        let sql = "INSERT INTO hsas (id, aid, uid, contribution_limit) VALUES (?1, ?2, ?3, ?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add HSA {}: {}", aid, error);
            }
        }
    }

    pub fn update_hsa_contribution_limit(
        &self,
        uid: u32,
        aid: u32,
        new_contribution_lmit: f32,
    ) -> Result<f32> {
        let p = rusqlite::params!(uid, aid, new_contribution_lmit);
        let sql = "UPDATE hsas SET contribution_limit = (?3) WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(new_contribution_lmit),
            Err(error) => {
                panic!("Unable to update hsa for hsa {}: {}!", aid, error);
            }
        }
    }

    pub fn get_hsa(&self, uid: u32, aid: u32) -> Result<HsaRecord, rusqlite::Error> {
        let p = rusqlite::params![uid, aid];
        let sql = "SELECT id, contribution_limit FROM hsas WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let cc_wrap = stmt.query_row(p, |row| {
                    Ok(HsaRecord {
                        id: row.get(0)?,
                        info: HsaInfo {
                            contribution_limit: row.get(1)?,
                        },
                    })
                });
                match cc_wrap {
                    Ok(cc) => return Ok(cc),
                    Err(error) => {
                        panic!("Unable to retrieve hsa info for account {}: {}", aid, error)
                    }
                }
            }
            false => {
                panic!("Unable to find hsa matching account id: {}!", aid);
            }
        }
    }
}
