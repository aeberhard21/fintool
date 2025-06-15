use inquire::autocompletion;
use inquire::autocompletion::*;
use inquire::Autocomplete;
use inquire::CustomUserError;
use rusqlite::Result;

use crate::database::DbConn;

#[derive(Clone)]
pub struct CertificateOfDepositRecord {
    pub id: u32,
    pub info: CertificateOfDepositInfo,
}

#[derive(Clone)]
pub struct CertificateOfDepositInfo {
    pub apy : f32,
    pub principal : f32,
    pub maturity_date : String,
    pub length_months : u32
}

impl DbConn {
    pub fn create_certificate_of_deposits_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS certificate_of_deposits ( 
                id          INTEGER NOT NULL,
                aid         INTEGER NOT NULL,
                uid         INTEGER NOT NULL,
                apy         REAL NOT NULL, 
                maturity_date STRING NOT NULL,
                principal   REAL NOT NULL, 
                length_months INTEGER NOT NULL,
                PRIMARY KEY (uid, aid, id),
                FOREIGN KEY(uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE,
                FOREIGN KEY(uid) REFERENCES users(id)
            )";

        self.conn
            .execute(sql, ())
            .expect("Unable to initialize certificate_of_deposits table!");
        Ok(())
    }

    pub fn add_certificate_of_deposit(&mut self, uid : u32, aid: u32, info: CertificateOfDepositInfo) -> Result<u32> {
        let id = self.get_next_credit_card_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, aid, uid, info.apy, info.maturity_date, info.principal, info.length_months);
        let sql = "INSERT INTO certificate_of_deposits (id, aid, uid, apy, maturity_date, principal, length_months) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add certificate_of_deposit for account {}: {}", aid, error);
            }
        }
    }

    pub fn update_cd_apy(&mut self, uid : u32, aid: u32, new_apy : f32) -> Result<f32> {
        let p = rusqlite::params!(uid, aid, new_apy);
        let sql = "UPDATE certificate_of_deposits SET apy = (?3) WHERE uid = (?1) and aid = (?2)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(new_apy),
            Err(error) => {
                panic!("Unable to update apy for certificate of deposit {}: {}!", aid, error);
            }
        }
    }

    pub fn update_cd_maturity_date(&mut self, uid : u32, aid: u32, new_maturity_date : String) -> Result<String> {
        let p = rusqlite::params!(uid, aid, new_maturity_date);
        let sql = "UPDATE certificate_of_deposits SET maturity_date = (?3) WHERE uid = (?1) and aid = (?2)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(new_maturity_date),
            Err(error) => {
                panic!("Unable to update maturity date for certificate of deposit {}: {}!", aid, error);
            }
        }
    }

    pub fn update_cd_principal(&mut self, uid : u32, aid: u32, new_principal : f32) -> Result<f32> {
        let p = rusqlite::params!(uid, aid, new_principal);
        let sql = "UPDATE certificate_of_deposits SET principal = (?3) WHERE uid = (?1) and aid = (?2)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(new_principal),
            Err(error) => {
                panic!("Unable to update principal for certificate of deposit {}: {}!", aid, error);
            }
        }
    }

    pub fn update_cd_length(&mut self, uid : u32, aid: u32, new_length : u32) -> Result<u32> {
        let p = rusqlite::params!(uid, aid, new_length);
        let sql = "UPDATE certificate_of_deposits SET length_months = (?3) WHERE uid = (?1) and aid = (?2)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(new_length),
            Err(error) => {
                panic!("Unable to update length for certificate of deposit {}: {}!", aid, error);
            }
        }
    }

    pub fn get_certificate_of_deposit(&mut self, uid : u32, aid : u32) -> Result<CertificateOfDepositRecord, rusqlite::Error> { 
        let p = rusqlite::params![uid,  aid];
        let sql = "SELECT id, apy, maturity_date, principal, length_months FROM certificate_of_deposits WHERE uid = (?1) and aid = (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists { 
            true => {
                stmt = self.conn.prepare(sql)?;
                let cd_wrap = stmt.query_row(p, |row| {
                        Ok(CertificateOfDepositRecord {
                            id : row.get(0)?,
                            info : CertificateOfDepositInfo { 
                                apy : row.get(1)?, 
                                maturity_date : row.get(2)?,
                                principal : row.get(3)?, 
                                length_months : row.get(4)?
                            }
                        })
                    }

                );
                match cd_wrap {
                    Ok(cd) => {return Ok(cd)}, 
                    Err(error) => {panic!("Unable to retrieve certificate of deposit info for account {}: {}", aid, error)}
                }
            }
            false => { 
                panic!("Unable to find certificate of deposit matching account id: {}!", aid);
            }
        }
    }

}

 
