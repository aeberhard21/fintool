use super::DbConn;
use rusqlite::{Error, Result};

#[derive(Debug, Clone)]
pub struct BudgetItem {
    pub category_id: u32,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub struct BudgetRecord { 
    pub id : u32, 
    pub item : BudgetItem
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
        self.conn
            .lock()
            .unwrap()
            .execute(sql, ())
            .expect("Unable to initialize budgets table!");
        Ok(())
    }

    pub fn add_budget_item(&self, uid : u32, aid: u32, item: BudgetItem) -> Result<u32> {
        let id = self.get_next_budget_item_id(uid, aid).unwrap();
        let p = rusqlite::params!(id, item.category_id, item.value, aid, uid);
        let sql = "INSERT INTO budgets (id, cid, value, aid, uid ) VALUES (?1, ?2, ?3, ?4, ?5)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add budget item {} for account {}: {}",
                    item.category_id, aid, error
                );
            }
        }
    }

    pub fn get_budget(&self, uid: u32, aid: u32) -> Result<Option<Vec<BudgetRecord>>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT * FROM budgets WHERE aid = (?1) and uid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut budget_items = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let items: Vec<Result<BudgetRecord, Error>> = stmt
                    .query_map(p, |row| {
                        Ok(BudgetRecord{
                            id : row.get(0)?, 
                            item : BudgetItem {
                            category_id: row.get(0)?,
                            value: row.get(1)?,
                        }})
                    })
                    .unwrap()
                    .collect::<Vec<_>>();
                for item in items {
                    budget_items.push(item.unwrap());
                }
                Ok(Some(budget_items))
            }
            false => {
                Ok(None)
            }
        }
    }

    pub fn get_budget_categories(
        &self,
        uid: u32,
        aid: u32,
    ) -> Result<Option<Vec<String>>, rusqlite::Error> {
        let p = rusqlite::params![aid, uid];
        let sql = "SELECT c.category FROM budgets b INNER JOIN categories c ON b.cid = c.id WHERE c.aid = (?1) and c.uid = (?2)";
        let categories_wrapped: Vec<Result<String>>;
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
                false => return Ok(None),
            }
        }

        for category_wrapped in categories_wrapped {
            categories.push(category_wrapped.unwrap())
        }

        Ok(Some(categories))
    }

    pub fn update_budget_item(
        &self,
        uid: u32,
        aid: u32,
        updated_item: BudgetRecord,
    ) -> Result<(), rusqlite::Error> {
        let p = rusqlite::params![aid, uid, updated_item.id, updated_item.item.category_id, updated_item.item.value];
        let sql = "UPDATE budgets SET cid = ?4, value = ?5 WHERE aid = (?1) and id = (?3) and uid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => Ok(()),
            Err(error) => {
                panic!(
                    "Unable to update budget item {} for account {}: {}",
                    updated_item.id, aid, error
                );
            }
        }
    }

    pub fn remove_budget_item(&self, uid: u32, aid: u32, cid: u32) -> Result<(), Error> {
        let p = rusqlite::params![uid, aid, cid];
        let sql = "DELETE FROM budgets WHERE uid = ?1 and aid = ?2 AND cid = ?3";
        let conn_lock = self.conn.lock().unwrap();
        match conn_lock.execute(sql, p) {
            Ok(_) => {},
            Err(error) => {
                panic!(
                    "Unable to update budget item {} for account {}: {}",
                    cid, aid, error
                );
            }
        }
        let sql = "UPDATE budgets SET id = id-1 WHERE id > ?3 and uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove ledger item: {}", error);
            }
        }

        let p = rusqlite::params![uid, aid];
        let sql = "UPDATE user_account_info SET bid = bid - 1 WHERE uid = ?1 and aid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'bid' value in 'user_account_info': {}",
                    error
                );
            }
        }
        Ok(())
    }
}
