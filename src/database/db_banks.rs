use super::DbConn;
use chrono::NaiveDate;
use rusqlite::Result;

pub struct BankRecord {
    pub amount: f32,
    pub date: String,
}

impl DbConn {
    pub fn create_bank_table(&mut self) -> Result<()> {
        let sql: &str = " CREATE TABLE IF NOT EXISTS banks ( 
                    date    TEXT NOT NULL, 
                    amount  REAL NOT NULL,
                    aid     INTEGER, 
                    FOREIGN KEY (aid) REFERENCES accounts(id)
            )";
        let rs = self.conn.execute(sql, ());
        match rs {
            Ok(_) => {
            }
            Err(error) => {
                panic!("Unable to create banks table: {}", error);
            }
        }
        Ok(())
    }

    pub fn record_bank_account(
        &mut self,
        aid: u32,
        record: BankRecord,
    ) -> Result<(), rusqlite::Error> {
        let sql: &str = "INSERT INTO banks (date, amount, aid) VALUES (?1, ?2, ?3)";
        let rs = self
            .conn
            .execute(sql, rusqlite::params!(record.date, record.amount, aid));
        match rs {
            Ok(_) => {
                println!("Added bank record");
            }
            Err(error) => {
                panic!("Unable to add bank record: {}", error);
            }
        }
        Ok(())
    }

    pub fn get_bank_value(&mut self, aid: u32) -> Result<BankRecord, rusqlite::Error> {
        let sql: &str = "SELECT * FROM banks WHERE aid = (?1)";
        match self.conn.prepare(sql) {
            Ok(mut stmt) => {
                let exists = stmt.exists([aid])?;
                if exists {
                    let mut entries = stmt
                        .query_map([aid], |row| {
                            Ok(BankRecord {
                                date: row.get(0)?,
                                amount: row.get(1)?,
                            })
                        })
                        .unwrap()
                        .collect::<Vec<_>>();

                    let mut latest_record = entries.pop().unwrap().unwrap();
                    for entry in entries {
                        let record = entry.unwrap();
                        if NaiveDate::parse_from_str(record.date.as_str(), "%Y-%m-%d").unwrap()
                            > NaiveDate::parse_from_str(latest_record.date.as_str(), "%Y-%m-%d")
                                .unwrap()
                        {
                            latest_record = record;
                        }
                    }

                    Ok(latest_record)
                } else {
                    panic!("Unable to find entries for the account id: {}", aid);
                }
            }
            Err(error) => {
                panic!("Unable to retrieve bank account information: {}", error);
            }
        }
    }

    // returns a vector of bank history within period and the just before that time
    pub fn get_bank_history(
        &mut self,
        aid: u32,
        date_start: String,
        date_end: String,
    ) -> Result<(Vec<BankRecord>, BankRecord), rusqlite::Error> {
        let sql: &str = "SELECT * FROM banks WHERE aid = (?1) and date >= (?2) and date <= (?3) ORDER BY date ASC";
        let p = rusqlite::params![aid, date_start, date_end];

        let mut history: Vec<BankRecord> = Vec::new();
        let mut initial: BankRecord = BankRecord {
            amount: 0.00,
            date: "1970-01-01".to_string(),
        };

        match self.conn.prepare(sql) {
            Ok(mut stmt) => {
                let exists = stmt.exists(p)?;
                if exists {
                    let entries = stmt
                        .query_map(p, |row| {
                            Ok(BankRecord {
                                date: row.get(0)?,
                                amount: row.get(1)?,
                            })
                        })
                        .unwrap()
                        .collect::<Vec<_>>();

                    history = Vec::new();
                    let mut record;
                    for entry in entries {
                        record = entry.unwrap();
                        history.push(record);
                    }
                } else {
                    panic!("Unable to find entries for the account id: {}", aid);
                }
            }
            Err(error) => {
                panic!("Unable to retrieve bank account information: {}", error);
            }
        }

        let sql: &str = "SELECT * FROM banks WHERE aid = (?1) and date <= (?2) ORDER BY date DESC";
        let p = rusqlite::params![aid, date_start];
        match self.conn.prepare(sql) {
            Ok(mut stmt) => {
                let exists = stmt.exists(p)?;
                if exists {
                    initial = stmt
                        .query_row(p, |row| {
                            Ok(BankRecord {
                                date: row.get(0)?,
                                amount: row.get(1)?,
                            })
                        })
                        .unwrap();
                } else {
                    // choosing not to do nothing for now in the event that a user
                    // chooses a date range that does not return a value
                    // this will return an initial value of 0 indicating
                    // that the account did not exist
                    // may want to do more in the future, but its good enough
                    // for now
                }
            }
            Err(error) => {
                panic!("Unable to retrieve bank account information: {}", error);
            }
        }

        Ok((history, initial))
    }
}
