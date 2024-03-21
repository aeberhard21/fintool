use super::DbConn;
use rusqlite::Result;

pub struct BankRecord {
    pub amount: f32,
    pub date: i64,
}

impl DbConn {
    pub fn create_bank_table(&mut self) -> Result<()> {
        let sql: &str = " CREATE TABLE IF NOT EXISTS banks ( 
                    date    INTEGER NOT NULL, 
                    amount  REAL NOT NULL,
                    aid     INTEGER, 
                    FOREIGN KEY (aid) REFERENCES accounts(id)
            )";
        let rs = self.conn.execute(sql, ());
        match rs {
            Ok(_) => {
                println!("Created bank table!");
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
                        if record.date > latest_record.date {
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
}
