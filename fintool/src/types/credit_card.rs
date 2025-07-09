use chrono::NaiveDate;
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
    pub credit_line: f32,
    // this is the day of each month that
    // an owed amount is due
    pub statement_due_date: u32,
}

#[derive(Debug, Clone)]
pub struct CreditCardExpense {
    pub category: String,
    pub amount: f32,
}

impl DbConn {
    pub fn create_credit_card_accounts_table(&self) -> Result<()> {
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

        let conn_lock = self.conn.lock().unwrap();
        conn_lock
            .execute(sql, ())
            .expect("Unable to initialize credit_cards table!");
        Ok(())
    }

    pub fn add_credit_card(&self, uid: u32, aid: u32, info: CreditCardInfo) -> Result<u32> {
        let id = self.get_next_credit_card_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, aid, uid, info.credit_line, info.statement_due_date);
        let sql = "INSERT INTO credit_cards (id, aid, uid, credit_line, statement_due_date) VALUES (?1, ?2, ?3, ?4, ?5)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add credit card for account {}: {}", aid, error);
            }
        }
    }

    pub fn update_credit_line(&self, uid: u32, aid: u32, new_credit_line: f32) -> Result<f32> {
        let p = rusqlite::params!(uid, aid, new_credit_line);
        let sql = "UPDATE credit_cards SET credit_line = (?3) FROM credit_cards WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(new_credit_line),
            Err(error) => {
                panic!(
                    "Unable to update credit line for credit card {}: {}!",
                    aid, error
                );
            }
        }
    }

    pub fn update_statement_due_date(
        &self,
        uid: u32,
        aid: u32,
        new_statement_due_date: u32,
    ) -> Result<u32> {
        let p = rusqlite::params!(uid, aid, new_statement_due_date);
        let sql = "UPDATE credit_cards SET statement_due_date = (?3) FROM credit_cards WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(new_statement_due_date),
            Err(error) => {
                panic!(
                    "Unable to update statement due date for credit card {}: {}!",
                    aid, error
                );
            }
        }
    }

    pub fn get_credit_card(&self, uid: u32, aid: u32) -> Result<CreditCardRecord, rusqlite::Error> {
        let p = rusqlite::params![uid, aid];
        let sql = "SELECT id, credit_line, statement_due_date FROM credit_cards WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let cc_wrap = stmt.query_row(p, |row| {
                    Ok(CreditCardRecord {
                        id: row.get(0)?,
                        info: CreditCardInfo {
                            credit_line: row.get(1)?,
                            statement_due_date: row.get(2)?,
                        },
                    })
                });
                match cc_wrap {
                    Ok(cc) => return Ok(cc),
                    Err(error) => {
                        panic!(
                            "Unable to retrieve credit card info for account {}: {}",
                            aid, error
                        )
                    }
                }
            }
            false => {
                panic!("Unable to find credit card matching account id: {}!", aid);
            }
        }
    }

    pub fn get_credit_expenditures_between_dates(
        &self,
        uid: u32,
        aid: u32,
        start_date : NaiveDate, 
        end_date : NaiveDate
    ) -> Result<Option<Vec<CreditCardExpense>>, rusqlite::Error> {
        let p = rusqlite::params![uid, aid, start_date.to_string(), end_date.to_string()];
        let sql = "
            SELECT 
                c.category, SUM(l.amount)
            FROM ledgers AS l 
            INNER JOIN categories AS C ON 
                l.cid = c.id and
                l.aid = c.aid and
                l.uid = c.uid
            WHERE 
                (l.transfer_type = 0 OR l.transfer_type = 2) AND
                l.date >= (?3) and l.date <= (?4) AND
                l.uid = (?1) AND
                l.aid = (?2)
            GROUP BY
                c.category;
        ";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut cumulative_expenses: Vec<CreditCardExpense> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let rows = stmt
                    .query_map(p, |row| {
                        Ok(CreditCardExpense {
                            category: row.get(0)?,
                            amount: row.get(1)?,
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();

                for row in rows {
                    cumulative_expenses.push(row.unwrap());
                }
                return Ok(Some(cumulative_expenses));
            }
            false => {
                return Ok(None);
            }
        }
    }

        pub fn get_credit_expenditures(
        &self,
        uid: u32,
        aid: u32,
    ) -> Result<Option<Vec<CreditCardExpense>>, rusqlite::Error> {
        let p = rusqlite::params![uid, aid];
        let sql = "
            SELECT 
                c.category, SUM(l.amount)
            FROM ledgers AS l 
            INNER JOIN categories AS C ON 
                l.cid = c.id and
                l.aid = c.aid and
                l.uid = c.uid
            WHERE 
                (l.transfer_type = 0 OR l.transfer_type = 2) AND
                l.uid = (?1) AND
                l.aid = (?2)
            GROUP BY
                c.category;
        ";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut cumulative_expenses: Vec<CreditCardExpense> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let rows = stmt
                    .query_map(p, |row| {
                        Ok(CreditCardExpense {
                            category: row.get(0)?,
                            amount: row.get(1)?,
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();

                for row in rows {
                    cumulative_expenses.push(row.unwrap());
                }
                return Ok(Some(cumulative_expenses));
            }
            false => {
                return Ok(None);
            }
        }
    }
}
