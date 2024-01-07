use tokio;
use tokio::time::timeout;
use tokio_test;
use yahoo_finance_api as yahoo;
use yahoo::{YahooConnector, YahooError};

// #[cfg(not(feature = "blocking"))]
pub fn get_stock_at_close(ticker: String) -> Result<f64, YahooError> {
    let provider = YahooConnector::new();
    let rs = tokio_test::block_on(provider.get_latest_quotes(ticker.as_str(), "1d"))?;
    let quote = rs.last_quote()?;
    let close = quote.close;
    Ok(close)
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