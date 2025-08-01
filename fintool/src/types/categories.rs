use inquire::autocompletion;
use inquire::autocompletion::*;
use inquire::Autocomplete;
use inquire::CustomUserError;
use rusqlite::Result;

use crate::database::DbConn;

#[derive(Clone)]
pub struct Category {
    pub name: String,
}

#[derive(Clone)]
pub struct CategoryRecord {
    pub id: u32,
    pub category: Category,
}

impl DbConn {
    pub fn create_budget_categories_table(&self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS categories ( 
                id          INTEGER NOT NULL,
                aid         INTEGER NOT NULL,
                uid         INTEGER NOT NULL, 
                category    TEXT NOT NULL,
                PRIMARY KEY (uid, aid, id),
                FOREIGN KEY(uid,aid) REFERENCES accounts(uid,id) ON DELETE CASCADE ON UPDATE CASCADE,
                FOREIGN KEY(uid) REFERENCES users(id)
            )";
        let conn_lock = self.conn.lock().unwrap();
        conn_lock
            .execute(sql, ())
            .expect("Unable to initialize categories table!");
        Ok(())
    }

    pub fn add_category(&self, uid: u32, aid: u32, category: String) -> Result<u32> {
        let id = self.get_next_category_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, aid, category, uid);
        let sql = "INSERT INTO categories (id, aid, category, uid) VALUES (?1, ?2, ?3, ?4)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add {} for account {}: {}", category, aid, error);
            }
        }
    }

    pub fn update_category_name(
        &self,
        uid: u32,
        aid: u32,
        old: String,
        new: String,
    ) -> Result<String> {
        let p = rusqlite::params!(uid, aid, old, new);
        let sql =
            "UPDATE categories SET name = (?4) WHERE uid = (?1) and aid = (?2) and name = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(new),
            Err(error) => {
                panic!(
                    "Unable to update category {} in account {}: {}!",
                    old, aid, error
                );
            }
        }
    }

    pub fn get_categories(
        &self,
        uid: u32,
        aid: u32,
    ) -> Result<Vec<CategoryRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT id, category FROM categories WHERE aid = (?1) and uid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut categories: Vec<CategoryRecord> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let cats = stmt
                    .query_map(p, |row| {
                        Ok(CategoryRecord {
                            id: row.get(0)?,
                            category: Category { name: row.get(1)? },
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();
                for cat in cats {
                    categories.push(cat.unwrap());
                }
            }
            false => {}
        }
        Ok(categories)
    }

    pub fn get_category_name(
        &self,
        uid: u32,
        aid: u32,
        cid: u32,
    ) -> rusqlite::Result<String, rusqlite::Error> {
        let sql: &str =
            "SELECT category FROM categories WHERE aid = (?1) AND id = (?2) and uid = (?3)";
        let p = rusqlite::params![aid, cid, uid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let name = stmt.query_row(p, |row| row.get::<_, String>(0));
                match name {
                    Ok(name) => {
                        return Ok(name);
                    }
                    Err(err) => {
                        panic!("Unable to retrieve id for account {}: {}", aid, err);
                    }
                }
            }
            false => {
                panic!("Unable to find account matching {}", aid);
            }
        }
    }

    pub fn get_category_id(
        &self,
        aid: u32,
        category: String,
        uid: u32,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let sql: &str =
            "SELECT id FROM categories WHERE aid = (?1) AND category = (?2) and uid = (?3)";
        let p = rusqlite::params![aid, category, uid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0));
                match id {
                    Ok(id) => {
                        return Ok(id);
                    }
                    Err(err) => {
                        panic!("Unable to retrieve id for account {}: {}", aid, err);
                    }
                }
            }
            false => {
                panic!("Unable to find account matching {}", aid);
            }
        }
    }

    pub fn check_and_add_category(&self, uid: u32, aid: u32, name: String) -> u32 {
        let sql = "SELECT id FROM categories WHERE aid = (?1) and category = (?2) and uid = (?3)";
        let cloned_conn = self.conn.clone();
        let conn_lock = cloned_conn.lock().unwrap();
        let p = rusqlite::params![aid, name, uid];
        {
            let prepared_stmt = conn_lock.prepare(sql);
            match prepared_stmt {
                Ok(mut stmt) => {
                    if let Ok(entry_found) = stmt.exists(p) {
                        if entry_found {
                            let id =
                                stmt.query_row(p, |row: &rusqlite::Row<'_>| row.get::<_, u32>(0));
                            return id.unwrap();
                            // {
                            //     return id.unwrap();
                            // } else {
                            //     panic!("Unable to query row!");
                            // }
                        } else {
                            //
                            // self.add_category(uid, aid, name).unwrap()
                        }
                    } else {
                        panic!("Unable to determine if exists!");
                    }
                }
                Err(e) => {
                    panic!(
                        "SQLITE error {} while executing searching for category {}.",
                        e.to_string(),
                        name
                    );
                }
            }
        }
        std::mem::drop(conn_lock);
        self.add_category(uid, aid, name).unwrap()
    }

    pub fn remove_category(
        &self,
        uid: u32,
        aid: u32,
        name: String,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let id = self.get_category_id(aid, name.clone(), uid).unwrap();
        let p = rusqlite::params![uid, aid, name];
        let sql = "DELETE FROM categories WHERE uid = (?1) and aid = (?2) and category = (?3)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!("Unable to remove category item: {}!", error);
            }
        }
        let p = rusqlite::params![id, uid, aid];
        let sql = "UPDATE categories SET id = id-1 WHERE id > ?1 and uid = ?2 and aid = ?3";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!("Unable to update category ids: {}!", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET cid = cid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!(
                    "Unable to update 'cid' value in 'user_account_info': {}!",
                    error
                );
            }
        }

        Ok(id)
    }
}

#[derive(Clone)]
pub struct CategoryAutoCompleter {
    pub aid: u32,
    pub uid: u32,
    pub db: DbConn,
    pub cats : Option<Vec<String>>
}

impl Autocomplete for CategoryAutoCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        let mut suggestions: Vec<String>;
        suggestions = if let Some(categories) = self.cats.clone() {
            categories
            .into_iter()
            .filter(|name| name.starts_with(input.to_ascii_uppercase().as_str()))
            .collect::<Vec<String>>()
        } else {
            self
            .db
            .get_categories(self.uid, self.aid)
            .unwrap()
            .into_iter()
            .map(|category| category.category.name)
            .filter(|cname| cname.starts_with(input.to_ascii_uppercase().as_str()))
            .collect()
        };
        suggestions.dedup();
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
                let suggestions = self
                    .get_suggestions(input.to_ascii_uppercase().as_str())
                    .unwrap();
                if suggestions.len() == 0 {
                    autocompletion::Replacement::None
                } else {
                    Some(suggestions[0].clone())
                }
            }
        })
    }
}
