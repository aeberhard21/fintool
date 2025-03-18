use chrono::NaiveDate;
use rusqlite::functions::FunctionFlags;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;

pub mod budget;
pub mod db_banks;
pub mod db_cd;
pub mod db_hsa;
mod db_user;

const CURRENT_DATABASE_SCHEMA_VERSION: i32 = 0;
pub const SQLITE_WILDCARD: &str = "%";

#[derive(Clone)]
pub struct DbConn {
    pub conn: Arc<Connection>,
}

impl DbConn {
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        // the ? returns early if error, otherwise ok
        let rs = Connection::open(db_path);
        let mut conn;
        match rs {
            Ok(rs_conn) => {
                conn = Self {
                    conn: Arc::new(rs_conn),
                };
                conn.initialize_database();
            }
            Err(error) => {
                panic!("unable to open db: {}", error)
            }
        };
        Ok(conn)
    }

    fn initialize_database(&mut self) -> Result<(), rusqlite::Error> {
        Self::allow_foreign_keys(&self.conn);
        Self::create_db_info_table(self).expect("unable to create!");
        Self::create_user_table(self);
        Self::create_accounts_table(self);
        Self::create_budget_categories_table(self);
        Self::create_people_table(self);
        Self::create_ledger_table(self);
        Self::create_bank_table(self);
        Self::create_investment_purchase_table(self);
        Self::create_investment_sale_table(self);
        Self::create_investment_sale_allocation_table(self);
        Self::create_cd_table(self);
        Self::create_budget_table(self);
        Self::create_account_transaction_table(self);
        Self::create_stock_split_table(self);
        Self::set_schema_version(&self.conn, CURRENT_DATABASE_SCHEMA_VERSION);

        // register custom functions
        let result = self
            .conn
            .create_scalar_function(
                "get_stock_value",
                1,
                FunctionFlags::SQLITE_INNOCUOUS,
                |ctx| {
                    let ticker: String = ctx.get(0)?;
                    match crate::stocks::get_stock_at_close(ticker) {
                        Ok(price) => Ok(price),
                        Err(e) => Err(rusqlite::Error::ToSqlConversionFailure(Box::new(e))),
                    }
                },
            )
            .unwrap();

        let result = self
            .conn
            .create_scalar_function(
                "get_stock_value_on_day",
                2,
                FunctionFlags::SQLITE_DETERMINISTIC,
                |ctx| {
                    let ticker: String = ctx.get(0)?;
                    let date: String = ctx.get(1)?;
                    match crate::stocks::get_stock_quote(
                        ticker,
                        NaiveDate::parse_from_str(date.as_str(), "%Y-%m-%d").unwrap(),
                    ) {
                        Ok(value) => Ok(value),
                        Err(e) => Err(rusqlite::Error::ToSqlConversionFailure(Box::new(e))),
                    }
                },
            )
            .unwrap();

        Ok(())
    }

    fn allow_foreign_keys(conn: &Connection) -> rusqlite::Result<()> {
        conn.pragma_update(None, "foreign_keys", "on")
    }

    fn get_schema_version(conn: &Connection) -> rusqlite::Result<i32> {
        conn.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))
    }

    fn set_schema_version(conn: &Connection, schema_version: i32) -> rusqlite::Result<()> {
        conn.pragma_update(None, "user_version", schema_version)
    }

    fn create_db_info_table(&mut self) -> rusqlite::Result<()> {
        let sql = "CREATE TABLE IF NOT EXISTS info (
            uid     INTEGER NOT NULL,
            aid     INTEGER NOT NULL,
            spid    INTEGER NOT NULL,
            ssid    INTEGER NOT NULL,
            said    INTEGER NOT NULL,
            cid     INTEGER NOT NULL,
            pid     INTEGER NOT NULL,
            bid     INTEGER NOT NULL, 
            lid     INTEGER NOT NULL, 
            tid     INTEGER NOT NULL,
            splid   INTEGER NOT NULL
        )";
        self.conn.execute(sql, ())?;
        let sql = "SELECT uid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        if !exists {
            let sql: &str =
                "INSERT INTO info (uid, aid, spid, ssid, said, cid, pid, bid, lid, tid, splid) VALUES ( ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)";
            match self.conn.execute(sql, (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)) {
                Ok(_rows_inserted) => {}
                Err(error) => {
                    panic!("Unable to initialize info table: {}", error);
                }
            }
        }
        Ok(())
    }

    pub fn get_next_user_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT uid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET uid = uid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next user ID within table 'info' does not exist.");
            }
        }
    }
    pub fn get_next_account_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT aid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET aid = aid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next account ID within table 'info' does not exist.");
            }
        }
    }

    pub fn get_next_stock_purchase_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT spid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET spid = spid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next stock purchase ID within table 'info' does not exist.");
            }
        }
    }

    pub fn get_next_stock_sale_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT ssid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET ssid = ssid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next stock sale ID within table 'info' does not exist.");
            }
        }
    }

    pub fn get_next_stock_sale_allocation_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT said FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET said = said + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next stock sale allocation ID within table 'info' does not exist.");
            }
        }
    }

    pub fn get_next_category_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT cid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET cid = cid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next category ID within table 'info' does not exist.");
            }
        }
    }

    pub fn get_next_people_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT pid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET pid = pid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next people ID within table 'info' does not exist.");
            }
        }
    }

    pub fn get_next_budget_item_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT bid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET bid = bid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next budget ID within table 'info' does not exist.");
            }
        }
    }

    pub fn get_next_ledger_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT lid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET lid = lid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next ledger ID within table 'info' does not exist.");
            }
        }
    }

    pub fn get_next_transaction_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT tid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET tid = tid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next transaction ID within table 'info' does not exist.");
            }
        }
    }

    pub fn get_next_stock_split_id(&mut self) -> rusqlite::Result<u32> {
        let sql = "SELECT splid FROM info";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE info SET splid = splid + 1";
                self.conn.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next stock split ID within table 'info' does not exist.");
            }
        }
    }

    // pub fn close(&mut self) {
    //     self.conn.
    //     self.conn.close().unwrap();
    // }
}
