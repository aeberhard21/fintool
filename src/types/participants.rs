use std::path::PrefixComponent;

use rusqlite::Result;
use inquire::Autocomplete;
use inquire::autocompletion::*;
use inquire::autocompletion;
use inquire::CustomUserError;

use crate::database::DbConn;

#[derive(Clone, Copy)]
pub enum ParticipantType {
    Payee,
    Payer,
    Both,
}

struct Participant {
    pub name : String, 
    pub ptype : ParticipantType,
}

pub struct ParticipantRecord {
    pub id : u32,
    pub participant : Participant
}

impl ParticipantType {
    fn to_string(self) -> String {
        match self {
            Self::Payee => "Payee".to_string(),
            Self::Payer => "Payer".to_string(),
            Self::Both  => "Both".to_string()
        }
    }
}

impl From<u32> for ParticipantType {
    fn from(value : u32) -> Self {
        match value {
            0 => ParticipantType::Payee, 
            1 => ParticipantType::Payer,
            _ => panic!("Invalid numeric value for ParticipantType!")
        }
    }
}

impl DbConn {
    pub fn create_people_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS people ( 
                id          INTEGER NOT NULL PRIMARY KEY,
                aid         INTEGER NOT NULL,
                type        INTEGER NOT NULL, 
                name        TEXT NOT NULL,
                uid         INTEGER,
                FOREIGN KEY(aid) REFERENCES accounts(id)
                FOREIGN KEY(uid) REFERENCES users(id)
            )";

        self.conn
            .execute(sql, ())
            .expect("Unable to initialize people table!");
        Ok(())
    }

    pub fn add_participant(&mut self, aid: u32, ptype: ParticipantType, name: String) -> Result<u32> {
        let id = self.get_next_people_id().unwrap();
        let p = rusqlite::params!(id, aid, ptype as u32, name);
        let sql = "INSERT INTO people (id, aid, type, name) VALUES (?1, ?2, ?3, ?4)";
        match self.conn.execute(sql, p) {
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

    // pub fn get_participants(&mut self, aid: u32, ptype: ParticipantType) -> Result<Vec<ParticipantRecord>, rusqlite::Error> {
    //     let sql;
    //     let p = rusqlite::params![aid, ptype as u32];
    //     sql = "SELECT id, name, type FROM people WHERE aid = (?1) and type = (?2)";

    //     let mut stmt = self.conn.prepare(sql)?;
    //     let exists = stmt.exists(p)?;
    //     let mut participants: Vec<ParticipantRecord> = Vec::new();
    //     match exists {
    //         true => {
    //             stmt = self.conn.prepare(sql)?;
    //             let party = stmt
    //                 .query_map(p, |row| Ok(
    //                     ParticipantRecord {
    //                         id : row.get(0)?,
    //                         participant : Participant {
    //                             name : row.get(1)?,
    //                             ptype : ParticipantType::from(row.get::<_, u32>(2)? as u32)
    //                         }
    //                     }))
    //                 .unwrap()
    //                 .collect::<Vec<_>>();
    //             for participant in party {
    //                 participants.push(participant.unwrap());
    //             }
    //         }
    //         false => {}
    //     }
    //     Ok(participants)
    // }

    pub fn get_participant_id(&mut self, aid : u32, name: String, ptype : ParticipantType) -> Option<u32> {
        let sql = "SELECT id FROM people WHERE aid = (?1) and name = (?2) and type = (?3)";
        let p = rusqlite::params![aid, name, ptype as u32];
        let conn = self.conn.clone();
        let prepared_stmt = conn.prepare(sql);
        match prepared_stmt {
            Ok(mut stmt) => {
                if let Ok(entry_found) = stmt.exists(p) {
                    if entry_found {
                        let id = stmt.query_row(p, |row: &rusqlite::Row<'_>| row.get::<_, u32>(0)).unwrap();
                        return Some(id);
                    } else { 
                        return None;
                    }
                } else {
                    panic!("Unable to determine if exists!");
                }
            }
            Err(e) => {
                panic!("SQLITE error {} while executing searching for person {}.", e.to_string(), name);
            }
        }
    }

    pub fn check_and_add_participant(&mut self, aid : u32, name: String, ptype : ParticipantType) -> u32 {
        let sql = "SELECT id FROM people WHERE aid = (?1) and name = (?2) and type = (?3)";
        let p = rusqlite::params![aid, name, ptype as u32];
        let conn = self.conn.clone();
        let prepared_stmt = conn.prepare(sql);
        match prepared_stmt {
            Ok(mut stmt) => {
                if let Ok(entry_found) = stmt.exists(p) {
                    if entry_found {
                        let id = stmt.query_row(p, |row: &rusqlite::Row<'_>| row.get::<_, u32>(0)).unwrap();
                        return id;
                    } else {
                        self.add_participant(aid, ptype, name).unwrap()
                    }
                } else {
                    panic!("Unable to determine if exists!");
                }
            }
            Err(e) => {
                panic!("SQLITE error {} while executing searching for person {}.", e.to_string(), name);
            }
        }
    }
}

#[derive(Clone)]
pub struct ParticipantAutoCompleter {
    pub aid : u32, 
    pub db : DbConn,
    pub ptype : ParticipantType
}

// impl Autocomplete for ParticipantAutoCompleter { 
//     fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
//         let suggestions : Vec<String>;
//         match self.ptype { 
//             ParticipantType::Payee => {
//                 suggestions = self.db.get_participants(self.aid, ParticipantType::Payee)
//                     .unwrap().into_iter()
//                     .map(|payee| payee.participant.name)
//                     .filter(|name| name.starts_with(input))
//                     .collect();
//             }
//             ParticipantType::Payer => {
//                 suggestions = self.db.get_participants(self.aid, ParticipantType::Payer)
//                     .unwrap().into_iter()
//                     .map(|payer| payer.participant.name)
//                     .filter(|name| name.starts_with(input))
//                     .collect();
//             }
//             _ => {
//                 panic!("Unable to match ParticipantType in Autocomplete!");
//             }
//         }
//         Ok(suggestions)
//     }

//     fn get_completion(
//             &mut self,
//             input: &str,
//             highlighted_suggestion: Option<String>,
//         ) -> Result<autocompletion::Replacement, CustomUserError> {

//         Ok ( match highlighted_suggestion { 
//             Some(suggestion) => {
//                 Replacement::Some(suggestion)
//             }
//             None => {
//                 let suggestions = self.get_suggestions(input).unwrap();
//                 if suggestions.len() == 0 {
//                     autocompletion::Replacement::None
//                 } else {
//                     Some(suggestions[0].clone())
//                 }
//             }
//         })
//     }

// }
