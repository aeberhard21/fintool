// use crate::ledger::Ledger;
// use crate::ledger::LedgerEntry;
use rusqlite::Result;

use super::DbConn;

pub struct LedgerEntry {
    pub date: String,
    pub amount: f32,
    pub transfer_type: TransferType,
    pub payee_id: u32,
    pub category_id: u32,
    pub description: String,
}

pub enum TransferType {
    WidthdrawalToExternalAccount,
    DepositFromExternalAccount,
    WidthdrawalToInternalAccount,
    DepositFromInternalAccount,
}

impl From<u32> for TransferType {
    fn from(value: u32) -> Self {
        match value { 
            0 => TransferType::WidthdrawalToExternalAccount, 
            1 => TransferType::DepositFromExternalAccount, 
            2 => TransferType::WidthdrawalToInternalAccount, 
            3 => TransferType::DepositFromInternalAccount,
            _ => panic!("Invalid numeric value for TransferType!")
        }
    }
}

impl DbConn {
    pub fn create_ledger_table(&mut self) -> Result<()> {
        let sql: &str;
        sql = "CREATE TABLE IF NOT EXISTS ledgers (
                date        TEXT NOT NULL, 
                amount      REAL NOT NULL, 
                transfer_type     INTEGER NOT NULL, 
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
        sql = "INSERT INTO ledgers ( date, amount, transfer_type, pid, cid, desc, aid) VALUES ( ?1, ?2, ?3, ?4, ?5, ?6, ?7)";
        let rs = self.conn.execute(
            sql,
            (
                entry.date.to_string(),
                entry.amount,
                entry.transfer_type as u32,
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
    
}
