use rusqlite::Result;

use crate::database::DbConn;

#[derive(Clone)]
pub struct Retirement401kRecord {
    pub id: u32,
    pub info: Retirement401kInfo,
}

#[derive(Clone)]
pub struct Retirement401kInfo {
    pub contribution_limit : f32,
}

impl DbConn {
    pub fn create_401k_accounts_table(&self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS plan_401ks ( 
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
            .expect("Unable to initialize 401K accounts table!");
        Ok(())
    }

    pub fn add_401k_account(&self, uid: u32, aid: u32, info: Retirement401kInfo) -> Result<u32> {
        let id = self.get_next_401k_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, aid, uid, info.contribution_limit);
        let sql = "INSERT INTO plan_401ks (id, aid, uid, contribution_limit) VALUES (?1, ?2, ?3, ?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add 401k account {}: {}", aid, error);
            }
        }
    }

    pub fn update_401k_contribution_limit(&self, uid: u32, aid: u32, new_contribution_lmit: f32) -> Result<f32> {
        let p = rusqlite::params!(uid, aid, new_contribution_lmit);
        let sql = "UPDATE plan_401ks SET contribution_limit = (?3) WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(new_contribution_lmit),
            Err(error) => {
                panic!(
                    "Unable to update hsa for 401k {}: {}!",
                    aid, error
                );
            }
        }
    }

    pub fn get_401k(&self, uid: u32, aid: u32) -> Result<Retirement401kRecord, rusqlite::Error> {
        let p = rusqlite::params![uid, aid];
        let sql = "SELECT id, contribution_limit FROM plan_401ks WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let cc_wrap = stmt.query_row(p, |row| {
                    Ok(Retirement401kRecord {
                        id: row.get(0)?,
                        info: Retirement401kInfo {
                            contribution_limit: row.get(1)?,
                        },
                    })
                });
                match cc_wrap {
                    Ok(cc) => return Ok(cc),
                    Err(error) => {
                        panic!(
                            "Unable to retrieve 401k info for account {}: {}",
                            aid, error
                        )
                    }
                }
            }
            false => {
                panic!("Unable to find 401k matching account id: {}!", aid);
            }
        }
    }

}
