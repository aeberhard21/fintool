use inquire::{autocompletion::{self, Replacement}, type_aliases::Suggester, Autocomplete, CustomUserError};
use rusqlite::{Error, Result};
use crate::{database::DbConn, types::ledger};

pub struct LabelInfo { 
    id : u32, 
    label : String
}

pub struct LabelMappingInfo { 
    pub id : u32, 
    pub ledger_id : u32, 
    pub label_id : u32,
}

impl DbConn { 
    pub fn create_labels_table(&self) -> rusqlite::Result<()> { 
        let sql = 
        "
            CREATE TABLE IF NOT EXISTS labels (
                id INTEGER NOT NULL,
                label TEXT NOT NULL, 
                uid INTEGER NOT NULL, 
                PRIMARY KEY (uid, id),
                FOREIGN KEY (uid) REFERENCES users(id) ON DELETE CASCADE ON UPDATE CASCADE
            )";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create labels table: {}", error)
            }
        }

        Ok(())
    }

    pub fn add_label(&self, uid : u32, label : String) -> Result<u32> { 
        let label_id = self.get_next_label_id(uid).unwrap();
        let p = rusqlite::params![uid, label_id, label];
        let sql = 
        "
            INSERT INTO labels (uid, id, label) VALUES (?1, ?2, ?3)
        ";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(id) => {
                Ok(label_id)
            }
            Err(error) => {
                panic!(
                    "Unable to add label {} for user {}: {}!",
                    &label, &uid, error
                );
            }
        }
    }

    pub fn check_and_add_label(&self, uid : u32, label : String) -> Result<u32> { 
        let p = rusqlite::params![uid, label];
        let sql = "SELECT id FROM labels WHERE uid = (?1) and label = (?2)";
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
                            return Ok(id);
                        } else {
                            // self.add_participant(uid, aid, ptype, name, is_account).unwrap()
                        }
                    } else {
                        panic!("Unable to determine if exists!");
                    }
                }
                Err(e) => {
                    panic!(
                        "SQLITE error {} while executing searching for label {}.",
                        e.to_string(),
                        label
                    );
                }
            }
        }
        std::mem::drop(conn_lock);
        let id = self.add_label(uid, label).unwrap();
        Ok(id)
    }

    pub fn remove_label(&self, uid : u32, label_id : u32) -> Result<u32> {
        let p = rusqlite::params![uid, label_id];
        let sql = "DELETE FROM labels WHERE uid = ?1 and id = ?2";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove label item: {}", error);
            }
        }
        let sql = "UPDATE labels SET id = id-1 WHERE id > ?2 and uid = ?1";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update label items: {}", error);
            }
        }

        let p = rusqlite::params![uid];
        let sql = "UPDATE account_ids SET next_label_id = next_label_id - 1 WHERE uid = ?1";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'label_id' value in 'account_ids': {}",
                    error
                );
            }
        }
        Ok(label_id)
    }

    pub fn update_label(&self, uid : u32, label_id : u32, new_label : String) -> Result<u32> { 
        let p = rusqlite::params![uid, label_id, new_label];
        let sql = "UPDATE labels SET label = (?3) WHERE uid = (?1) and id = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update label: {}", error);
            }
        }
        Ok(label_id)
    }

    pub fn get_labels(&self, uid : u32) -> Result<Vec<LabelInfo>, rusqlite::Error> {
        let p = rusqlite::params![uid];
        let sql = "SELECT * FROM labels WHERE uid = (?1)"; 
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut entries: Vec<LabelInfo> = Vec::new();
        match exists {
            true => {
                let found_entries = stmt.query_map(p, |row| {
                    Ok(LabelInfo { 
                        id : row.get(0)?, 
                        label : row.get(1)?
                    })
                }).unwrap().collect::<Vec<_>>();

                for entry in found_entries { 
                    entries.push(entry.unwrap());
                }
                return Ok(entries);
            }
            false => {
                return Ok(entries)
            }
        }
    }

    pub fn create_label_allocations_table(&self) -> rusqlite::Result<()> {
        let sql = 
        "
            CREATE TABLE IF NOT EXISTS label_allocations (
                id INTEGER NOT NULL,
                label_id INTEGER NOT NULL, 
                ledger_id INTEGER NOT NULL,
                uid INTEGER NOT NULL,
                aid INTEGER NOT NULL, 
                PRIMARY KEY (uid, aid, id),
                FOREIGN KEY (uid, label_id) REFERENCES labels (uid, id) ON DELETE CASCADE ON UPDATE CASCADE
                FOREIGN KEY (uid, aid, ledger_id) REFERENCES ledgers (uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE
                FOREIGN KEY (uid) REFERENCES users(id) ON DELETE CASCADE ON UPDATE CASCADE
        )";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create label allocations table: {}", error)
            }
        }

        Ok(())
    }

    pub fn add_label_mapping(&self, uid : u32, aid : u32, label_id : u32, ledger_id : u32) -> Result<u32, rusqlite::Error> { 
        let id = self.get_next_label_allocation_id(uid, aid).unwrap();
        let p = rusqlite::params![id, uid, aid, ledger_id, label_id];
        let sql = 
        "
            INSERT INTO label_allocations (id, uid, aid, ledger_id, label_id) VALUES (?1, ?2, ?3, ?4, ?5)
        ";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add label mapping between {} and {}:{}!",
                    label_id, aid, ledger_id
                );
            }
        }
    }

    pub fn check_and_get_label_mapping_matching_ledger_id(&self, uid : u32, aid : u32, ledger_id : u32) -> Result<Vec<LabelMappingInfo>, rusqlite::Error> { 
        let p = rusqlite::params![uid, aid, ledger_id];
        let sql = "SELECT id, ledger_id, label_id FROM label_allocations WHERE uid = (?1) and aid = (?2) and ledger_id = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut entries: Vec<LabelMappingInfo> = Vec::new();
        match exists {
            true => {
                let found_entries = stmt.query_map(p, |row| {
                    Ok(LabelMappingInfo { 
                        id : row.get(0)?, 
                        ledger_id : row.get(1)?, 
                        label_id : row.get(2)?
                    })
                }).unwrap().collect::<Vec<_>>();

                for entry in found_entries { 
                    entries.push(entry.unwrap());
                }
                return Ok(entries);
            }
            false => {
                return Ok(entries)
            }
        } 
    }

    pub fn remove_label_mapping(&self, uid : u32, aid : u32, id : u32) -> Result<u32, rusqlite::Error> { 
        let p = rusqlite::params![uid, aid, id];
        let sql = "DELETE FROM label_allocations WHERE uid = (?1) and aid = (?2) and id = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove label map: {}", error);
            }
        }
        let sql = "UPDATE label_allocations SET id = id-1 WHERE id > ?2 and uid = ?1";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update label items: {}", error);
            }
        }

        let p = rusqlite::params![uid];
        let sql = "UPDATE user_account_info SET label_allocation_id = label_allocation_id - 1 WHERE uid = ?1";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'label_allocation_id' value in 'user_account_info': {}",
                    error
                );
            }
        }
        Ok(id)
    }

}

#[derive(Clone)]
pub struct LabelAutoCompleter { 
    pub uid : u32,
    pub db : DbConn,
}

impl Autocomplete for LabelAutoCompleter { 
    fn get_suggestions(&mut self, input: &str) -> std::result::Result<Vec<String>, inquire::CustomUserError> {
        let mut suggestions: Vec<String>;
        suggestions = self.db.get_labels(self.uid)
            .unwrap()
            .into_iter()
            .map(|info| info.label)
            .filter(|label_name| label_name.starts_with(input.to_ascii_uppercase().as_str()))
            .collect();
        suggestions.dedup();
        Ok(suggestions)
    }

    fn get_completion(
            &mut self,
            input: &str,
            highlighted_suggestion: Option<String>,
        ) -> std::result::Result<inquire::autocompletion::Replacement, CustomUserError> {
            Ok(
                match highlighted_suggestion {
                    Some(suggestion) => Replacement::Some(suggestion),
                    None => { 
                        let suggestions = self.get_suggestions(input.to_ascii_uppercase().as_str()).unwrap();
                        if suggestions.len() == 0 { 
                            autocompletion::Replacement::None
                        } else { 
                            Some(suggestions[0].clone())
                        }
                    }
                }
            )
    }
}