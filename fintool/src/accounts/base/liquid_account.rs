use chrono::NaiveDate;

pub trait LiquidAccount {
    fn get_positive_cash_flow(&self, start: NaiveDate, end: NaiveDate) -> f32;
    fn get_negative_cash_flow(&self, start: NaiveDate, end: NaiveDate) -> f32;
    fn get_cash_flow(&self, start: NaiveDate, end: NaiveDate) -> f32;
}
