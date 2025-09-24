pub mod bank_account;
pub mod base;
pub mod certificate_of_deposit;
pub mod credit_card_account;
pub mod investment_account_manager;
pub mod health_savings_account;
pub mod roth_ira;
pub mod wallet;
pub mod retirement_401k_plan;

use crate::accounts::bank_account::BankAccount;
use crate::accounts::base::Account;
use crate::accounts::base::liquid_account::LiquidAccount;
use crate::accounts::wallet::Wallet;

#[cfg(feature = "ratatui_support")]
pub fn as_liquid_account(account: &dyn Account) -> Option<&dyn LiquidAccount> {
    if let Some(ca) = account.as_any().downcast_ref::<BankAccount>() {
        return Some(ca);
    }
    if let Some(ca) = account.as_any().downcast_ref::<Wallet>() {
        return Some(ca);
    }
    None
}