use chrono::NaiveDate;
use inquire::*;
use rusqlite::Result;
use crate::database::DbConn;
use inquire::autocompletion::Replacement;
use super::participants::ParticipantType;
use super::participants::ParticipantRecord;
use super::transfer_types::{self, TransferType};

pub struct LedgerEntry {
    pub date: String,
    pub amount: f32,
    pub transfer_type: TransferType,
    pub participant: u32,
    pub category_id: u32,
    pub description: String,
}

pub struct Ledger { 
    pub id : u32, 
    pub entry : LedgerEntry
}

impl DbConn {
    pub fn create_ledger_table(&mut self) -> Result<()> {
        let sql: &str;
        sql = "CREATE TABLE IF NOT EXISTS ledgers (
                id          INTEGER NOT NULL PRIMARY KEY,
                date        TEXT NOT NULL, 
                amount      REAL NOT NULL, 
                transfer_type INTEGER NOT NULL, 
                pid   INTEGER NOT NULL, 
                cid         INTEGER NOT NULL,
                desc        TEXT,
                aid         INTEGER NOT NULL,
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

    pub fn add_ledger_entry(&mut self, aid: u32, entry: LedgerEntry) -> rusqlite::Result<u32, rusqlite::Error> {
        let sql: &str;
        let id = self.get_next_ledger_id().unwrap();
        sql = "INSERT INTO ledgers ( id, date, amount, transfer_type, pid, cid, desc, aid) VALUES ( ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";
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
        Ok(id)
    }

    pub fn get_ledger_entries_within_timestamps(&mut self, aid : u32, start : NaiveDate, end : NaiveDate) -> rusqlite::Result<Vec<LedgerEntry>, rusqlite::Error> {
        let p = rusqlite::params![aid, start.to_string(), end.to_string()];
        let sql = "SELECT * FROM ledgers WHERE aid = (?1) and date >= (?2) and date <= (?3) ORDER by date ASC";

        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut entries: Vec<LedgerEntry> = Vec::new();
        match exists { 
            true => {
                let found_entries = stmt.query_map( p, |row| {
                    Ok( LedgerEntry { 
                        date : row.get(1)?, 
                        amount : row.get(2)?, 
                        transfer_type : TransferType::from(row.get::<_, u32>(3)? as u32),
                        participant : row.get(4)?,
                        category_id : row.get(5)?, 
                        description : row.get(6)?
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

    pub fn get_current_value(&mut self, aid : u32) -> rusqlite::Result<f32, rusqlite::Error> {
        let p = rusqlite::params![aid];
        let mut sum: f32 = 0.0;
        let sql = 
            "SELECT COALESCE(SUM(CASE
                WHEN transfer_type == 0 or transfer_type = 2 THEN -amount    -- withdrawal
                WHEN transfer_type == 1 or transfer_type = 3 THEN amount     -- deposit from external account
                ELSE 0 
            END), 0) as total_balance FROM ledgers WHERE aid = (?1);";

        let mut stmt = self.conn.prepare(sql)?;
        if stmt.exists(p)? { 
            sum = stmt.query_row(p, |row| row.get(0))?;
        } else { 
            panic!("Not found!");
        }

        Ok(sum)
    }

    pub fn get_cumulative_total_of_ledger_before_date(&mut self, aid : u32, end : NaiveDate) -> rusqlite::Result<f32, rusqlite::Error> {
        let p = rusqlite::params![aid, end.to_string()];
        let mut sum : f32 = 0.0;
        let sql = 
        "SELECT COALESCE(SUM(CASE
            WHEN transfer_type == 0 or transfer_type = 2 THEN -amount    -- withdrawal
            WHEN transfer_type == 1 or transfer_type = 3 THEN amount     -- deposit from external account
            ELSE 0 
        END), 0) as total_balance FROM ledgers WHERE aid = (?1) and date <= (?2);";

        let mut stmt = self.conn.prepare(sql)?;
        if stmt.exists(p)? { 
            sum = stmt.query_row(p, |row| row.get(0))?;
        } else { 
            panic!("Not found!");
        }
        Ok(sum)
    }

    pub fn get_participants(&mut self, aid: u32, transfer_type: TransferType) -> Result<Vec<String>, rusqlite::Error> {
        let sql;
        let p = rusqlite::params![aid, transfer_type as u32];
        sql = "SELECT p.name FROM people p JOIN ledgers l ON p.id = l.cid WHERE l.aid = (?1) and l.transfer_type = (?2)";

        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut participants: Vec<String> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let party = stmt
                    .query_map(p, |row| Ok(
                                row.get(0)?,
                        ))
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

#[derive(Clone)]
pub struct ParticipantAutoCompleter {
    pub aid : u32, 
    pub db : DbConn,
    pub ptype : ParticipantType
}

impl Autocomplete for ParticipantAutoCompleter { 
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        let mut suggestions : Vec<String>;
        match self.ptype { 
            ParticipantType::Payee => {
                let x: Vec<String> = self.db.get_participants(self.aid, TransferType::WidthdrawalToExternalAccount)
                    .unwrap().into_iter()
                    .filter(|name| name.starts_with(input))
                    .collect();
                let y: Vec<String> = self.db.get_participants(self.aid, TransferType::WidthdrawalToInternalAccount)
                    .unwrap().into_iter()
                    .filter(|name| name.starts_with(input))
                    .collect();
                suggestions = [x,y].concat();
            }
            ParticipantType::Payer => {
                let x: Vec<String> = self.db.get_participants(self.aid, TransferType::DepositFromExternalAccount)
                    .unwrap().into_iter()
                    .filter(|name| name.starts_with(input))
                    .collect();
                let y: Vec<String> = self.db.get_participants(self.aid, TransferType::DepositFromExternalAccount)
                    .unwrap().into_iter()
                    .filter(|name| name.starts_with(input))
                    .collect();
                suggestions = [x,y].concat();
            }
            ParticipantType::Both => {
                let w: Vec<String> = self.db.get_participants(self.aid, TransferType::WidthdrawalToExternalAccount)
                    .unwrap().into_iter()
                    .filter(|name| name.starts_with(input))
                    .collect();
                let x: Vec<String> = self.db.get_participants(self.aid, TransferType::WidthdrawalToInternalAccount)
                    .unwrap().into_iter()
                    .filter(|name| name.starts_with(input))
                    .collect();
                let y: Vec<String> = self.db.get_participants(self.aid, TransferType::DepositFromExternalAccount)
                    .unwrap().into_iter()
                    .filter(|name| name.starts_with(input))
                    .collect();
                let z: Vec<String> = self.db.get_participants(self.aid, TransferType::DepositFromExternalAccount)
                    .unwrap().into_iter()
                    .filter(|name| name.starts_with(input))
                    .collect();
                suggestions = [[w,x].concat(), [y,z].concat()].concat();
            }
            _ => {
                panic!("Unable to match ParticipantType in Autocomplete!");
            }
        }
        Ok(suggestions)
    }

    fn get_completion(
            &mut self,
            input: &str,
            highlighted_suggestion: Option<String>,
        ) -> Result<autocompletion::Replacement, CustomUserError> {

        Ok ( match highlighted_suggestion { 
            Some(suggestion) => {
                Replacement::Some(suggestion)
            }
            None => {
                let suggestions = self.get_suggestions(input).unwrap();
                if suggestions.len() == 0 {
                    autocompletion::Replacement::None
                } else {
                    Some(suggestions[0].clone())
                }
            }
        })
    }

}

// pub fn record_ledger_entry(_aid: u32, _db: &mut DbConn, action : Option<TransferType> ) -> LedgerEntry {
//     // this function returns either "Ok" or "Err". "Ok" indicates that the type T in Result<T, E>
//     // is okay to be used.
//     let date_input: Result<NaiveDate, InquireError> = DateSelect::new("Enter date").prompt();
//     let date: String = date_input.unwrap().to_string();

//     println!("Entered date is {0}", date);

//     let amount_input: Result<f32, InquireError> = CustomType::<f32>::new("Enter amount")
//         .with_placeholder("00000.00")
//         .with_default(00000.00)
//         .with_error_message("Please type a valid amount!")
//         .prompt();
//     let amount: f32 = amount_input.unwrap();

//     println!("Entered amount is {}", amount.to_string());

//     let transfer_type: TransferType;
//     let mut pid = 0;
//     let mut deposit_type : String;
//     let mut participant;

//     if action.is_none() {
//         let deposit_options: Vec<&str> = vec!["Widthdrawal", "Deposit"];
//         deposit_type = Select::new("Widthdrawal or deposit:", deposit_options)
//             .prompt()
//             .unwrap()
//             .to_string();
        
//         if deposit_type == "Widthdrawal" {
//             transfer_type = TransferType::WidthdrawalToExternalAccount;
//         } else {
//             transfer_type = TransferType::DepositFromExternalAccount;
//         }
//     } else {
//        transfer_type = action.unwrap();
//     }

//     // the match is equivalent to a switch statement
//     match transfer_type {
//         TransferType::WidthdrawalToExternalAccount => {
//             let mut participants = _db.get_participants(_aid, ParticipantType::participant).unwrap();
//             if participants.len() > 0 {
//                 participants.push("None".to_string());
//                 participants.push("New participant".to_string());
//                 participant = Select::new("Select participant:", participants)
//                     .prompt()
//                     .unwrap()
//                     .to_string();
//                 if participant == "New participant" {
//                     participant = Text::new("Enter participant:").prompt().unwrap().to_string();
//                     pid = _db.add_person(_aid, ParticipantType::participant, participant).unwrap();
//                 } else if participant == "None" {
//                     pid = _db.add_person(_aid, ParticipantType::participant, participant).unwrap();
//                 } else {
//                     pid = _db.get_person_id(_aid, participant).unwrap();
//                 }
//             } else {
//                 participant = Text::new("Enter participant:").prompt().unwrap().to_string();
//                 pid = _db.add_person(_aid, ParticipantType::participant, participant).unwrap();
//             }
//         }
//         TransferType::DepositFromExternalAccount => {
//             let mut participants = _db.get_participants(_aid, ParticipantType::participant).unwrap();
//             if participants.len() > 0 {
//                 participants.push("None".to_string());
//                 participants.push("New Payer".to_string());
//                 participant = Select::new("Select payer:", participants)
//                     .prompt()
//                     .unwrap()
//                     .to_string();
//                 if participant == "New Payer" {
//                     participant = Text::new("Enter payer:").prompt().unwrap().to_string();
//                     pid = _db.add_person(_aid, ParticipantType::Payer, participant).unwrap();
//                 } else if participant == "None" {
//                     pid = _db.add_person(_aid, ParticipantType::Payer, participant).unwrap();
//                 } else {
//                     pid = _db.get_person_id(_aid, participant).unwrap();
//                 }
//             } else {
//                 participant = Text::new("Enter payer:").prompt().unwrap().to_string();
//                 pid = _db.add_person(_aid, ParticipantType::Payer, participant).unwrap();
//             }
//         }
//         TransferType::WidthdrawalToInternalAccount => {
//             let mut participants = _db.get_participants(_aid, ParticipantType::participant).unwrap();
//             if participants.len() > 0 {
//                 participants.push("None".to_string());
//                 participants.push("New Beneficiary".to_string());
//                 participant = Select::new("Select withdrawal beneficiary:", participants)
//                     .prompt()
//                     .unwrap()
//                     .to_string();
//                 if participant == "New Beneficiary" {
//                     participant = Text::new("Enter withdrawal beneficiary:").prompt().unwrap().to_string();
//                     pid = _db.add_person(_aid, ParticipantType::participant, participant).unwrap();
//                 } else if participant == "None" {
//                     pid = _db.add_person(_aid, ParticipantType::participant, participant).unwrap();
//                 } else {
//                     pid = _db.get_person_id(_aid, participant).unwrap();
//                 }
//             } else {
//                 participant = Text::new("Enter withdrawal beneficiary:").prompt().unwrap().to_string();
//                 pid = _db.add_person(_aid, ParticipantType::participant, participant).unwrap();
//             } 
//         }
//         TransferType::WidthdrawalToInternalAccount => {
//             let mut participants = _db.get_participants(_aid, ParticipantType::participant).unwrap();
//             if participants.len() > 0 {
//                 participants.push("None".to_string());
//                 participants.push("New Source".to_string());
//                 participant = Select::new("Select deposit source:", participants)
//                     .prompt()
//                     .unwrap()
//                     .to_string();
//                 if participant == "New Source" {
//                     participant = Text::new("Enter deposit source:").prompt().unwrap().to_string();
//                     pid = _db.add_person(_aid, ParticipantType::participant, participant).unwrap();
//                 } else if participant == "None" {
//                     pid = _db.add_person(_aid, ParticipantType::participant, participant).unwrap();
//                 } else {
//                     pid = _db.get_person_id(_aid, participant).unwrap();
//                 }
//             } else {
//                 participant = Text::new("Enter deposit source:").prompt().unwrap().to_string();
//                 pid = _db.add_person(_aid, ParticipantType::participant, participant).unwrap();
//             } 
//         }
//         _ => {
//             panic!("Invalid entry.");
//         }
//     }

//     let mut categories = _db.get_categories(_aid).unwrap();
//     let mut category;
//     let mut cid = 0;
//     if categories.len() > 0 {
//         categories.push("None".to_string());
//         categories.push("New Category".to_string());
//         category = Select::new("Select category:", categories)
//             .prompt()
//             .unwrap()
//             .to_string();

//         if category == "New Category" {
//             category = Text::new("Enter payment category:")
//                 .prompt()
//                 .unwrap()
//                 .to_string();
//             cid = _db.add_category(_aid, category).unwrap();
//         } else if category == "None" {
//             cid = _db.add_category(_aid, category).unwrap();
//         } else {
//             cid = _db.get_category_id(_aid, category).unwrap();
//         }
//     } else {
//         category = Text::new("Enter payment category:")
//             .prompt()
//             .unwrap()
//             .to_string();
//         cid = _db.add_category(_aid, category).unwrap();
//     }

//     let description_input: String = Text::new("Enter payment description:")
//         .prompt()
//         .unwrap()
//         .to_string();

//     let entry = LedgerEntry {
//         date: date,
//         amount: amount,
//         transfer_type: transfer_type,
//         participant_id: pid,
//         category_id: cid,
//         description: description_input,
//     };

//     return entry;
// }
