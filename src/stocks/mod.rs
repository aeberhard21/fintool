use chrono::{FixedOffset, NaiveDate, NaiveTime};
use time::OffsetDateTime;
use tokio_test;
use yahoo::{YResponse, YahooConnector, YahooError};
use yahoo_finance_api::{self as yahoo, Quote};
// use time::OffsetDateTime;

use crate::types::investments::StockRecord;

pub enum StockRange { 
    OneDay,
    OneWeek,
    OneMonth, 
    ThreeMonth,
    SixMonth, 
    OneYear,
    TwoYear,
    FiveYear,
    All
}

// #[cfg(not(feature = "blocking"))]
pub fn get_stock_at_close(ticker: String) -> Result<f64, YahooError> {
    let provider = YahooConnector::new().unwrap();
    let rs = tokio_test::block_on(provider.get_latest_quotes(ticker.as_str(), "1d"))?;
    let quote = rs.last_quote()?;
    let close = quote.close;
    Ok(close)
}

pub fn get_stock_history(ticker: String, period_start : NaiveDate, period_end : NaiveDate) -> Result<Vec<Quote>, YahooError> {
    let provider = YahooConnector::new().unwrap();

    let start  = OffsetDateTime::from_unix_timestamp(
        period_start
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .and_utc()
        .timestamp())
        .unwrap();

    let end = OffsetDateTime::from_unix_timestamp(
        period_end
        .and_time(NaiveTime::from_hms_opt(0,0,0).unwrap())
        .and_utc()
        .timestamp())
        .unwrap();

    let rs = tokio_test::block_on(provider.get_quote_history(&ticker, start, end)).unwrap();
    return rs.quotes();
} 

pub fn return_stock_values(stocks: Vec<StockRecord>) -> f64 {
    let mut value: f64 = 0.0;
    for s in stocks {
        value += get_stock_at_close(s.ticker).unwrap() * s.shares as f64;
    }
    return value;
}

// pub async fn get_stock_at_date(provider: yahoo::YahooConnector, ticker: &str) -> f32 {
//     let result = timeout(std::time::Duration::from_secs(5), provider.get_quote_history() ).await;
//     match result {
//         Ok(Ok(resp)) => {
//             return resp.last_quote().unwrap().adjclose as f32;
//         }
//         Ok(Err(error)) => {
//             panic!("Error fetching quote: {:?}", error);
//         }
//         Err(_) => {
//             panic!("Timeout occurred!");
//         }
//     }
//}
