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
    pub fn create_budget_categories_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS categories ( 
                id          INTEGER NOT NULL PRIMARY KEY,
                aid         INTEGER NOT NULL,
                category    TEXT NOT NULL,
                FOREIGN KEY(aid) REFERENCES accounts(id)
            )";

        self.conn
            .execute(sql, ())
            .expect("Unable to initialize categories table!");
        Ok(())
    }

    pub fn add_category(&mut self, aid: u32, category: String) -> Result<u32> {
        let id = self.get_next_category_id().unwrap();
        let p = rusqlite::params!(id, aid, category);
        let sql = "INSERT INTO categories (id, aid, category) VALUES (?1, ?2, ?3)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!("Unable to add {} for account {}: {}", category, aid, error);
            }
        }
    }

    pub fn get_categories(&mut self, aid: u32) -> Result<Vec<CategoryRecord>, rusqlite::Error> {
        let p = rusqlite::params![aid];
        let sql = "SELECT id, category FROM categories WHERE aid = (?1)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut categories: Vec<CategoryRecord> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
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
        &mut self,
        aid: u32,
        cid: u32,
    ) -> rusqlite::Result<String, rusqlite::Error> {
        let sql: &str = "SELECT category FROM categories WHERE aid = (?1) AND id = (?2)";
        let p = rusqlite::params![aid, cid];
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
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
        &mut self,
        aid: u32,
        category: String,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let sql: &str = "SELECT id FROM categories WHERE aid = (?1) AND category = (?2)";
        let p = rusqlite::params![aid, category];
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
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

    pub fn check_and_add_category(&mut self, aid: u32, name: String) -> u32 {
        let sql = "SELECT id FROM categories WHERE aid = (?1) and category = (?2)";
        let conn = self.conn.clone();
        let p = rusqlite::params![aid, name];
        let prepared_stmt = conn.prepare(sql);
        match prepared_stmt {
            Ok(mut stmt) => {
                if let Ok(entry_found) = stmt.exists(p) {
                    if entry_found {
                        if let id =
                            stmt.query_row(p, |row: &rusqlite::Row<'_>| row.get::<_, u32>(0))
                        {
                            return id.unwrap();
                        } else {
                            panic!("Unable to query row!");
                        }
                    } else {
                        self.add_category(aid, name).unwrap()
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
}

#[derive(Clone)]
pub struct CategoryAutoCompleter {
    pub aid: u32,
    pub db: DbConn,
}

impl Autocomplete for CategoryAutoCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        let mut suggestions: Vec<String>;
        suggestions = self
            .db
            .get_categories(self.aid)
            .unwrap()
            .into_iter()
            .map(|category| category.category.name)
            .filter(|cname| cname.starts_with(input))
            .collect();
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
