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
}

#[derive(Clone, Debug)]
pub struct LedgerRecord {
    pub id: u32,
    pub info: LedgerInfo,
}

#[derive(Clone, Debug)]
pub struct DisplayableLedgerInfo {
    pub date: String,
    pub amount: String,
    pub transfer_type: String,
    pub participant: String,
    pub category: String,
    pub description: String,
    pub labels: String,
}

#[derive(Clone, Debug)]
pub struct DisplayableLedgerRecord {
    pub id: String,
    pub info: DisplayableLedgerInfo,
}

#[derive(Debug, Clone)]
pub struct Expenditure {
    pub category: String,
    pub amount: f32,
}

impl DbConn {
    pub fn create_ledger_table(&self) -> Result<()> {
        let sql: &str;
        sql = "CREATE TABLE IF NOT EXISTS ledgers (
                id          INTEGER NOT NULL,
                date        TEXT NOT NULL, 
                amount      REAL NOT NULL, 
                transfer_type INTEGER NOT NULL, 
                pid         INTEGER NOT NULL, 
                cid         INTEGER NOT NULL,
                desc        TEXT,
                aid         INTEGER NOT NULL,
                uid         INTEGER NOT NULL,
                PRIMARY KEY(uid, aid, id),
                FOREIGN KEY(uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE,
                FOREIGN KEY(uid, aid, cid) REFERENCES categories(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
                FOREIGN KEY(uid, aid, pid) REFERENCES people(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
                FOREIGN KEY(uid) REFERENCES users(id)
            )";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create: {}", error)
            }
        }
        Ok(())
    }

    pub fn add_ledger_entry(
        &self,
        uid: u32,
        aid: u32,
        entry: LedgerInfo,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let sql: &str;
        let id = self.get_next_ledger_id(uid, aid).unwrap();
        sql = "INSERT INTO ledgers ( id, date, amount, transfer_type, pid, cid, desc, aid, uid) VALUES ( ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(
            sql,
            (
                id,
                entry.date.to_string(),
                entry.amount,
                entry.transfer_type as u32,
                entry.participant,
                entry.category_id,
                entry.description,
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
        &self,
        uid: u32,
        aid: u32,
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
            uid,
            aid
        ];
        let sql = "UPDATE ledgers SET date = ?2, amount = ?3, transfer_type = ?4, pid = ?5, cid = ?6, desc = ?7  WHERE id = ?1 and uid = ?8 and aid = ?9";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update ledger: {}", error);
            }
        }
        Ok(update.id)
    }

    pub fn remove_ledger_item(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let p = rusqlite::params![id, uid, aid];
        let sql = "DELETE FROM ledgers WHERE id = ?1 and uid = ?2 and aid = ?3";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove ledger item: {}", error);
            }
        }

        let sql = "UPDATE ledgers SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove ledger item: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET lid = lid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'lid' value in 'user_account_info': {}",
                    error
                );
            }
        }
        Ok(id)
    }

    pub fn get_ledger(
        &self,
        uid: u32,
        aid: u32,
    ) -> rusqlite::Result<Vec<LedgerRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT id, date, amount, transfer_type, pid, cid, desc FROM ledgers WHERE aid = (?1) and uid = (?2) order by date DESC";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
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

    pub fn get_displayable_ledger(
        &self,
        uid: u32,
        aid: u32,
    ) -> rusqlite::Result<Vec<DisplayableLedgerRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "
            SELECT l.id, l.date, l.amount, l.transfer_type, p.name, c.category, l.desc, COALESCE(GROUP_CONCAT(labels.label, ', '), '') AS label_list 
            FROM ledgers l 
            INNER JOIN categories c ON
                l.cid = c.id AND
                l.uid = c.uid AND
                l.aid = c.aid
            INNER JOIN people p ON
                l.pid = p.id AND
                l.uid = p.uid AND
                l.aid = p.aid
            LEFT JOIN label_allocations ON
                label_allocations.ledger_id = l.id AND
                label_allocations.uid = l.uid AND
                label_allocations.aid = l.aid
            LEFT JOIN labels ON
                labels.id = label_allocations.label_id
            WHERE l.aid = (?1) and l.uid = (?2)
            GROUP BY l.id 
            ORDER BY date DESC";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut entries: Vec<DisplayableLedgerRecord> = Vec::new();
        match exists {
            true => {
                let found_entries = stmt
                    .query_map(p, |row| {
                        Ok(DisplayableLedgerRecord {
                            id: row.get::<_, u32>(0)?.to_string(),
                            info: DisplayableLedgerInfo {
                                date: row.get(1)?,
                                amount: row.get::<_, f32>(2)?.to_string(),
                                transfer_type: format!(
                                    "{}",
                                    TransferType::from_repr(row.get::<_, u32>(3)? as usize)
                                        .unwrap()
                                ),
                                participant: row.get(4)?,
                                category: row.get(5)?,
                                description: row.get(6)?,
                                labels: row.get(7)?,
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

    pub fn get_full_ledger(
        &self,
        uid: u32,
        aid: u32,
    ) -> rusqlite::Result<Vec<LedgerRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT id, date, amount, transfer_type, pid, cid, desc FROM ledgers WHERE aid = (?1) and uid = (?2) order by date DESC";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
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

    pub fn check_if_ledger_references_category(
        &self,
        uid: u32,
        aid: u32,
        category: String,
    ) -> rusqlite::Result<Option<Vec<LedgerRecord>>, rusqlite::Error> {
        let p = rusqlite::params![uid, aid, category];
        let sql = "
            SELECT 
                l.id, l.date, l.amount, l.transfer_type, l.pid, l.cid, l.desc 
            FROM ledgers AS l
            INNER JOIN categories ON 
                l.cid = categories.id AND
                l.aid = categories.aid AND
                l.uid = categories.uid
            WHERE
                l.uid = (?1) and
                l.aid = (?2) and
                categories.category = (?3)
        ";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        if exists {
            let matched_record_wrap = stmt
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
                        },
                    })
                })
                .unwrap()
                .collect::<Vec<_>>();

            let mut records: Vec<LedgerRecord> = Vec::new();
            for wrapped_record in matched_record_wrap {
                records.push(wrapped_record.unwrap());
            }

            return Ok(Some(records));
        }
        return Ok(None);
    }

    pub fn check_if_ledger_references_participant(
        &self,
        uid: u32,
        aid: u32,
        ptype: ParticipantType,
        name: String,
    ) -> rusqlite::Result<Option<Vec<LedgerRecord>>, rusqlite::Error> {
        let (p, sql) = match ptype {
            ParticipantType::Both => {
                (
                    rusqlite::params![uid, aid, name],
                    "
                        SELECT 
                            l.id, l.date, l.amount, l.transfer_type, l.pid, l.cid, l.desc 
                        FROM ledgers AS l
                        INNER JOIN people ON 
                            l.cid = people.id AND
                            l.aid = people.aid AND
                            l.uid = people.uid
                        WHERE
                            l.uid = (?1) and
                            l.aid = (?2) and
                            people.name = (?3)
                    "
                )
            }
            _ => {
                (
                    rusqlite::params![uid, aid, name, ptype as u32],
                    "
                        SELECT 
                            l.id, l.date, l.amount, l.transfer_type, l.pid, l.cid, l.desc 
                        FROM ledgers AS l
                        INNER JOIN people ON 
                            l.cid = people.id AND
                            l.aid = people.aid AND
                            l.uid = people.uid
                        WHERE
                            l.uid = (?1) and
                            l.aid = (?2) and
                            people.name = (?3) and
                            people.type = (?4)
                    "
                )
            }
        };

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        if exists {
            let matched_record_wrap = stmt
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
                        },
                    })
                })
                .unwrap()
                .collect::<Vec<_>>();

            let mut records: Vec<LedgerRecord> = Vec::new();
            for wrapped_record in matched_record_wrap {
                records.push(wrapped_record.unwrap());
            }

            return Ok(Some(records));
        }
        return Ok(None);
    }

    pub fn get_ledger_entries_within_timestamps(
        &self,
        uid: u32,
        aid: u32,
        start: NaiveDate,
        end: NaiveDate,
    ) -> rusqlite::Result<Vec<LedgerRecord>, rusqlite::Error> {
        let p = rusqlite::params![
            aid,
            start.format("%Y-%m-%d").to_string(),
            end.format("%Y-%m-%d").to_string(),
            uid
        ];
        let sql = "SELECT * FROM ledgers WHERE aid = (?1) and date >= (?2) and date <= (?3) and uid = (?4) ORDER by date ASC";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
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

    pub fn get_current_value(&self, uid: u32, aid: u32) -> rusqlite::Result<f32, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let mut sum: f32 = 0.0;
        let sql: &str ="SELECT COALESCE(SUM(CASE
                WHEN transfer_type == 0 or transfer_type = 2 THEN -amount    -- withdrawal
                WHEN transfer_type == 1 or transfer_type = 3 THEN amount     -- deposit from external account
                ELSE 0 
            END), 0) as total_balance FROM ledgers WHERE aid = (?1) and uid = (?2);";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        if stmt.exists(p)? {
            sum = stmt.query_row(p, |row| row.get(0))?;
        } else {
            panic!("Not found!");
        }

        Ok(sum)
    }

    pub fn get_cumulative_total_of_ledger_before_date(
        &self,
        uid: u32,
        aid: u32,
        end: NaiveDate,
    ) -> rusqlite::Result<Option<f32>, rusqlite::Error> {
        let p = rusqlite::params![aid, end.format("%Y-%m-%d").to_string(), uid];
        let mut sum: f32 = 0.0;
        let sql = "SELECT COALESCE(SUM(CASE
            WHEN transfer_type == 0 or transfer_type = 2 THEN -amount    -- withdrawal
            WHEN transfer_type == 1 or transfer_type = 3 THEN amount     -- deposit from external account
            ELSE 0 
        END), 0) as total_balance FROM ledgers WHERE aid = (?1) and date < (?2) and uid = (?3);";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                sum = stmt.query_row(p, |row| row.get(0))?;
                return Ok(Some(sum));
            }
            false => {
                return Ok(None);
            }
        }
    }

    pub fn get_cumulative_total_of_ledger_on_date(
        &self,
        uid: u32,
        aid: u32,
        end: NaiveDate,
    ) -> rusqlite::Result<Option<f32>, rusqlite::Error> {
        let p = rusqlite::params![aid, end.format("%Y-%m-%d").to_string(), uid];
        let mut sum: f32 = 0.0;
        let sql = "SELECT COALESCE(SUM(CASE
            WHEN transfer_type == 0 or transfer_type = 2 THEN -amount    -- withdrawal
            WHEN transfer_type == 1 or transfer_type = 3 THEN amount     -- deposit from external account
            ELSE 0 
        END), 0) as total_balance FROM ledgers WHERE aid = (?1) and date <= (?2) and uid = (?3);";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                sum = stmt.query_row(p, |row| row.get(0))?;
                return Ok(Some(sum));
            }
            false => {
                return Ok(None);
            }
        }
    }

    pub fn get_external_transactions_between_timestamps(
        &self,
        uid: u32,
        aid: u32,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Option<Vec<LedgerRecord>>, rusqlite::Error> {
        let p = rusqlite::params![
            aid,
            start.format("%Y-%m-%d").to_string(),
            end.format("%Y-%m-%d").to_string(),
            uid
        ];
        let sql = "SELECT * FROM ledgers WHERE (transfer_type = 0 OR transfer_type = 1) AND aid = (?1) and date >= (?2) and date <= (?3) and uid = (?4) ORDER by date ASC";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
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
                            },
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();

                for entry in found_entries {
                    entries.push(entry.unwrap());
                }
                Ok(Some(entries))
            }
            false => {
                return Ok(None);
            }
        }
    }

    pub fn get_expenditures_between_dates(
        &self,
        uid: u32,
        aid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Option<Vec<Expenditure>>, rusqlite::Error> {
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
        let mut cumulative_expenses: Vec<Expenditure> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let rows = stmt
                    .query_map(p, |row| {
                        Ok(Expenditure {
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

    pub fn get_cumulative_total_of_ledger_of_external_transactions_on_date(
        &self,
        uid: u32,
        aid: u32,
        end: NaiveDate,
    ) -> rusqlite::Result<Option<f32>, rusqlite::Error> {
        let p = rusqlite::params![aid, end.format("%Y-%m-%d").to_string(), uid];
        let mut sum: f32 = 0.0;
        let sql = "SELECT COALESCE(SUM(CASE
            WHEN transfer_type == 0 THEN -amount    -- withdrawal
            WHEN transfer_type == 1 THEN amount     -- deposit from external account
            ELSE 0 
        END), 0) as total_balance FROM ledgers WHERE aid = (?1) and date <=(?2) and uid = (?3);";

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                sum = stmt.query_row(p, |row| row.get(0))?;
                return Ok(Some(sum));
            }
            false => {
                return Ok(None);
            }
        }
    }

    pub fn get_expenditures(
        &self,
        uid: u32,
        aid: u32,
    ) -> Result<Option<Vec<Expenditure>>, rusqlite::Error> {
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
        let mut cumulative_expenses: Vec<Expenditure> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let rows = stmt
                    .query_map(p, |row| {
                        Ok(Expenditure {
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
