use std::{ptr::null, result};

use chrono::format::StrftimeItems;
use rusqlite::{Connection, Error, params, types::Null};
use crate::{user::*, ledger};

use super::DbConn;

impl DbConn {
    pub fn create_user_table(&mut self) -> rusqlite::Result<()> {
        let sql : &str;
        sql = 
            "CREATE TABLE IF NOT EXISTS users (
                id          INTEGER NOT NULL, 
                name        TEXT    NOT NULL,
                ledgers     TEXT    NOT NULL,
                admin       BOOL    NOT NULL
            )";
        let rs = self.conn.execute(sql, ());
        match rs {
            Ok(0) => { println!("Table already created!");}
            Ok(_) => { println!("Created users table!");}
            Err(error) => {panic!("Unable to create users table: {}!", error);}
        }
        Ok(())
    }

    pub fn store_users(&mut self, users: &mut Vec<crate::user::User>) {
        let mut sql : &str; 
        let mut ledgers : String = String::new();
        println!("Number of users to store: {}.", users.len());
        for mut user in users {
            sql = "INSERT INTO users (id, name, ledgers, admin) VALUES ( ?1, ?2, ?3, ?4)";
            let mut user_ledgers = user.get_ledgers();
            if user_ledgers.len() > 0 {
                ledgers = format!("{}", user_ledgers[0]);
                user_ledgers.remove(0);
                for ledger in user_ledgers {
                    ledgers = format!("{}.{}", ledgers, ledger);
                }
            }
            let rs = self.conn.execute(sql, params![user.get_id(), user.get_name(), ledgers, user.get_admin()]);
            match rs {
                Ok(num_rows_updated) => {
                    println!("({}) Saved user: {}", num_rows_updated, user.get_name());
                }
                Err(error) => {
                    println!("Unable to add user {} because of: {}", user.get_name(), error);
                }
            }
        }
    }

    fn covert_string_to_ledgers(saved_string : String) -> Vec<String> {
        let mut ledgers : Vec<String> = Vec::new();
        let mut ledger : String = String::new();
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
        let sql : &str;
        sql = "SELECT * FROM users";
        let mut rs = self.conn.prepare(sql).unwrap();
        let results = rs.query_map(
            [], 
            |row| Ok(
                User {
                    id      : row.get(0)?,
                    name    : row.get(1)?,
                    ledgers : Self::covert_string_to_ledgers(row.get::<usize, String>(2).unwrap().to_string()),
                    is_admin: row.get(3)?,
                }
            )
        ).unwrap();

        let mut users: Vec<crate::user::User> = Vec::new();
        let mut i = 0;
        for result in results {
            let mut user = result.unwrap();
            let name = user.get_name();
            users.push(user);
            println!("Restored user:{}, {}", i,name);
            i+=1;
        }
        println!("The number of users restored: {}", users.len());
        return Ok(users)
    }



}