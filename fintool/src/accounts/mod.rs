/* ------------------------------------------------------------------------
    Copyright (C) 2025  Andrew J. Eberhard

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
  -----------------------------------------------------------------------*/
pub mod bank_account;
pub mod base;
pub mod certificate_of_deposit;
pub mod credit_card_account;
pub mod health_savings_account;
pub mod investment_account_manager;
pub mod retirement_401k_plan;
pub mod roth_ira;
pub mod wallet;

use crate::accounts::bank_account::BankAccount;
use crate::accounts::base::liquid_account::LiquidAccount;
use crate::accounts::base::Account;
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
