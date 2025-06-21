
use inquire::autocompletion;
use inquire::autocompletion::Replacement;
use inquire::{Autocomplete, CustomUserError};
use rusqlite::Result;
use shared_lib::TransferType;

use crate::database::DbConn;

#[derive(Clone, Copy)]
pub enum ParticipantType {
    Payee,
    Payer,
    Both,
}

pub struct Participant {
    pub name: String,
    pub ptype: ParticipantType,
}

pub struct ParticipantRecord {
    pub id: u32,
    pub participant: Participant,
}

impl ParticipantType {
    fn to_string(self) -> String {
        match self {
            Self::Payee => "Payee".to_string(),
            Self::Payer => "Payer".to_string(),
            Self::Both => "Both".to_string(),
        }
    }
}

impl From<u32> for ParticipantType {
    fn from(value: u32) -> Self {
        match value {
            0 => ParticipantType::Payee,
            1 => ParticipantType::Payer,
            _ => panic!("Invalid numeric value for ParticipantType!"),
        }
    }
}

impl DbConn {
    pub fn create_people_table(&self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS people ( 
                id          INTEGER NOT NULL,
                aid         INTEGER NOT NULL,
                type        INTEGER NOT NULL, 
                name        TEXT NOT NULL,
                uid         INTEGER NOT NULL,
                is_account  BOOL NOT NULL,
                PRIMARY KEY (uid, aid, id),
                FOREIGN KEY(uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE,
                FOREIGN KEY(uid) REFERENCES users(id)
            )";

        self.conn.lock().unwrap()   
            .execute(sql, ())
            .expect("Unable to initialize people table!");
        Ok(())
    }

    pub fn add_participant(
        &self,
        uid: u32,
        aid: u32,
        ptype: ParticipantType,
        name: String,
        is_account : bool
    ) -> Result<u32> {
        let id = self.get_next_people_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, aid, ptype as u32, name, uid, is_account);
        let sql = "INSERT INTO people (id, aid, type, name, uid, is_account) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add {} {} for account {}: {}",
                    ptype.to_string(),
                    name,
                    aid,
                    error
                );
            }
        }
    }

    pub fn get_participant_id(
        &self,
        uid : u32,
        aid: u32,
        name: String,
        ptype: ParticipantType,
    ) -> Option<u32> {
        let sql = "SELECT id FROM people WHERE aid = (?1) and name = (?2) and type = (?3) and uid = (?4)";
        let p = rusqlite::params![aid, name, ptype as u32, uid];
        let conn_lock = self.conn.lock().unwrap();
        let prepared_stmt = conn_lock.prepare(sql);
        match prepared_stmt {
            Ok(mut stmt) => {
                if let Ok(entry_found) = stmt.exists(p) {
                    if entry_found {
                        let id = stmt
                            .query_row(p, |row: &rusqlite::Row<'_>| row.get::<_, u32>(0))
                            .unwrap();
                        return Some(id);
                    } else {
                        return None;
                    }
                } else {
                    panic!("Unable to determine if exists!");
                }
            }
            Err(e) => {
                panic!(
                    "SQLITE error {} while executing searching for person {}.",
                    e.to_string(),
                    name
                );
            }
        }
    }

    pub fn get_participant(&self, uid : u32, aid: u32, pid: u32) -> Option<String> {
        let sql = "SELECT name FROM people WHERE aid = (?1) and id = (?2) and uid = (?3)";
        let p = rusqlite::params![aid, pid, uid];
        let conn = self.conn.lock().unwrap();
        let prepared_stmt = conn.prepare(sql);
        match prepared_stmt {
            Ok(mut stmt) => {
                if let Ok(entry_found) = stmt.exists(p) {
                    if entry_found {
                        let name: String = stmt
                            .query_row(p, |row: &rusqlite::Row<'_>| row.get::<_, String>(0))
                            .unwrap();
                        return Some(name);
                    } else {
                        return None;
                    }
                } else {
                    panic!("Unable to determine if exists!");
                }
            }
            Err(e) => {
                panic!(
                    "SQLITE error {} while executing searching for person {}.",
                    e.to_string(),
                    pid
                );
            }
        }
    }

    pub fn check_and_add_participant(
        &self,
        uid: u32, 
        aid: u32,
        name: String,
        ptype: ParticipantType,
        is_account : bool
    ) -> u32 {
        let sql = "SELECT id FROM people WHERE aid = (?1) and name = (?2) and (type = (?3) or type = (?4)) and uid = (?5)";
        let p = rusqlite::params![aid, name, ptype as u32, ParticipantType::Both as u32, uid];
        let cloned_conn = self.conn.clone();
        let conn_lock = cloned_conn.lock().unwrap();
        {
            let prepared_stmt = conn_lock.prepare(sql);
            match prepared_stmt {
                Ok(mut stmt) => {
                    if let Ok(entry_found) = stmt.exists(p) {
                        if entry_found {
                            let id = stmt
                                .query_row(p, |row: &rusqlite::Row<'_>| row.get::<_, u32>(0))
                                .unwrap();
                            return id;
                        } else {
                            // self.add_participant(uid, aid, ptype, name, is_account).unwrap()
                        }
                    } else {
                        panic!("Unable to determine if exists!");
                    }
                }
                Err(e) => {
                    panic!(
                        "SQLITE error {} while executing searching for person {}.",
                        e.to_string(),
                        name
                    );
                }
            }
        }
        std::mem::drop(conn_lock);
        self.add_participant(uid, aid, ptype, name, is_account).unwrap()
    }

    pub fn get_participants(&self, uid: u32, aid: u32, ptype: ParticipantType) -> Result<Vec<ParticipantRecord>, rusqlite::Error> {
        let (sql, p) = if ptype as u32 == ParticipantType::Both as u32 {
            ("SELECT id, name, type FROM people WHERE uid = (?1) and aid = (?2)",
            rusqlite::params![uid, aid])
        } else {
            ("SELECT id, name, type FROM people WHERE uid = (?1) and aid = (?2) and type = (?3)",
            rusqlite::params![uid, aid, ptype as u32])
        };
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut participants: Vec<ParticipantRecord> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let party = stmt
                    .query_map(p, |row| Ok(
                        ParticipantRecord { 
                            id : row.get(0)?, 
                            participant : Participant { 
                                name: row.get(1)?, 
                                ptype: ParticipantType::from(row.get::<_, u32>(2)? as u32)
                            }
                    }))
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

    pub fn get_participants_by_transfer_type(
        &self,
        uid: u32, 
        aid: u32,
        transfer_type: TransferType,
        include_accounts : bool
    ) -> Result<Vec<String>, rusqlite::Error> {
        let sql;
        let p = rusqlite::params![aid, transfer_type as u32, uid, include_accounts];
        sql = if include_accounts {
            "SELECT p.name FROM people p JOIN ledgers l ON p.id = l.cid WHERE l.aid = (?1) and l.transfer_type = (?2) and l.uid = (?3) and l.uid = (?4)"
        } else { 
            "SELECT p.name FROM people p JOIN ledgers l ON p.id = l.cid WHERE l.aid = (?1) and l.transfer_type = (?2) and l.uid = (?3) and l.uid = (?4) and p.is_account = false"
        };

        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut participants: Vec<String> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
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

    pub fn update_participant_name(&self, uid : u32, aid: u32, ptype : ParticipantType, old : String, new : String) -> Result<String> {
        let (p, sql) = match ptype { 
            ParticipantType::Both => {
                (
                    rusqlite::params![uid, aid, old, new],
                    "UPDATE people SET name = (?4) WHERE uid = (?1) and aid = (?2) and name = (?3)"
                )
            } 
            _ => {
                (
                    rusqlite::params![uid, aid, old, new, ptype as u32],
                    "UPDATE people SET name = (?4) WHERE uid = (?1) and aid = (?2) and name = (?3) and type = (?5)"
                )
            }
        };
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(new),
            Err(error) => {
                panic!("Unable to update participant {} in account {}: {}!", old, aid, error);
            }
        }
    }

    pub fn remove_participant(&self, uid : u32, aid : u32, ptype: ParticipantType, name : String) -> rusqlite::Result<Option<u32>, rusqlite::Error> {
        let id_opt = self.get_participant_id(uid, aid, name.clone(), ptype);
        if id_opt.is_none() {
            return Ok(None);
        }
        let id = id_opt.unwrap();
        let p = rusqlite::params![uid, aid, name, ptype as u32];
        let sql =  "DELETE FROM people WHERE uid = (?1) and aid = (?2) and name = (?3) and type = (?4)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!("Unable to remove participant: {}!", error);
            }
        }
        let p = rusqlite::params![id, uid, aid];
        let sql = "UPDATE people SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!("Unable to update participants ids: {}!", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET pid = pid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!("Unable to update 'pid' value in 'user_account_info': {}!", error);
            }
        }

        Ok(Some(id)) 
    }
}

