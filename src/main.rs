use std::fs::{self};
use std::path::{Path, PathBuf};
use tokio;
use tokio::time::timeout;
use yahoo::YahooConnector;
use yahoo_finance_api as yahoo;

use crate::database::DbConn;
use crate::user::User;
// use crate::stocks::*;

mod database;
mod ledger;
mod stocks;
mod tui;
mod user;

fn main() {
    let stock_provider: YahooConnector = yahoo::YahooConnector::new();
    // let sp : &'static YahooConnector = &stock_provider;
    // get_quote(provider).await;

    let db_dir: String = String::from("./db");

    let mut _db: DbConn;
    match Path::new(&db_dir).try_exists() {
        Ok(true) => {}
        Ok(false) => {
            fs::create_dir(&db_dir);
        }
        Err(_) => {
            panic!("Unable to verify existence of database directory!");
        }
    }

    let mut db = PathBuf::new();
    db.push(&db_dir);
    db.push("finances.db");
    match Path::new(&db_dir).join(&db).try_exists() {
        Ok(_) => {
            // nothing to do
            _db = DbConn::new(db).unwrap();
            println!("Connect to db");
        }
        Err(_) => {
            panic!("Unable to verify existence of the database!");
        }
    }

    println!("Welcome to FinTool!");
    let mut _user: User;
    let next_id: u32 = 0;
    {
        tui::menu(&mut _db);
    }
    _db.close();
}

async fn get_quote(provider: YahooConnector) {
    // let provider = yahoo::YahooConnector::new();
    // let quote = tokio::spawn(provider.get_latest_quotes("SOXX", "1d"));
    // let tmp = quote.await.unwrap().unwrap();
    // println!("Stock is: {}", tmp.last_quote().unwrap().adjclose);
    let result = timeout(
        std::time::Duration::from_secs(5),
        provider.get_latest_quotes("SOXX", "1d"),
    )
    .await;
    match result {
        Ok(Ok(tmp)) => {
            println!("Stock is: {}", tmp.last_quote().unwrap().adjclose);
        }
        Ok(Err(err)) => {
            eprintln!("Error fetching quote: {:?}", err);
        }
        Err(_) => {
            eprintln!("Timeout occurred");
        }
    }
}

// async fn fetch_quote(mut provider: YahooConnector) -> Result<yahoo::Quote, YahooError> {
//     provider.timeout(tokio::time::Duration::from_secs(5));

// }
