use chrono::{Datelike, Days, Duration, NaiveDate, NaiveTime, Weekday};
use time::OffsetDateTime;
use tokio_test;
use yahoo::{YahooConnector, YahooError};
use yahoo_finance_api::{self as yahoo, Quote};
use std::time::Instant;

use crate::types::investments::StockInfo;

// #[cfg(not(feature = "blocking"))]
pub fn get_stock_at_close(ticker: String) -> Result<f64, YahooError> {
    let provider = YahooConnector::new().unwrap();
    let rs = tokio_test::block_on(provider.get_latest_quotes(ticker.as_str(), "1d"))?;
    let quote = rs.last_quote()?;
    let close = quote.close;
    Ok(close)
}

pub fn get_stock_history(
    ticker: String,
    period_start: NaiveDate,
    period_end: NaiveDate,
) -> Result<Vec<Quote>, YahooError> {
    let provider = YahooConnector::new().unwrap();

    let start = OffsetDateTime::from_unix_timestamp(
        period_start
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .and_utc()
            .timestamp(),
    )
    .unwrap();

    let end = OffsetDateTime::from_unix_timestamp(
        period_end
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .and_utc()
            .timestamp(),
    )
    .unwrap();

    // let starter = Instant::now();
    let rs = tokio_test::block_on(provider.get_quote_history(&ticker, start, end))?;
    // let duration = starter.elapsed();
    // println!("Duration is {:?}!", duration);
    return rs.quotes();
}

pub fn get_stock_quote(ticker: String, date: NaiveDate) -> Result<f64, YahooError> {
    // to get stock quote, we need a start and end date. The "start" date will be
    // the returned quoted by the call to the function.

    // However, we first need to validate the provided date to ensure that the
    // stock exhange was open that day, i.e., not a weekend or a recognized holiday.
    let mut start_date = date;
    // let mut new_date;
    loop {
        start_date = match start_date.weekday() {
            chrono::Weekday::Sat => start_date
                .checked_sub_days(Days::new(1))
                .expect("Saturday date out of range!"), // this is a Friday
            chrono::Weekday::Sun => start_date
                .checked_sub_days(Days::new(2))
                .expect("Sunday date out of range!"), // this is a Friday
            _ => start_date,
        };

        if check_if_holiday(start_date) == false {
            break;
        } else {
            start_date = start_date
                .checked_sub_days(Days::new(1))
                .expect("Invalid date!");
        }
    }
    // should return 1 quote only, the day requested. End date is not inclusive (despite what documentation states)
    let quote = get_stock_history(
        ticker,
        start_date,
        start_date
            .checked_add_days(Days::new(1))
            .expect("Invalid day!"),
    )
    .unwrap()
    .get(0)
    .expect("Quote not found!")
    .to_owned();
    Ok(quote.close)
}

pub fn check_if_holiday(date: NaiveDate) -> bool {
    let may_31st: NaiveDate =
        NaiveDate::from_ymd_opt(date.year(), 5, 31).expect("May 31st date invalid!");
    let memorial_day = match may_31st.weekday() {
        Weekday::Tue => may_31st
            .checked_sub_days(Days::new(1))
            .expect("Invalid Tuesday date!"),
        Weekday::Wed => may_31st
            .checked_sub_days(Days::new(2))
            .expect("Invalid Wednesday date!"),
        Weekday::Thu => may_31st
            .checked_sub_days(Days::new(3))
            .expect("Invalid Thursday date!"),
        Weekday::Fri => may_31st
            .checked_sub_days(Days::new(4))
            .expect("Invalid Friday date!"),
        Weekday::Sat => may_31st
            .checked_sub_days(Days::new(5))
            .expect("Invalid Saturday date!"),
        Weekday::Sun => may_31st
            .checked_sub_days(Days::new(6))
            .expect("Invalid Sunday date!"),
        _ => may_31st,
    };

    // Check if holiday
    let holidays = vec![
        NaiveDate::from_ymd_opt(date.year(), 1, 1).expect("New Year's not valid!"),
        NaiveDate::from_weekday_of_month_opt(date.year(), 1, Weekday::Mon, 3)
            .expect("MLK Jr. day not valid!"),
        NaiveDate::from_weekday_of_month_opt(date.year(), 2, Weekday::Mon, 3)
            .expect("President's day not valid!"),
        memorial_day,
        NaiveDate::from_ymd_opt(date.year(), 6, 19).expect("Juneteenth not valid!"),
        NaiveDate::from_ymd_opt(date.year(), 7, 4).expect("July 4th not valid!"),
        NaiveDate::from_weekday_of_month_opt(date.year(), 9, Weekday::Mon, 1)
            .expect("Labor day not valid!"),
        NaiveDate::from_weekday_of_month_opt(date.year(), 11, Weekday::Thu, 4)
            .expect("Thanksgiving not valid!"),
        NaiveDate::from_ymd_opt(date.year(), 12, 25).expect("Christmas not valid!"),
    ];

    return holidays.contains(&date);
}
