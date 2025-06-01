use chrono::NaiveDate;
use rusqlite::functions::FunctionFlags;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;

pub mod budget;
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
        Self::create_user_account_info_table(self);
        Self::create_accounts_id_table(self);
        Self::create_users_id_table(self);
        Self::create_user_table(self);
        Self::create_accounts_table(self);
        Self::create_budget_categories_table(self);
        Self::create_people_table(self);
        Self::create_ledger_table(self);
        Self::create_investment_purchase_table(self);
        Self::create_investment_sale_table(self);
        Self::create_investment_sale_allocation_table(self);
        Self::create_cd_table(self);
        Self::create_budget_table(self);
        Self::create_account_transaction_table(self);
        Self::create_stock_split_table(self);
        Self::create_stock_split_allocation_table(self);
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

}