#[derive(Clone)]
pub struct ParticipantAutoCompleter {
    pub uid: u32,
    pub aid: u32,
    pub db: DbConn,
    pub ptype: ParticipantType,
    pub with_accounts : bool,
}

impl Autocomplete for ParticipantAutoCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        let suggestions: Vec<String>;
        if !self.with_accounts { 
            suggestions = match self.ptype {
                ParticipantType::Payee => {
                    let mut x: Vec<String> = self
                        .db
                        .get_participants_by_transfer_type(self.uid, self.aid, TransferType::WithdrawalToExternalAccount, false)
                        .unwrap()
                        .into_iter()
                        .filter(|name| name.starts_with(input.to_ascii_uppercase().as_str()))
                        .collect();
                    let mut y: Vec<String> = self
                        .db
                        .get_participants_by_transfer_type(self.uid, self.aid, TransferType::WithdrawalToInternalAccount, false)
                        .unwrap()
                        .into_iter()
                        .filter(|name| name.starts_with(input.to_ascii_uppercase().as_str()))
                        .collect();
                    x.dedup();
                    y.dedup();
                    [x, y].concat()
                }
                ParticipantType::Payer => {
                    let mut x: Vec<String> = self
                        .db
                        .get_participants_by_transfer_type(self.uid, self.aid, TransferType::DepositFromExternalAccount, false)
                        .unwrap()
                        .into_iter()
                        .filter(|name| name.starts_with(input.to_ascii_uppercase().as_str()))
                        .collect();
                    let mut y: Vec<String> = self
                        .db
                        .get_participants_by_transfer_type(self.uid, self.aid, TransferType::DepositFromInternalAccount, false)
                        .unwrap()
                        .into_iter()
                        .filter(|name| name.starts_with(input.to_ascii_uppercase().as_str()))
                        .collect();
                    x.dedup();
                    y.dedup();
                    [x, y].concat()
                }
                ParticipantType::Both => {
                    let mut w: Vec<String> = self
                        .db
                        .get_participants_by_transfer_type(self.uid, self.aid, TransferType::WithdrawalToExternalAccount, false)
                        .unwrap()
                        .into_iter()
                        .filter(|name| name.starts_with(input.to_ascii_uppercase().as_str()))
                        .collect();
                    let mut x: Vec<String> = self
                        .db
                        .get_participants_by_transfer_type(self.uid, self.aid, TransferType::WithdrawalToInternalAccount, false)
                        .unwrap()
                        .into_iter()
                        .filter(|name| name.starts_with(input.to_ascii_uppercase().as_str()))
                        .collect();
                    let mut y: Vec<String> = self
                        .db
                        .get_participants_by_transfer_type(self.uid, self.aid, TransferType::DepositFromInternalAccount, false)
                        .unwrap()
                        .into_iter()
                        .filter(|name| name.starts_with(input.to_ascii_uppercase().as_str()))
                        .collect();
                    let mut z: Vec<String> = self
                        .db
                        .get_participants_by_transfer_type(self.uid, self.aid, TransferType::DepositFromExternalAccount, false)
                        .unwrap()
                        .into_iter()
                        .filter(|name| name.starts_with(input.to_ascii_uppercase().as_str()))
                        .collect();
                    w.dedup();
                    x.dedup();
                    y.dedup();
                    z.dedup();
                    [[w, x].concat(), [y, z].concat()].concat()
                }
                _ => {
                    panic!("Unable to match ParticipantType in Autocomplete!");
                }
            };
        } else { 
            let current_account_name = self.db.get_account_name(self.uid, self.aid).unwrap();
            let mut x : Vec<String> = self.db.get_user_accounts(self.uid).unwrap().iter().map(|acct| acct.info.name.clone()).filter(|x| *x!=current_account_name).collect();
            x.push("New Account".to_ascii_uppercase().to_string());
            x.push("None".to_ascii_uppercase().to_string());
            suggestions = x;
        }

        Ok(suggestions)
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<autocompletion::Replacement, CustomUserError> {
        Ok(match highlighted_suggestion {
            Some(suggestion) => Replacement::Some(suggestion),
            None => {
                let suggestions = self.get_suggestions(input.to_ascii_uppercase().as_str()).unwrap();
                if suggestions.len() == 0 {
                    autocompletion::Replacement::None
                } else {
                    Some(suggestions[0].clone())
                }
            }
        })
    }
}
