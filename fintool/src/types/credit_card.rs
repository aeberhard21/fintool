use inquire::autocompletion;
use inquire::autocompletion::*;
use inquire::Autocomplete;
use inquire::CustomUserError;
use rusqlite::Result;

use crate::database::DbConn;

#[derive(Clone)]
pub struct CreditCardRecord {
    pub id: u32,
    pub info: CreditCardInfo,
}

#[derive(Clone)]
pub struct CreditCardInfo {
    pub credit_line : f32,
    // this is the day of each month that
    // an owed amount is due
    pub statement_due_date : u32
}

impl DbConn {
    pub fn create_credit_card_accounts_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS credit_cards ( 
                id          INTEGER NOT NULL,
                aid         INTEGER NOT NULL,
                uid         INTEGER NOT NULL,
                credit_line INTEGER NOT NULL, 
                statement_due_date INTEGER NOT NULL,
                PRIMARY KEY (uid, aid, id),
                FOREIGN KEY(uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE,
                FOREIGN KEY(uid) REFERENCES users(id)
            )";

        self.conn
            .execute(sql, ())
            .expect("Unable to initialize credit_cards table!");
        Ok(())
    }

    pub fn add_credit_card(&mut self, uid : u32, aid: u32, info: CreditCardInfo) -> Result<u32> {
        let id = self.get_next_credit_card_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, aid, uid, info.credit_line, info.statement_due_date);
        let sql = "INSERT INTO credit_cards (id, aid, uid, credit_line, statement_due_date) VALUES (?1, ?2, ?3, ?4, ?5)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add credit card for account {}: {}", aid, error);
            }
        }
    }

    pub fn update_credit_line(&mut self, uid : u32, aid: u32, new_credit_line : f32) -> Result<f32> {
        let p = rusqlite::params!(uid, aid, new_credit_line);
        let sql = "UPDATE credit_cards SET credit_line = (?3) WHERE uid = (?1) and aid = (?2)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(new_credit_line),
            Err(error) => {
                panic!("Unable to update credit line for credit card {}: {}!", aid, error);
            }
        }
    }

    pub fn update_statement_due_date(&mut self, uid : u32, aid: u32, new_statement_due_date : u32) -> Result<u32> {
        let p = rusqlite::params!(uid, aid, new_statement_due_date);
        let sql = "UPDATE credit_cards SET statement_due_date = (?3) WHERE uid = (?1) and aid = (?2)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(new_statement_due_date),
            Err(error) => {
                panic!("Unable to update statement due date for credit card {}: {}!", aid, error);
            }
        }
    }

    pub fn get_credit_card(&mut self, uid : u32, aid : u32) -> Result<CreditCardRecord, rusqlite::Error> { 
        let p = rusqlite::params![uid,  aid];
        let sql = "SELECT id, credit_line, statement_due_date WHERE uid = (?1) and aid = (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists { 
            true => {
                stmt = self.conn.prepare(sql)?;
                let cc_wrap = stmt.query_row(p, |row| {
                        Ok(CreditCardRecord {
                            id : row.get(0)?,
                            info : CreditCardInfo { 
                                credit_line : row.get(1)?, 
                                statement_due_date : row.get(2)?
                            }
                        })
                    }

                );
                match cc_wrap {
                    Ok(cc) => {return Ok(cc)}, 
                    Err(error) => {panic!("Unable to retrieve credit card info for account {}: {}", aid, error)}
                }
            }
            false => { 
                panic!("Unable to find credit card matching account id: {}!", aid);
            }
        }
    }

}

 
