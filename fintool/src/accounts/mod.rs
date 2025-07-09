pub mod bank_account;
pub mod base;
pub mod certificate_of_deposit;
pub mod credit_card_account;
pub mod investment_account_manager;
pub mod wallet;

pub fn float_range(start: f64, end: f64, step: f64) -> Vec<f64> {
    let mut vec = Vec::new();
    let mut current = start;
    while current <= end {
        vec.push(current);
        current += step;
    }
    vec
}
