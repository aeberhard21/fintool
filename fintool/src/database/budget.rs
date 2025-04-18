use super::DbConn;
use rusqlite::{Error, Result};

pub struct BudgetItem {
    pub category_id: u32,
    pub value: f32,
}

impl DbConn {
    pub fn create_budget_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS budgets (
            id          INTEGER NOT NULL PRIMARY KEY, 
            cid         INTEGER NOT NULL, 
            value       INTEGER NOT NULL,
            aid         INTEGER NOT NULL, 
            FOREIGN KEY(aid) references accounts(id)
            FOREIGN KEY(cid) references categories(id)
        )";
        self.conn
            .execute(sql, ())
            .expect("Unable to initialize budgets table!");
        Ok(())
    }

    pub fn add_budget_item(&mut self, aid: u32, item: BudgetItem) -> Result<u32> {
        let id = self.get_next_budget_item_id().unwrap();
        let p = rusqlite::params!(id, item.category_id, item.value, aid);
        let sql = "INSERT INTO budgets (id, cid, value, aid ) VALUES (?1, ?2, ?3, ?4)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add budget item {} for account {}: {}",
                    item.category_id, aid, error
                );
            }
        }
    }

    pub fn get_budget(&mut self, aid: u32) -> Result<Vec<BudgetItem>, rusqlite::Error> {
        let p = rusqlite::params![aid];
        let sql = "SELECT * FROM budgets WHERE aid = (?1)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut budget_items = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
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

    pub fn get_budget_categories(&mut self, uid: u32, aid: u32) -> Result<Vec<String>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT cid FROM budgets WHERE aid = (?1) and uid = (?2)";
        let mut cids;
        let mut categories = Vec::new();
        {
            let mut stmt = self.conn.prepare(sql)?;
            let exists = stmt.exists(p)?;
            match exists {
                true => {
                    stmt = self.conn.prepare(sql)?;
                    cids = stmt
                        .query_map(p, |row| Ok(row.get(0)?))
                        .unwrap()
                        .collect::<Vec<_>>();
                }
                false => {
                    panic!("A list of budget items doe not exist for account {}", aid);
                }
            }
        }

        for id in cids {
            categories.push(self.get_category_name(uid, aid, id.unwrap()).unwrap());
        }
        Ok(categories)
    }

    pub fn update_budget_item(
        &mut self,
        aid: u32,
        updated_item: BudgetItem,
    ) -> Result<(), rusqlite::Error> {
        let p = rusqlite::params![aid, updated_item.category_id, updated_item.value];
        let sql = "UPDATE budgets SET value = ?3 WHERE aid = (?1) and cid = (?2)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(()),
            Err(error) => {
                panic!(
                    "Unable to update budget item {} for account {}: {}",
                    updated_item.category_id, aid, error
                );
            }
        }
    }

    pub fn delete_budget_item(&mut self, aid: u32, cid: u32) -> Result<(), Error> {
        let p = rusqlite::params![aid, cid];
        let sql = "DELETE FROM budgets WHERE aid = ?1 AND cid = ?2";
        match self.conn.execute(sql, p) {
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
