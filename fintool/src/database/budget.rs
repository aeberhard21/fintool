use rusqlite::{Error, Result};
use super::DbConn;

pub struct BudgetItem {
    pub category_id: u32,
    pub value: f32,
}

impl DbConn {
    pub fn create_budget_table(&self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS budgets (
            id          INTEGER NOT NULL, 
            cid         INTEGER NOT NULL, 
            value       INTEGER NOT NULL,
            aid         INTEGER NOT NULL, 
            uid         INTEGER NOT NULL,
            PRIMARY KEY(uid, aid, id),
            FOREIGN KEY(aid,uid) references accounts(id,uid) ON DELETE CASCADE ON UPDATE CASCADE
            FOREIGN KEY(cid,uid,aid) references categories(id, uid, aid) ON DELETE CASCADE ON UPDATE CASCADE
        )";
        self.conn.lock().unwrap()
            .execute(sql, ())
            .expect("Unable to initialize budgets table!");
        Ok(())
    }

    // pub fn add_budget_item(&self, aid: u32, item: BudgetItem) -> Result<u32> {
    //     let id = self.get_next_budget_item_id(aid).unwrap();
    //     let p = rusqlite::params!(id, item.category_id, item.value, aid);
    //     let sql = "INSERT INTO budgets (id, cid, value, aid ) VALUES (?1, ?2, ?3, ?4)";
    //     match conn_lock.execute(sql, p) {
    //         Ok(_) => Ok(id),
    //         Err(error) => {
    //             panic!(
    //                 "Unable to add budget item {} for account {}: {}",
    //                 item.category_id, aid, error
    //             );
    //         }
    //     }
    // }

    pub fn get_budget(&self, aid: u32) -> Result<Vec<BudgetItem>, rusqlite::Error> {
        let p = rusqlite::params![aid];
        let sql = "SELECT * FROM budgets WHERE aid = (?1)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut budget_items = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let items: Vec<Result<BudgetItem, Error>> = stmt
                    .query_map(p, |row| {
                        Ok(BudgetItem {
                            category_id: row.get(0)?,
                            value: row.get(1)?,
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();
                for item in items {
                    budget_items.push(item.unwrap());
                }
                Ok(budget_items)
            }
            false => {
                panic!("A list of budget items doe not exist for account {}", aid);
            }
        }
    }

    pub fn get_budget_categories(&self, uid: u32, aid: u32) -> Result<Option<Vec<String>>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT c.category FROM budgets b INNER JOIN categories c ON b.cid = c.id WHERE aid = (?1) and uid = (?2)";
        let mut categories_wrapped: Vec<Result<String>>;
        let mut categories = Vec::new();
        {
            let conn_lock = self.conn.lock().unwrap();
            let mut stmt = conn_lock.prepare(sql)?;
            let exists = stmt.exists(p)?;
            match exists {
                true => {
                    stmt = conn_lock.prepare(sql)?;
                    categories_wrapped = stmt
                        .query_map(p, |row| Ok(row.get(0)?))
                        .unwrap()
                        .collect::<Vec<_>>();
                }
                false => {
                    return Ok(None)
                }
            }
        }

        for category_wrapped in categories_wrapped { 
            categories.push(category_wrapped.unwrap())
        }

        Ok(Some(categories))
    }

    pub fn update_budget_item(
        &self,
        aid: u32,
        updated_item: BudgetItem,
    ) -> Result<(), rusqlite::Error> {
        let p = rusqlite::params![aid, updated_item.category_id, updated_item.value];
        let sql = "UPDATE budgets SET value = ?3 WHERE aid = (?1) and cid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(()),
            Err(error) => {
                panic!(
                    "Unable to update budget item {} for account {}: {}",
                    updated_item.category_id, aid, error
                );
            }
        }
    }

    pub fn delete_budget_item(&self, aid: u32, cid: u32) -> Result<(), Error> {
        let p = rusqlite::params![aid, cid];
        let sql = "DELETE FROM budgets WHERE aid = ?1 AND cid = ?2";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(()),
            Err(error) => {
                panic!(
                    "Unable to update budget item {} for account {}: {}",
                    cid, aid, error
                );
            }
        }
    }
}
