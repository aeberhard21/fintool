use core::num;
use std::collections::hash_map::DefaultHasher;
use std::{ptr::null, result};
use std::hash::{Hash, Hasher};

use crate::{ledger, user::{*, self}};
use chrono::format::StrftimeItems;
use inquire::error;
use rusqlite::{params, types::Null, Connection, Error};

use std::cell::RefCell;
use std::rc::Rc;

use super::{DbConn, statements};

impl DbConn {
    pub fn create_user_table(&mut self) -> rusqlite::Result<()> {
        let sql: &str;
        sql = "CREATE TABLE IF NOT EXISTS users (
                id          INTEGER NOT NULL PRIMARY KEY, 
                name        TEXT    NOT NULL,
                admin       BOOL    NOT NULL
            )";
        let rs = self.conn.execute(sql, ());
        match rs {
            Ok(_) => {
                println!("Created users table!");
            }
            Err(error) => {
                panic!("Unable to create users table: {}!", error);
            }
        }
        Ok(())
    }

    pub fn add_user(&mut self, name: String, admin: bool) -> rusqlite::Result<u32, Error>{
        let sql: &str = "INSERT INTO users (id, name, admin) VALUES ( ?1, ?2, ?3)";
        
        // let mut s = DefaultHasher::new();
        // name.hash(&mut s);
        // let id: u32 = s.finish() as u32;
        let id = self.get_next_user_id().unwrap();

        let test_db: &str = "SELECT * FROM users where id = ?1";
        let mut stmt = self.conn.prepare(test_db);
        match stmt {
            Ok(mut stmt) => {
                let exists = stmt.exists(params![&id]);
                match exists {
                    Ok(true) => {
                        panic!("User already exists!");
                    }
                    Ok(false) => {
                        let number_of_rows_inserted = self.conn.execute(sql, params![id, name, admin]);
                        match number_of_rows_inserted {
                            Ok(rows_inserted) => {
                                println!("({}) Saved user: {}", rows_inserted, name);                
                                Ok(id)
                            }
                            Err(err) => {
                                panic!("Unable to add user {}: {}", &name, err);
                            }
                        }
                    }
                    Err(err) => {
                        panic!("Unable to add user {}: {}", &name, err);
                    }
                }
            }
            Err(err) => {
                panic!("Unable to add user {}: {}", &name, err);
            }
        }

    }

    pub fn get_users(&mut self) -> rusqlite::Result<Vec<String>, rusqlite::Error> {
        let sql: &str = "SELECT * FROM users";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        let mut users: Vec<String> = Vec::new();
        match exists {
            true => {
                let sql: &str = "SELECT name from users";
                let mut rs: rusqlite::Statement<'_> = self.conn.prepare(sql).unwrap();
                let names: Vec<Result<String, Error>> = rs.query_map([], |row|  {
                    Ok(row.get(0)?)
                }).unwrap().collect::<Vec<_>>();

                for name in names {
                    users.push(name.unwrap());
                }
                return Ok(users);
            }
            false => {
                return Ok(users);
            }
        }
    }

    pub fn get_user_id(&mut self, name: String) -> rusqlite::Result<u32, rusqlite::Error> {
        let sql: &str = "SELECT id FROM users WHERE name = (?1)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists((&name,))?;
        match exists {
            true => {
                let sql: &str = "SELECT id from users WHERE name = (?1)";
                let mut stmt = self.conn.prepare(sql)?;
                let id = stmt.query_row((&name,), |row| row.get::<_,u32>(0));
                match id {
                    Ok(id) => {
                        return Ok(id);
                    }
                    Err(err) => {
                        panic!("Unable t retrieve id for user {}: {}", &name, err);
                    }
                }
            } 
            false => {
                panic!("Unable to find user {}!", name);
            }
        }
    }

    pub fn is_admin(&mut self, uid: u32) -> rusqlite::Result<bool, Error> {
        let sql: &str = "SELECT admin FROM users WHERE id = (?1)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists((&uid,))?;
        match exists {
            true => {
                let admin = stmt.query_row((&uid,), |row| row.get::<_, bool>(0))?;
                Ok(admin)
            }
            false => {
                panic!("Unable to find user {}!", uid);
            }
        }
    }

    fn covert_string_to_ledgers(saved_string: String) -> Vec<String> {
        let mut ledgers: Vec<String> = Vec::new();
        let mut ledger: String = String::new();
        println!("String to restore: {}", saved_string);
        for c in saved_string.chars() {
            println!("Char = {}", c);
            if c == '.' {
                ledgers.push(ledger);
                ledger = String::new();
                continue;
            }
            ledger += &c.to_string();
        }
        ledgers.push(ledger);

        return ledgers;
    }

    pub fn restore_users(&mut self) -> Result<Vec<crate::user::User>, rusqlite::Error> {
        let sql: &str;
        sql = "SELECT * FROM users";
        let mut rs = self.conn.prepare(sql).unwrap();
        let mut results = rs
            .query_map([], |row| {
                Ok(User {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    ledgers: Self::covert_string_to_ledgers(
                        row.get::<usize, String>(2).unwrap().to_string(),
                    ),
                    is_admin: row.get(3)?,
                })
            })
            .unwrap().collect::<Vec<_>>();

        let mut users: Vec<crate::user::User> = Vec::new();
        let mut i = 0;
        // println!("Length before pushing: {}", Rc::new(results.count()));
        for result in results {
            let mut user = result.unwrap();
            let name = user.get_name();
            users.push(user);
            println!("Restored user:{}, {}", i, name);
            i += 1;
        }
        println!("The number of users restored: {}", users.len());
        return Ok(users);
    }
}
