use super::participants::ParticipantType;
use crate::database::DbConn;
use chrono::NaiveDate;
use inquire::autocompletion::Replacement;
use inquire::*;
use rusqlite::Result;
use shared_lib::TransferType;

#[derive(Clone, Debug)]
pub struct LedgerInfo {
    pub date: String,
    pub amount: f32,
    pub transfer_type: TransferType,
    pub participant: u32,
    pub category_id: u32,
    pub description: String,
    pub ancillary_f32data : f32
}

#[derive(Clone)]
pub struct LedgerRecord {
    pub id: u32,
    pub info: LedgerInfo,
}

impl DbConn {
    pub fn create_ledger_table(&mut self) -> Result<()> {
        let sql: &str;
        sql = "CREATE TABLE IF NOT EXISTS ledgers (
                id          INTEGER NOT NULL,
                date        TEXT NOT NULL, 
                amount      REAL NOT NULL, 
                transfer_type INTEGER NOT NULL, 
                pid         INTEGER NOT NULL, 
                cid         INTEGER NOT NULL,
                desc        TEXT,
                ancillary_f32 REAL NOT NULL, 
                aid         INTEGER NOT NULL,
                uid         INTEGER NOT NULL,
                PRIMARY KEY(uid, aid, id),
                FOREIGN KEY(uid,aid) REFERENCES accounts(uid,id),
                FOREIGN KEY(uid, aid, cid) REFERENCES categories(uid, aid, id),
                FOREIGN KEY(uid, aid, pid) REFERENCES people(uid, aid, id),
                FOREIGN KEY(uid) REFERENCES users(id)
            )";

