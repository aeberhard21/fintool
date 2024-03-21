// use crate::ledger::Ledger;
// use crate::ledger::LedgerEntry;
use rusqlite::Result;

use super::DbConn;

pub struct LedgerEntry {
    pub date: String,
    pub amount: f32,
    pub deposit: bool,
    pub payee_id: u32,
    pub category_id: u32,
    pub description: String,
}

impl DbConn {
    pub fn create_ledger_table(&mut self) -> Result<()> {
        let sql: &str;
        sql = "CREATE TABLE IF NOT EXISTS ledgers (
                date        TEXT NOT NULL, 
                amount      REAL NOT NULL, 
                deposit     INTEGER NOT NULL, 
                pid         INTEGER NOT NULL, 
                cid         INTEGER NOT NULL,
                desc        TEXT,
                aid         INTEGER,
                FOREIGN KEY(aid) REFERENCES accounts(id)
                FOREIGN KEY(cid) REFERENCES categories(id)
                FOREIGN KEY(pid) REFERENCES people(id)
            )";

        let rs = self.conn.execute(sql, ());
        match rs {
            Ok(_) => {
                println!("Created!")
            }
            Err(error) => {
                panic!("Unable to create: {}", error)
            }
        }
        Ok(())
    }

    pub fn add_ledger_entry(&mut self, aid: u32, entry: LedgerEntry) -> Result<()> {
        let sql: &str;
        sql = "INSERT INTO ledgers ( date, amount, deposit, pid, cid, desc, aid) VALUES ( ?1, ?2, ?3, ?4, ?5, ?6, ?7)";
        let rs = self.conn.execute(
            sql,
            (
                entry.date.to_string(),
                entry.amount,
                entry.deposit,
                entry.payee_id,
                entry.category_id,
                entry.description,
                aid,
            ),
        );
        match rs {
            Ok(_usize) => {
                println!("Added statement");
            }
            Err(Error) => {
                println!("Unable to add ledger: {}", Error);
            }
        }
        Ok(())
    }

    // pub fn read_ledger(&mut self, name: String) -> Result<Ledger> {
    //     let mut ledger: Ledger = Ledger::new(name.as_str());
    //     let sql: String;
    //     sql = format!("SELECT * FROM {}", name);
    //     let mut rs = self.conn.prepare(sql.as_str()).unwrap();
    //     let mut rows;
    //     rows = rs
    //         .query_map([], |row| {
    //             Ok(LedgerEntry {
    //                 date: row.get(0)?,
    //                 amount: row.get(1)?,
    //                 deposit: row.get(2)?,
    //                 payee: row.get(3)?,
    //                 description: row.get(4)?,
    //             })
    //         })
    //         .unwrap();
    //     for row in rows {
    //         ledger.add(row.unwrap());
    //     }
    //     ledger.print();
    //     return Ok(ledger);
    // }

    // fn get_ledger_entry(self, )
}
