use tokio_test;
use yahoo::{YResponse, YahooConnector, YahooError};
use yahoo_finance_api as yahoo;
// use time::OffsetDateTime;

use crate::database::db_investments::StockRecord;

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

pub fn get_stock_growth_by_range(ticker: String, range: StockRange) -> Result<YResponse, YahooError> {
    let provider = YahooConnector::new().unwrap();
    let yrange: &str;
    let interval: &str;
    //https://cryptocointracker.com/yahoo-finance/yahoo-finance-api
    match range {
        StockRange::OneDay => {
            // interval = "1d";
            yrange = "1d";
        }
        StockRange::OneWeek => {
            // interval
            yrange = "1wk"
        }
        StockRange::OneMonth => {
            yrange = "1mo"
        }
        StockRange::ThreeMonth => {
            yrange = "3mo"
        }
        StockRange::SixMonth => {
            yrange = "6mo"
        }
        StockRange::OneYear => {
            yrange = "1y"
        }
        StockRange::TwoYear => {
            yrange = "2y"
        }
        StockRange::FiveYear => {
            yrange = "5y"
        }
        StockRange::All => {
            yrange = "10y"
        }
        _ => {
            yrange = "2mo"
        }
    }
    let rs = tokio_test::block_on(provider.get_quote_range(&ticker, "1d", yrange));
    return rs;
} 

// pub fn get_stock_growth_by_interval(ticker: String, start: &str, ) -> Result<YResponse, YahooError> {
//     let provider = YahooConnector::new();
//     let yrange: &str;
//     let interval: &str;
//     let rs = tokio_test::block_on(provider.get_quote_range(&ticker, "1d", yrange));
//     return rs;
// } 

pub fn get_stock_history(ticker: String, range: StockRange) -> Result<YResponse, YahooError> {
    let provider = YahooConnector::new().unwrap();
    let yrange: &str;
    let interval: &str;
    //https://cryptocointracker.com/yahoo-finance/yahoo-finance-api
    match range {
        StockRange::OneDay => {
            // interval = "1d";
            yrange = "1d";
        }
        StockRange::OneWeek => {
            // interval
            yrange = "1wk"
        }
        StockRange::OneMonth => {
            yrange = "1mo"
        }
        StockRange::ThreeMonth => {
            yrange = "3mo"
        }
        StockRange::SixMonth => {
            yrange = "6mo"
        }
        StockRange::OneYear => {
            yrange = "1y"
        }
        StockRange::TwoYear => {
            yrange = "2y"
        }
        StockRange::FiveYear => {
            yrange = "5y"
        }
        StockRange::All => {
            yrange = "10y"
        }
        _ => {
            yrange = "2mo"
        }
    }
    let rs = tokio_test::block_on(provider.get_quote_range(&ticker, "1d", yrange));
    return rs;
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