        let rs = self.conn.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create: {}", error)
            }
        }
        Ok(())
    }

    pub fn add_ledger_entry(
        &mut self,
        uid: u32,
        aid: u32,
        entry: LedgerInfo,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let sql: &str;
        let id = self.get_next_ledger_id(uid, aid).unwrap();
        sql = "INSERT INTO ledgers ( id, date, amount, transfer_type, pid, cid, desc, ancillary_f32, aid, uid) VALUES ( ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)";
        let rs = self.conn.execute(
            sql,
            (
                id,
                entry.date.to_string(),
                entry.amount,
                entry.transfer_type as u32,
                entry.participant,
                entry.category_id,
                entry.description,
                entry.ancillary_f32data,
                aid,
                uid,
            ),
        );
        match rs {
            Ok(_usize) => {
                // println!("Added statement");
            }
            Err(error) => {
                panic!("Unable to add ledger: {}", error);
            }
        }
        Ok(id)
    }

    pub fn update_ledger_item(
        &mut self,
        uid : u32,
        aid : u32,
        update: LedgerRecord,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let p = rusqlite::params![
            update.id,
            update.info.date,
            update.info.amount,
            update.info.transfer_type as u32,
            update.info.participant,
            update.info.category_id,
            update.info.description,
            update.info.ancillary_f32data,
            uid,
            aid
        ];
        let sql = "UPDATE ledgers SET date = ?2, amount = ?3, transfer_type = ?4, pid = ?5, cid = ?6, desc = ?7, ancillary_f32 = ?8 WHERE id = ?1 and uid = ?9 and aid = ?10";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update ledger: {}", error);
            }
        }
        Ok(update.id)
    }

    pub fn remove_ledger_item(&mut self, uid: u32, aid :u32, id: u32) -> rusqlite::Result<u32, rusqlite::Error> {
        let p = rusqlite::params![id, uid, aid];
        let sql = "DELETE FROM ledgers WHERE id = ?1 and uid = ?2 and aid = ?3";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove ledger item: {}", error);
            }
        }

        let sql = "UPDATE ledgers SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove ledger item: {}", error);
            }
        }
        Ok(id)
    }

    pub fn get_ledger(&mut self, uid: u32, aid: u32) -> rusqlite::Result<Vec<LedgerRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT id, date, amount, transfer_type, pid, cid, desc, ancillary_f32 FROM ledgers WHERE aid = (?1) and uid = (?2) order by date DESC";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut entries: Vec<LedgerRecord> = Vec::new();
        match exists {
            true => {
                let found_entries = stmt
                    .query_map(p, |row| {
                        Ok(LedgerRecord {
                            id: row.get(0)?,
                            info: LedgerInfo {
                                date: row.get(1)?,
                                amount: row.get(2)?,
                                transfer_type: TransferType::from(row.get::<_, u32>(3)? as u32),
                                participant: row.get(4)?,
                                category_id: row.get(5)?,
                                description: row.get(6)?,
                                ancillary_f32data : row.get(7)?,
                            },
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();

                for entry in found_entries {
                    entries.push(entry.unwrap());
                }
                Ok(entries)
            }
            false => {
                return Ok(entries);
            }
        }
    }

    pub fn get_ledger_entries_within_timestamps(
        &mut self,
        uid: u32,
        aid: u32,
        start: NaiveDate,
        end: NaiveDate,
    ) -> rusqlite::Result<Vec<LedgerInfo>, rusqlite::Error> {
        let p = rusqlite::params![aid, start.to_string(), end.to_string(), uid];
        let sql = "SELECT * FROM ledgers WHERE aid = (?1) and date >= (?2) and date <= (?3) and uid = (?4) ORDER by date ASC";

        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut entries: Vec<LedgerInfo> = Vec::new();
        match exists {
            true => {
                let found_entries = stmt
                    .query_map(p, |row| {
                        Ok(LedgerInfo {
                            date: row.get(1)?,
                            amount: row.get(2)?,
                            transfer_type: TransferType::from(row.get::<_, u32>(3)? as u32),
                            participant: row.get(4)?,
                            category_id: row.get(5)?,
                            description: row.get(6)?,
                            ancillary_f32data : row.get(7)?
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();

                for entry in found_entries {
                    entries.push(entry.unwrap());
                }
                Ok(entries)
            }
            false => {
                return Ok(entries);
            }
        }
    }

    pub fn get_current_value(&mut self, uid: u32, aid: u32) -> rusqlite::Result<f32, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let mut sum: f32 = 0.0;
        let sql: &str ="SELECT COALESCE(SUM(CASE
                WHEN transfer_type == 0 or transfer_type = 2 THEN -amount    -- withdrawal
                WHEN transfer_type == 1 or transfer_type = 3 THEN amount     -- deposit from external account
                ELSE 0 
            END), 0) as total_balance FROM ledgers WHERE aid = (?1) and uid = (?2);";

        let mut stmt = self.conn.prepare(sql)?;
        if stmt.exists(p)? {
            sum = stmt.query_row(p, |row| row.get(0))?;
        } else {
            panic!("Not found!");
        }

        Ok(sum)
    }

    pub fn get_cumulative_total_of_ledger_before_date(
        &mut self,
        uid : u32,
        aid: u32,
        end: NaiveDate,
    ) -> rusqlite::Result<f32, rusqlite::Error> {
        let p = rusqlite::params![aid, end.to_string(), uid];
        let mut sum: f32 = 0.0;
        let sql = "SELECT COALESCE(SUM(CASE
            WHEN transfer_type == 0 or transfer_type = 2 THEN -amount    -- withdrawal
            WHEN transfer_type == 1 or transfer_type = 3 THEN amount     -- deposit from external account
            ELSE 0 
        END), 0) as total_balance FROM ledgers WHERE aid = (?1) and date <= (?2) and uid = (?3);";

        let mut stmt = self.conn.prepare(sql)?;
        if stmt.exists(p)? {
            sum = stmt.query_row(p, |row| row.get(0))?;
        } else {
            panic!("Not found!");
        }
        Ok(sum)
    }

    pub fn get_participants(
        &mut self,
        uid: u32, 
        aid: u32,
        transfer_type: TransferType,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let sql;
        let p = rusqlite::params![aid, transfer_type as u32, uid];
        sql = "SELECT p.name FROM people p JOIN ledgers l ON p.id = l.cid WHERE l.aid = (?1) and l.transfer_type = (?2) and l.uid = (?3)";

        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut participants: Vec<String> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let party = stmt
                    .query_map(p, |row| Ok(row.get(0)?))
                    .unwrap()
                    .collect::<Vec<_>>();
                for participant in party {
                    participants.push(participant.unwrap());
                }
            }
            false => {}
        }
        Ok(participants)
    }
}
