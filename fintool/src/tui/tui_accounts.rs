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
use core::panic;

use crate::accounts::base::liquid_account::{self, LiquidAccount};
#[cfg(feature = "ratatui_support")]
use crate::accounts::base::AnalysisPeriod;
use crate::database::{self, *};
// use crate::tui::tui_budgets::create_budget;
use crate::tui::BankAccount;
use crate::types::accounts::AccountType;
use crate::types::accounts::*;
use crate::Account;
#[cfg(feature = "ratatui_support")]
use chrono::NaiveDate;
use inquire::*;

pub fn select_account_by_type(
    _uid: u32,
    _db: &mut DbConn,
    atype: AccountType,
) -> Option<(u32, String)> {
    let msg;
    match &atype {
        &AccountType::Bank => msg = "Select bank account: ",
        &AccountType::CD => msg = "Select CD account: ",
        &AccountType::Wallet => msg = "Select wallet: ",
        &AccountType::Investment => msg = "Select investment account: ",
        _ => panic!("Unrecognized account type!"),
    }
    let accounts: Option<Vec<String>> = _db.get_user_accounts_by_type(_uid, atype).unwrap();
    if accounts.is_none() {
        return None;
    }
    let account: String = Select::new(msg, accounts.unwrap())
        .prompt()
        .unwrap()
        .to_string();
    let aid = _db.get_account_id(_uid, account.clone()).unwrap();
    return Some((aid, account));
}

pub fn select_account_by_filter(_uid: u32, _db: &mut DbConn, filter: AccountFilter) -> u32 {
    let msg;
    match &filter {
        &AccountFilter::Bank => msg = "Select bank account: ",
        &AccountFilter::Stocks => msg = "Select investment account: ",
        &AccountFilter::Wallet => msg = "Select wallet account: ",
        &AccountFilter::Budget => msg = "Select budget account: ",
    }
    let accounts: Vec<String> = _db.get_user_accounts_by_filter(_uid, filter).unwrap();
    let account: String = Select::new(msg, accounts).prompt().unwrap().to_string();
    let aid = _db.get_account_id(_uid, account).unwrap();
    return aid;
}

#[cfg(feature = "ratatui_support")]
pub fn get_total_assets(accounts: &Vec<Box<dyn Account>>) -> f32 {
    let mut assets = 0.0;
    for account in accounts {
        match account.kind() {
            AccountType::CreditCard => assets = assets,
            _ => assets = assets + account.get_value(),
        }
    }
    return assets;
}

#[cfg(feature = "ratatui_support")]
pub fn get_total_liabilities(accounts: &Vec<Box<dyn Account>>) -> f32 {
    let mut liabilities = 0.0;
    for account in accounts {
        match account.kind() {
            AccountType::CreditCard => liabilities = liabilities + account.get_value(),
            _ => liabilities = liabilities,
        }
    }
    return liabilities;
}

#[cfg(feature = "ratatui_support")]
pub fn get_dollar_change_y2y(
    accounts: &Vec<Box<dyn Account>>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> f32 {
    let mut starting_value = 0.0;
    let mut ending_value = 0.0;
    for account in accounts {
        match account.kind() {
            AccountType::CreditCard => starting_value = starting_value,
            _ => {
                starting_value = starting_value + account.get_value_on_day(start_date);
                ending_value = ending_value + account.get_value_on_day(end_date);
            }
        }
    }

    return (ending_value - starting_value);
}

#[cfg(feature = "ratatui_support")]
pub fn get_net_worth_growth(
    accounts: &Vec<Box<dyn Account>>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> f32 {
    let mut starting_value = 0.0;
    let mut ending_value = 0.0;
    for account in accounts {
        match account.kind() {
            AccountType::CreditCard => starting_value = starting_value,
            _ => {
                starting_value = starting_value + account.get_value_on_day(start_date);
                ending_value = ending_value + account.get_value_on_day(end_date);
            }
        }
    }

    return (ending_value - starting_value) / (starting_value) * 100.;
}

#[cfg(feature = "ratatui_support")]
pub fn get_compound_annual_growth_rate(
    accounts: &Vec<Box<dyn Account>>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> f32 {
    let mut starting_value = 0.0;
    let mut ending_value = 0.0;
    for account in accounts {
        match account.kind() {
            AccountType::CreditCard => starting_value = starting_value,
            _ => {
                starting_value = starting_value + account.get_value_on_day(start_date);
                ending_value = ending_value + account.get_value_on_day(end_date);
            }
        }
    }

    return (f32::powf(
        ending_value / starting_value,
        1.0 / (((end_date - start_date).num_days() as f32) / 365.25),
    ) - 1.)
        * 100.;
}
