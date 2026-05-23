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
use core::{alloc, f32};
use std::backtrace;
use std::collections::HashMap;
use std::io::Read;
use std::ops::Sub;
use std::sync::{Arc, RwLockReadGuard};

use chrono::{Date, Days, Local, NaiveDate, NaiveDateTime};
use chrono::{Datelike, NaiveTime};
use csv::DeserializeError;
use inquire::*;
use rusqlite::types::Value;
use rustyline::validate::Validator;
use time::OffsetDateTime;
use yahoo_finance_api::Quote;
use yahoo_finance_api::YahooError;

use crate::accounts::base::fixed_account::{fixed_account_value, fixed_account_value_on_day};
use crate::accounts::base::{AccountContext, AccountFileIO, DisplayablePositionStatistics, HasContext, HasVariableAccountContext, LedgerOps, SharesOwned, StockData, Valuable, VariableAccountContext};
use crate::accounts::growth::{GrowthCalculable, GrowthMetric, compound_annual_growth_rate, money_weighted_return, simple_rate_of_return, time_weighted_return};
use crate::accounts::base::interest_bearing_fixed_account::{InterestBearingFixedAccount, InterestBearingLedger};
use crate::database::DbConn;
use crate::types::investments::{
    SaleAllocationInfo, SaleAllocationRecord, StockInfo, StockRecord, StockSplitAllocationInfo,
    StockSplitInfo, StockSplitRecord,
};
use crate::types::ledger::{LedgerInfo, LedgerRecord};
use crate::types::participants::ParticipantAutoCompleter;
use crate::types::participants::ParticipantType;
use crate::types::stock_prices::StockPriceInfo;
use crate::types::stock_prices::StockPriceRecord;
use csv::ReaderBuilder;
use rustyline::completion::FilenameCompleter;
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::Completer;
use rustyline::CompletionType;
use rustyline::Config;
use rustyline::EditMode;
use rustyline::Editor;
use rustyline::Helper;
use rustyline::Highlighter;
use rustyline::Hinter;
use rustyline::Validator;
use shared_lib::stocks::{self, get_stock_history, get_stock_quote};
use shared_lib::{FlatLedgerEntry, LedgerEntry, TransferType};
use std::path::Path;

use super::fixed_account::FixedAccount;

#[derive(Helper, Completer, Hinter, Highlighter, Validator)]
pub struct FilePathHelper {
    #[rustyline(Completer)]
    pub completer: FilenameCompleter,
    #[rustyline(Highlighter)]
    pub highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    pub validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    pub hinter: HistoryHinter,
    pub colored_prompt: String,
}

pub trait VariableAccount : HasContext + HasVariableAccountContext + InterestBearingFixedAccount {

    fn purchase_stock(
        &mut self,
        initial_opt: Option<StockRecord>,
        overwrite_entry: bool,
    ) -> Option<LedgerRecord> {
        let ctx = self.ctx();
        let vctx = self.variable_ctx();

        let purchase: LedgerInfo;
        let defaults_to_use: bool;
        let mut initial: StockRecord = StockRecord {
            id: 0,
            info: StockInfo {
                shares: 0.0,
                costbasis: 0.0,
                remaining: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };

        if initial_opt.is_some() {
            defaults_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            defaults_to_use = false;
        }

        let ticker_msg = "Enter stock ticker:";
        let ticker = if defaults_to_use {
            let pid = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .participant;
            let initial_ticker = ctx.db.get_participant(ctx.uid, ctx.aid, pid).unwrap();
            let entered_ticker = Text::new(ticker_msg)
                .with_default(initial_ticker.as_str())
                .with_autocomplete(ParticipantAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    ptype: ParticipantType::Payee,
                    with_accounts: false,
                    stock_tickers_only: true,
                    manually_recorded_only: false,
                })
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string();

            entered_ticker
        } else {
            let entered_ticker = Text::new(ticker_msg)
                .with_autocomplete(ParticipantAutoCompleter {
                    uid: ctx.uid,
                    aid: ctx.aid,
                    db: ctx.db.clone(),
                    ptype: ParticipantType::Payee,
                    with_accounts: false,
                    stock_tickers_only: true,
                    manually_recorded_only: false,
                })
                .prompt()
                .unwrap()
                .to_ascii_uppercase()
                .trim()
                .to_string();

            entered_ticker
        };

        let public_ticker = confirm_public_ticker(ticker.clone());
        let manual_entry = if !public_ticker {
            // check if already a member that is being tracked.
            let pid_opt = ctx.db.get_participant_id(
                ctx.uid,
                ctx.aid,
                ticker.clone(),
                ParticipantType::Payee,
            );
            if let Some(pid) = pid_opt {
                let stock_is_tracked = ctx.db
                    .check_and_get_stock_price_record_matching_from_participant_id(
                        ctx.uid, ctx.aid, pid,
                    )
                    .unwrap();
                if stock_is_tracked.is_empty() {
                    panic!("Non-public ticker does not have a stock price record!");
                }
            } else {
                let manual_entry = Confirm::new(
                    format!("Ticker {} was not publicly found. Would you like to enter its price manually?", ticker.clone())
                    .as_str())
                    .prompt()
                    .unwrap();

                if !manual_entry {
                    println!("Stock was not purchased!");
                    return None;
                }
            }
            true
        } else {
            false
        };

        let pid = ctx.db.check_and_add_participant(
            ctx.uid,
            ctx.aid,
            ticker.clone(),
            ParticipantType::Payee,
            false,
        );

        let date_msg = "Enter date of purchase:";
        let date_input = if defaults_to_use {
            let initial_date = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .date;
            DateSelect::new(date_msg)
                .with_default(NaiveDate::parse_from_str(&initial_date, "%Y-%m-%d").unwrap())
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string()
        } else {
            DateSelect::new(date_msg)
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string()
        };

        let shares_msg = "Enter number of shares purchased:";
        let shares: f32 = if defaults_to_use {
            CustomType::<f32>::new(shares_msg)
                .with_placeholder("0.00")
                .with_default(initial.info.shares)
                .with_error_message("Please enter a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(shares_msg)
                .with_placeholder("0.00")
                .with_default(0.00)
                .with_error_message("Please enter a valid amount!")
                .prompt()
                .unwrap()
        };

        let costbasis_msg = "Enter cost basis of shares purchased:";
        let costbasis: f32 = if defaults_to_use {
            CustomType::<f32>::new(costbasis_msg)
                .with_placeholder("0.00")
                .with_default(initial.info.costbasis)
                .with_error_message("Please enter a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(costbasis_msg)
                .with_placeholder("0.00")
                .with_default(0.00)
                .with_error_message("Please enter a valid amount!")
                .prompt()
                .unwrap()
        };

        let cid = ctx.db
            .check_and_add_category(ctx.uid, ctx.aid, "buy".to_ascii_uppercase());

        purchase = LedgerInfo {
            date: date_input.clone(),
            amount: shares * costbasis,
            transfer_type: TransferType::WithdrawalToInternalAccount,
            participant: pid.clone(),
            category_id: cid,
            description: format!(
                "[Internal] Purchase {} shares of {} at ${} on {}.",
                shares,
                ticker,
                costbasis.clone(),
                date_input.clone()
            ),
        };

        let ledger_id = if defaults_to_use && overwrite_entry {
            ctx.db
                .update_ledger_item(
                    ctx.uid,
                    ctx.aid,
                    LedgerRecord {
                        id: initial.info.ledger_id,
                        info: purchase.clone(),
                    },
                )
                .unwrap()
        } else {
            ctx.db
                .add_ledger_entry(ctx.uid, ctx.aid, purchase.clone())
                .unwrap()
        };

        let stock_record = StockInfo {
            shares: shares,
            costbasis: costbasis,
            remaining: shares,
            ledger_id: ledger_id,
        };

        if manual_entry {
            let stock_price_info = StockPriceInfo {
                date: date_input.clone(),
                stock_ticker_peer_id: pid,
                price_per_unit_share: costbasis.clone(),
            };

            ctx.db
                .add_stock_price(ctx.uid, ctx.aid, stock_price_info)
                .unwrap();
        }

        ctx.db
            .add_stock_purchase(ctx.uid, ctx.aid, stock_record)
            .unwrap();

        let data = initialize_buffer(ctx, vctx);
        self.variable_ctx_mut().buffer = data;

        return Some(LedgerRecord {
            id: ledger_id,
            info: purchase.clone(),
        });
    }

    fn sell_stock(
        &mut self,
        initial_opt: Option<StockRecord>,
        overwrite_entry: bool,
    ) -> Option<LedgerRecord> {
        let ctx = self.ctx();
        let vctx = self.variable_ctx();
        let defaults_to_use: bool;
        let mut initial: StockRecord = StockRecord {
            id: 0,
            info: StockInfo {
                shares: 0.0,
                costbasis: 0.0,
                remaining: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };

        if initial_opt.is_some() {
            defaults_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            defaults_to_use = false;
        }

        let ticker: String;
        let ticker_msg = "Select which stock you would like to record a sale of:";
        let mut tickers_undup = ctx.db.get_stock_tickers(ctx.uid, ctx.aid).unwrap();
        tickers_undup.sort();
        tickers_undup.dedup();
        let tickers = tickers_undup;
        ticker = if defaults_to_use {
            let pid = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .participant;
            let initial_ticker = ctx.db.get_participant(ctx.uid, ctx.aid, pid).unwrap();
            Select::new(ticker_msg, tickers)
                .with_starting_filter_input(initial_ticker.as_str())
                .prompt()
                .unwrap()
                .to_string()
                .to_ascii_uppercase()
        } else {
            Select::new(ticker_msg, tickers)
                .prompt()
                .unwrap()
                .to_string()
                .to_ascii_uppercase()
        };

        let pid = ctx.db.check_and_add_participant(
            ctx.uid,
            ctx.aid,
            ticker.clone(),
            ParticipantType::Payer,
            false,
        );

        let date_msg = "Enter date of sale:";
        let sale_date = if defaults_to_use {
            let initial_date = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .date;
            DateSelect::new(date_msg)
                .with_default(NaiveDate::parse_from_str(&initial_date, "%Y-%m-%d").unwrap())
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
        } else {
            DateSelect::new(date_msg)
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
        };

        let price_msg = "Enter sale price (per share):";
        let sale_price: f32 = if defaults_to_use {
            CustomType::<f32>::new(price_msg)
                .with_placeholder("00000.00")
                .with_default(initial.info.costbasis)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(price_msg)
                .with_placeholder("00000.00")
                .with_default(00000.00)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        };

        let shares_msg = "Enter quantity sold:";
        let number_of_shares_sale: f32 = if defaults_to_use {
            CustomType::<f32>::new(shares_msg)
                .with_placeholder("00000.00")
                .with_default(initial.info.shares)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(shares_msg)
                .with_placeholder("00000.00")
                .with_default(00000.00)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        };

        let value_received = number_of_shares_sale * sale_price;
        let stock_cid =
            ctx.db
                .check_and_add_category(ctx.uid, ctx.aid, "sale".to_ascii_uppercase());

        let sale = LedgerInfo {
            date: sale_date.to_string(),
            amount: value_received,
            transfer_type: TransferType::DepositFromInternalAccount,
            participant: pid,
            category_id: stock_cid,
            description: format!(
                "[Internal]: Sell {} shares of {} at ${} on {}.",
                number_of_shares_sale,
                ticker,
                sale_price,
                sale_date.to_string()
            ),
        };

        let ledger_id: u32 = if defaults_to_use && overwrite_entry {
            ctx.db
                .update_ledger_item(
                    ctx.uid,
                    ctx.aid,
                    LedgerRecord {
                        id: initial.info.ledger_id,
                        info: sale.clone(),
                    },
                )
                .unwrap()
        } else {
            ctx.db
                .add_ledger_entry(ctx.uid, ctx.aid, sale.clone())
                .unwrap()
        };

        let sale_record = StockInfo {
            shares: number_of_shares_sale,
            costbasis: sale_price,
            remaining: 0.0,
            ledger_id: ledger_id,
        };

        let sale_id = ctx.db
            .add_stock_sale(ctx.uid, ctx.aid, sale_record.clone())
            .unwrap();

        let sale_info = StockRecord {
            id: sale_id,
            info: sale_record.clone(),
            txn_opt: Some(sale.clone()),
        };

        const SALE_METHOD_OPTIONS: [&'static str; 2] = ["LIFO", "FIFO"];
        let sell_method: String =
            Select::new("Select sale methodology:", SALE_METHOD_OPTIONS.to_vec())
                .prompt()
                .unwrap()
                .to_string();

        allocate_sale_stock(ctx, sale_info, sell_method);
        let data = initialize_buffer(ctx, vctx);
        self.variable_ctx_mut().buffer = data;


        return Some(LedgerRecord {
            id: ledger_id,
            info: sale.clone(),
        });
    }

    fn split_stock(
        &mut self,
        initial_opt: Option<StockSplitRecord>,
        overwrite_entry: bool,
    ) -> Option<LedgerRecord> {
        let ctx = self.ctx();
        let vctx = self.variable_ctx();
        let defaults_to_use: bool;
        let mut initial: StockSplitRecord = StockSplitRecord {
            id: 0,
            info: StockSplitInfo {
                split: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };

        if initial_opt.is_some() {
            defaults_to_use = true;
            initial = initial_opt.unwrap();
        } else {
            defaults_to_use = false;
        }

        let mut tickers_undup = ctx.db.get_stock_tickers(ctx.uid, ctx.aid).unwrap();
        tickers_undup.sort();
        tickers_undup.dedup();
        tickers_undup.push("None".to_string());
        let mut tickers = tickers_undup;
        let ticker_msg = "Select which stock you would like to report a split of:";
        let ticker = if defaults_to_use {
            let pid = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .participant;
            let initial_ticker: String = ctx.db.get_participant(ctx.uid, ctx.aid, pid).unwrap();
            Select::new(ticker_msg, tickers)
                .with_starting_filter_input(&initial_ticker.as_str())
                .prompt()
                .unwrap()
                .to_string()
        } else {
            Select::new(ticker_msg, tickers)
                .prompt()
                .unwrap()
                .to_string()
        };

        if ticker == "None" {
            return None;
        }

        let split_msg = "Enter split factor:";
        let split: f32 = if defaults_to_use {
            CustomType::<f32>::new(split_msg)
                .with_default(initial.info.split)
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        } else {
            CustomType::<f32>::new(split_msg)
                .with_placeholder("2.0")
                .with_error_message("Please type a valid amount!")
                .prompt()
                .unwrap()
        };

        let date_msg = "Enter date of split:";
        let split_date = if defaults_to_use {
            let initial_date = initial
                .clone()
                .txn_opt
                .expect("Ledger information not populated!")
                .date;
            DateSelect::new(date_msg)
                .with_starting_date(
                    NaiveDate::parse_from_str(initial_date.as_str(), "%Y-%m-%d")
                        .expect("Unable to convert date to NaiveDate"),
                )
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string()
        } else {
            DateSelect::new(date_msg)
                .prompt()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string()
        };

        let pid = ctx.db.check_and_add_participant(
            ctx.uid,
            ctx.aid,
            ticker.clone(),
            ParticipantType::Both,
            false,
        );
        let cid = ctx.db.check_and_add_category(
            ctx.uid,
            ctx.aid,
            "stock dividend/split".to_ascii_uppercase(),
        );

        // if split is for a manually entered stock,
        // then all previous stock price records
        // need to be updated
        let price_records = ctx.db
            .check_and_get_stock_price_record_matching_from_participant_id(ctx.uid, ctx.aid, pid)
            .unwrap();
        if !price_records.is_empty() {
            ctx.db
                .apply_stock_split_to_stock_prices(ctx.uid, ctx.aid, pid, split);
        }

        let ledger_entry = LedgerInfo {
            date: split_date.clone(),
            amount: 0.0,
            transfer_type: TransferType::ZeroSumChange,
            participant: pid,
            category_id: cid,
            description: format!(
                "[Internal]: Split of {} by factor of {} on {}.",
                ticker.clone(),
                split.clone(),
                split_date
            ),
        };

        let lid = if defaults_to_use && overwrite_entry {
            ctx.db
                .update_ledger_item(
                    ctx.uid,
                    ctx.aid,
                    LedgerRecord {
                        id: initial.info.ledger_id,
                        info: ledger_entry.clone(),
                    },
                )
                .unwrap()
        } else {
            ctx.db
                .add_ledger_entry(ctx.uid, ctx.aid, ledger_entry.clone())
                .unwrap()
        };

        let stock_split_id = ctx.db
            .add_stock_split(ctx.uid, ctx.aid, split.clone(), lid)
            .unwrap();

        let stock_split_record = StockSplitRecord {
            id: stock_split_id,
            info: StockSplitInfo {
                split: split.clone(),
                ledger_id: lid.clone(),
            },
            txn_opt: Some(ledger_entry.clone()),
        };

        allocate_stock_split(ctx, vctx, stock_split_record);
        let data = initialize_buffer(ctx, vctx);
        self.variable_ctx_mut().buffer = data;

        return Some(LedgerRecord {
            id: lid,
            info: ledger_entry,
        });
    }
}

pub fn allocate_sale_stock(ctx : &AccountContext, record: StockRecord, method: String) {
    let stocks: Vec<StockRecord>;
    let ticker = ctx.db
        .get_participant(
            ctx.uid,
            ctx.aid,
            record
                .txn_opt
                .expect("Transaction required but not found!")
                .participant,
        )
        .unwrap();
    match method.as_str() {
        "LIFO" => {
            stocks = ctx.db
                .get_stock_history_ascending(ctx.uid, ctx.aid, ticker)
                .unwrap();
        }
        "FIFO" => {
            stocks = ctx.db
                .get_stock_history_descending(ctx.uid, ctx.aid, ticker)
                .unwrap();
        }
        _ => {
            panic!("Unrecognized input!");
        }
    }

    let mut num_shares_remaining_to_allocate = record.info.shares;
    let mut num_shares_allocated: f32;
    for mut stock in stocks {
        let purchase_id = stock.id;

        // can't sell what you don't have
        if stock.info.remaining == 0.0 {
            continue;
        }

        if stock.info.remaining > num_shares_remaining_to_allocate {
            stock.info.remaining = stock.info.remaining - num_shares_remaining_to_allocate;
            num_shares_allocated = num_shares_remaining_to_allocate;
        } else {
            num_shares_allocated = stock.info.remaining;
            stock.info.remaining = 0.0;
        }
        ctx.db
            .update_stock_remaining(ctx.uid, ctx.aid, stock.id, stock.info.remaining)
            .unwrap();
        ctx.db
            .add_stock_sale_allocation(
                ctx.uid,
                ctx.aid,
                purchase_id,
                record.id,
                num_shares_allocated,
            )
            .unwrap();
        num_shares_remaining_to_allocate =
            num_shares_remaining_to_allocate - num_shares_allocated;

        // if there are no shares to allocate, we are done here and all sales
        // are accounted for
        if num_shares_remaining_to_allocate == 0.0 {
            break;
        }
    }
}

fn deallocate_sale_stock(ctx : &AccountContext, vctx : &VariableAccountContext, sale_id: u32) {
    let stock_allocation_records = ctx.db
        .get_stock_sale_allocation_for_sale_id(ctx.uid, ctx.aid, sale_id)
        .unwrap();
    for record in stock_allocation_records {
        // add shares back to ledger
        let _ = ctx.db
            .add_to_stock_remaining(
                ctx.uid,
                ctx.aid,
                record.info.purchase_id,
                record.info.quantity,
            )
            .unwrap();
        ctx.db
            .remove_stock_sale_allocation(ctx.uid, ctx.aid, record.id);
    }
}

pub fn allocate_stock_split(ctx : &AccountContext, vctx : &VariableAccountContext, record: StockSplitRecord) {
    if record.txn_opt.is_none() {
        panic!(
            "Expected ledger data matching stock split id: {}",
            record.id
        );
    }
    let split_txn = record.txn_opt.unwrap();

    let ticker = ctx.db
        .get_participant(ctx.uid, ctx.aid, split_txn.participant)
        .unwrap();
    let stock_purchase_records = ctx.db.get_stocks(ctx.uid, ctx.aid, ticker).unwrap();

    let mut sales_to_update: Vec<(u32, f32)> = Vec::new();
    for stock in stock_purchase_records {
        // update shares so it looks like we have always purchased those
        ctx.db
            .update_stock_shares_purchased(
                ctx.uid,
                ctx.aid,
                stock.id,
                stock.info.shares * record.info.split,
            )
            .unwrap();
        ctx.db
            .update_stock_remaining(
                ctx.uid,
                ctx.aid,
                stock.id,
                stock.info.remaining * record.info.split,
            )
            .unwrap();
        ctx.db
            .update_cost_basis(
                ctx.uid,
                ctx.aid,
                stock.id,
                stock.info.costbasis / record.info.split,
            )
            .unwrap();
        ctx.db
            .add_stock_split_allocation(
                ctx.uid,
                ctx.aid,
                StockSplitAllocationInfo {
                    stock_split_id: record.id,
                    stock_purchase_id: stock.id,
                },
            )
            .unwrap();

        // if stock was part of sale, we need to increase number of stocks sold by factor
        let sale_allocations_opt = ctx.db
            .check_and_get_stock_sale_allocation_record_matching_from_purchase_id(
                ctx.uid, ctx.aid, stock.id,
            )
            .unwrap();
        if sale_allocations_opt.is_none() {
            // no sale allocations founds
            continue;
        }
        let sale_allocations = sale_allocations_opt.unwrap();
        for sale_allocation in sale_allocations {
            let sale_txn_opt = ctx.db
                .check_and_get_stock_sale_record_matching_from_sale_id(
                    ctx.uid,
                    ctx.aid,
                    sale_allocation.info.sale_id,
                )
                .unwrap();
            if sale_txn_opt.is_none() {
                panic!(
                    "Stock stale record not found for sale id: {}",
                    sale_allocation.info.sale_id
                );
            }
            let stock_sale = sale_txn_opt.unwrap();
            if stock_sale.txn_opt.is_none() {
                panic!(
                    "Transaction is missing with sale transaction matching id: {}",
                    stock_sale.id
                );
            }
            let sale_txn: LedgerInfo = stock_sale.txn_opt.unwrap();
            // if the sale occured after the split, ignore it.
            if sale_txn.date > split_txn.date {
                continue;
            }
            ctx.db
                .update_stock_sale_allocation_quantity(
                    ctx.uid,
                    ctx.aid,
                    sale_allocation.id,
                    sale_allocation.info.quantity * record.info.split,
                )
                .unwrap();
            sales_to_update.push((stock_sale.id, stock_sale.info.shares));
        }
    }

    if !sales_to_update.is_empty() {
        sales_to_update.sort_by(|a, b| (a.0).cmp(&b.0));
        sales_to_update.dedup_by(|a, b| a.0 == b.0);
        for sale in sales_to_update {
            ctx.db
                .update_stock_shares_sold(ctx.uid, ctx.aid, sale.0, sale.1 * record.info.split)
                .unwrap();
        }
    }
}

fn deallocate_stock_split(ctx : &AccountContext, vctx : &VariableAccountContext, record: StockSplitRecord) {
    let mut stock_split_alloc_records = ctx.db
        .get_stock_split_allocation_for_stock_split_id(ctx.uid, ctx.aid, record.id)
        .unwrap();
    // remove the highest ids first
    stock_split_alloc_records.sort_by(|a, b| (b.id).cmp(&a.id));

    if record.txn_opt.is_none() {
        panic!(
            "Expected ledger data matching stock split id: {}",
            record.id
        );
    }
    let split_txn = record.txn_opt.unwrap();

    let mut sales_to_update = Vec::new();

    for alloc_record in stock_split_alloc_records {
        // add shares back to ledger
        let stock_purchase = ctx.db
            .check_and_get_stock_purchase_record_matching_from_purchase_id(
                ctx.uid,
                ctx.aid,
                alloc_record.info.stock_purchase_id,
            )
            .unwrap()
            .expect("Stock record not returned");

        let updated_shares = stock_purchase.info.remaining / record.info.split;
        let updated_costbasis = stock_purchase.info.costbasis * record.info.split;

        let _ = ctx.db.update_stock_remaining(
            stock_purchase.id,
            ctx.aid,
            alloc_record.info.stock_purchase_id,
            updated_shares,
        );
        let _ = ctx.db.update_stock_shares_purchased(
            ctx.uid,
            ctx.aid,
            stock_purchase.id,
            updated_shares,
        );
        let _ = ctx.db.update_cost_basis(
            ctx.uid,
            ctx.aid,
            alloc_record.info.stock_purchase_id,
            updated_costbasis,
        );

        // check if there have been any sales affected by this stock that would be affected by this split
        let sale_allocations_opt = ctx.db
            .check_and_get_stock_sale_allocation_record_matching_from_purchase_id(
                ctx.uid,
                ctx.aid,
                alloc_record.info.stock_purchase_id,
            )
            .unwrap();
        if sale_allocations_opt.is_some() {
            let sale_allocations = sale_allocations_opt.unwrap();
            for sale_allocation in sale_allocations {
                let stock_sale_opt = ctx.db
                    .check_and_get_stock_sale_record_matching_from_sale_id(
                        ctx.uid,
                        ctx.aid,
                        sale_allocation.info.sale_id,
                    )
                    .unwrap();
                if stock_sale_opt.is_none() {
                    panic!(
                        "Stock stale record not found for sale id: {}",
                        sale_allocation.info.sale_id
                    );
                }
                let stock_sale = stock_sale_opt.unwrap();
                if stock_sale.txn_opt.is_none() {
                    panic!(
                        "Transaction is missing with sale transaction matching id: {}",
                        stock_sale.id
                    );
                }
                let sale_txn: LedgerInfo = stock_sale.txn_opt.unwrap();

                // if the sale occured after the split, ignore it.
                if sale_txn.date > split_txn.date {
                    continue;
                }
                ctx.db
                    .update_stock_sale_allocation_quantity(
                        ctx.uid,
                        ctx.aid,
                        sale_allocation.id,
                        sale_allocation.info.quantity / record.info.split,
                    )
                    .unwrap();
                sales_to_update.push((stock_sale.id, stock_sale.info.shares));
            }
        }
        ctx.db
            .remove_stock_split_allocation(ctx.uid, ctx.aid, alloc_record.id)
            .unwrap();
    }
    if !sales_to_update.is_empty() {
        sales_to_update.sort_by(|a, b| (a.0).cmp(&b.0));
        sales_to_update.dedup_by(|a, b| a.0 == b.0);
        for sale in sales_to_update {
            ctx.db
                .update_stock_shares_sold(ctx.uid, ctx.aid, sale.0, sale.1 / record.info.split)
                .unwrap();
        }
    }
    ctx.db
        .remove_stock_split(ctx.uid, ctx.aid, record.info.ledger_id)
        .unwrap();
}

pub fn confirm_public_ticker(ticker: String) -> bool {
    let rs = stocks::get_stock_at_close(ticker.clone());
    match rs {
        Ok(price) => true,
        Err(error) => {
            // panic!("Fetch failed for ticker '{}': {}!", ticker.clone(), error);
            false
        }
    }
}

pub fn get_positions(ctx : &AccountContext) -> Option<Vec<(String, f32)>> {
    return ctx.db.get_positions(ctx.uid, ctx.aid).unwrap();
}

pub fn get_costbasis(ctx : &AccountContext, vctx : &VariableAccountContext, ticker: String) -> f32 {
    let x = ctx.db
        .get_total_cost_basis(ctx.aid, ctx.uid, ticker)
        .unwrap();
    if x.is_none() {
        return 0.0;
    } else {
        return x.unwrap();
    }
}

pub fn get_position_stats(ctx : &AccountContext, vctx : &VariableAccountContext) -> Option<Vec<DisplayablePositionStatistics>> {
    #[derive(Debug,Clone)]
    struct Position {
        ticker: String,
        shares: f32,
    };

    if let Some(positions) = get_positions(ctx) {
        let positions = positions
            .iter()
            .map(|x| Position {ticker : x.0.clone(), shares : x.1})
            .collect::<Vec<Position>>();
        let filtered_positions = 
            positions
                .iter()
                .filter(|x| x.shares != 0.0)
                .into_iter()
                .map(|x| x.clone())
                .collect::<Vec<Position>>();

        use std::fs::OpenOptions;
        use std::io::Write;
        // let mut file = OpenOptions::new()
        //     .create(true)
        //     .append(true)
        //     .open("debug.log")
        //     .unwrap();

        let mut statistics : Vec<DisplayablePositionStatistics> = Vec::new();
        for position in filtered_positions {
            let x = format!("{},{},{}", ctx.uid, ctx.aid, position.ticker.to_string());
            // writeln!(file,"{x}").expect("failed to write");
            // file.flush().ok();
            let cost_basis = ctx.db.get_total_cost_basis(ctx.uid, ctx.aid, position.ticker.clone()).unwrap().unwrap();
            let quote_opt = get_latest_quote(ctx,vctx,  position.ticker.clone());
            let stats = if let Some(quote) = quote_opt {
                let unit_price = quote.close as f32;
                let current_value = unit_price * position.shares.clone();
                let effective_unit_cost = cost_basis / position.shares.clone();
                let unrealized_gl = current_value - cost_basis;
                let unrealized_gl_percent = (current_value - cost_basis)/(cost_basis) * 100.;
                
                DisplayablePositionStatistics { 
                    ticker: position.ticker, 
                    quantity: format!("{:.2}",position.shares), 
                    value: format!("{:.2}", current_value), 
                    price: format!("{:.2}", unit_price), 
                    total_cost_basis: format!("{:.2}", cost_basis), 
                    unit_cost: format!("{:.2}", effective_unit_cost), 
                    unrealized_gl: format!("{:.2}", unrealized_gl), 
                    unrealized_gl_per: format!("{:.2}", unrealized_gl_percent) 
                }
            } else {
                DisplayablePositionStatistics { 
                    ticker: position.ticker, 
                    quantity: format!("{:.2}",position.shares), 
                    value: format!("{}", "Not known!"), 
                    price: format!("{}", "Not found!"), 
                    total_cost_basis: format!("{:.2}", cost_basis), 
                    unit_cost: format!("{}", "Not known!"), 
                    unrealized_gl: format!("{}", "Not known!"), 
                    unrealized_gl_per: format!("{}", "Not known!") }
            };

            statistics.push(stats)
        }
        Some(statistics)
    } else { 
        None
    }
}

pub fn get_value_of_positions_on_day(ctx : &AccountContext, vctx : &VariableAccountContext, day: &NaiveDate) -> f32 {
    let mut value: f32 = 0.0;
    if let Some(buffer) = vctx.buffer.as_ref() {
        for e in buffer {
            let mut owned_shares = e
                .history
                .iter()
                .filter(|x| x.date <= *day)
                .collect::<Vec<&SharesOwned>>();
            if owned_shares.is_empty() {
                // if no shares owned before date, then just continue 0
                continue;
            }
            owned_shares.sort_by(|x, y| (x.date).cmp(&y.date));
            let most_recently_owned = owned_shares.last().unwrap();
            let quotes = e
                .quotes
                .iter()
                .filter(|x| {
                    let date = OffsetDateTime::from_unix_timestamp(x.timestamp as i64)
                        .unwrap()
                        .date();
                    let ndate = NaiveDate::from_ymd_opt(
                        date.year(),
                        date.month() as u32,
                        date.day() as u32,
                    )
                    .unwrap();
                    ndate < *day
                })
                .collect::<Vec<&Quote>>();
            let quote_opt = quotes.last();
            if quote_opt.is_none() {
                continue;
            }
            let quote = quote_opt.unwrap();
            let partial_value = (quote.close * most_recently_owned.shares as f64) as f32;
            value = value + partial_value
        }
    }
    return value;
}

pub fn manually_record_stock_close_price(ctx : &AccountContext) {
    let ticker = Text::new("What ticker are you recording for?")
        .with_autocomplete(ParticipantAutoCompleter {
            uid: ctx.uid,
            aid: ctx.aid,
            db: ctx.db.clone(),
            ptype: ParticipantType::Payee,
            with_accounts: false,
            stock_tickers_only: true,
            manually_recorded_only: true,
        })
        .prompt()
        .unwrap();

    let peer_id = ctx.db.check_and_add_participant(
        ctx.uid,
        ctx.aid,
        ticker.clone(),
        ParticipantType::Payee,
        false,
    );

    let date = DateSelect::new("Enter date to record:").prompt().unwrap();
    let close_price = CustomType::<f32>::new(
        format!(
            "Enter close price per unit share on {}:",
            date.to_string().clone()
        )
        .as_str(),
    )
    .prompt()
    .unwrap();
    let info: StockPriceInfo = StockPriceInfo {
        stock_ticker_peer_id: peer_id,
        price_per_unit_share: close_price,
        date: date.to_string(),
    };
    ctx.db.add_stock_price(ctx.uid, ctx.aid, info).unwrap();
}

fn convert_stock_price_record_to_quotes(
    stock_prices: &Vec<StockPriceRecord>,
) -> Vec<yahoo_finance_api::Quote> {
    let mut quotes: Vec<yahoo_finance_api::Quote> = Vec::new();
    for r in stock_prices {
        let timestamp = NaiveDate::parse_from_str(&r.info.date, "%Y-%m-%d")
            .unwrap()
            .and_time(NaiveTime::from_num_seconds_from_midnight_opt(0, 0).unwrap())
            .and_utc()
            .timestamp() as u64;

        quotes.push(Quote {
            timestamp: timestamp,
            // right now, the user doesn't store this information in the database
            // because it might not be available, so set it to the known value.
            open: r.info.price_per_unit_share as f64,
            high: r.info.price_per_unit_share as f64,
            low: r.info.price_per_unit_share as f64,
            close: r.info.price_per_unit_share as f64,
            volume: 0,
            adjclose: r.info.price_per_unit_share as f64,
        });
    }
    return quotes;
}

fn get_latest_quote(ctx : &AccountContext, vctx : &VariableAccountContext, ticker : String) -> Option<Quote> {
    if let Some(buffer) = vctx.buffer.as_ref() { 
        let rcrd = buffer.iter().find(|x| x.ticker == ticker);
        if let Some(record) = rcrd { 
            return record.quotes.last().cloned();
        } else { 
            return None;
        }
    }
    None
}

fn variable_account_value(ctx : &AccountContext, vctx : &VariableAccountContext) -> Option<f32> {
    let today = Local::now().date_naive();
    let fixed = ctx
        .db
        .get_cumulative_total_of_ledger_on_date(ctx.uid, ctx.aid, today)
        .unwrap();
    if fixed.is_none() { 
        return None;
    }
    return Some(fixed.unwrap() + get_value_of_positions_on_day(ctx, vctx,&today));
}

fn variable_account_value_on_day(ctx : &AccountContext, vctx : &VariableAccountContext, day: &NaiveDate) -> Option<f32> {
    let mut value: f32 = 0.0;
    if let Some(buffer) = vctx.buffer.as_ref() {
        for e in buffer {
            let mut owned_shares = e
                .history
                .iter()
                .filter(|x| x.date <= *day)
                .collect::<Vec<&SharesOwned>>();
            if owned_shares.is_empty() {
                // if no shares owned before date, then just continue 0
                continue;
            }
            owned_shares.sort_by(|x, y| (x.date).cmp(&y.date));
            let most_recently_owned = owned_shares.last().unwrap();
            let quote = e
                .quotes
                .iter()
                .filter(|x| {
                    let date = OffsetDateTime::from_unix_timestamp(x.timestamp as i64)
                        .unwrap()
                        .date();
                    let ndate = NaiveDate::from_ymd_opt(
                        date.year(),
                        date.month() as u32,
                        date.day() as u32,
                    )
                    .unwrap();
                    ndate < *day
                })
                .last()
                .expect(
                    format!("No quote matching date {}", most_recently_owned.date).as_str(),
                );
            let partial_value = (quote.close * most_recently_owned.shares as f64) as f32;
            value = value + partial_value
        }
    }

    let fixed_value = ctx
        .db
        .get_cumulative_total_of_ledger_on_date(ctx.uid, ctx.aid, *day)
        .unwrap();
    if let Some(fixed) = fixed_value {
        value = value + fixed;
    } else {
        return None;
    }
    return Some(value);
}
pub trait VariableValuable: Valuable + HasVariableAccountContext {
    fn variable_value(&self) -> Option<f32> {
        variable_account_value(self.ctx(), self.variable_ctx())
    }
    fn variable_value_on_day(&self, day:&  NaiveDate  ) -> Option<f32> {
        variable_account_value_on_day(self.ctx(), self.variable_ctx(), day)
    }
    fn fixed_value(&self) -> Option<f32> {
        fixed_account_value(self.ctx())
    }
    fn fixed_value_on_day(&self, day: &NaiveDate) -> Option<f32> {
        fixed_account_value_on_day(self.ctx(), day)
    }
    fn positions_value_on_day(&self, day: &NaiveDate) -> f32 { 
        get_value_of_positions_on_day(self.ctx(), self.variable_ctx(), day)
    }
}
pub trait VariableGrowth: GrowthCalculable + VariableValuable + HasVariableAccountContext {
    fn variable_growth(&self, metric: GrowthMetric, start_date: NaiveDate, end_date : NaiveDate) -> f32 {
        match metric {
            GrowthMetric::CAGR => {
                compound_annual_growth_rate(self, start_date, end_date)
            }
            GrowthMetric::MWRR => {
                money_weighted_return(self, start_date, end_date)
            }
            GrowthMetric::SimpleReturn => {
                simple_rate_of_return(self, start_date, end_date)
            }
            GrowthMetric::TWRR => {
                time_weighted_return(self, start_date, end_date)
            }
        }
    }
}

pub trait VariableLedger: LedgerOps + HasContext + VariableAccount + InterestBearingLedger { 
    fn modify_variable(&mut self, record: LedgerRecord) -> Option<LedgerRecord> {
        let ctx = self.ctx();
        let vctx = self.variable_ctx();
        let was_stock_purchase_opt = ctx
            .db
            .check_and_get_stock_purchase_record_matching_from_ledger_id(
                ctx.uid, ctx.aid, record.id,
            )
            .unwrap();
        let was_stock_sale_opt = ctx
            .db
            .check_and_get_stock_sale_record_matching_from_ledger_id(ctx.uid, ctx.aid, record.id)
            .unwrap();
        let was_stock_split_opt = ctx
            .db
            .check_and_get_stock_split_record_matching_from_ledger_id(ctx.uid, ctx.aid, record.id)
            .unwrap();

        let mut is_stock_purchase: bool = false;
        let mut is_stock_sale: bool = false;
        let mut is_stock_split: bool = false;
        let mut stock_record: StockRecord = StockRecord {
            id: 0,
            info: StockInfo {
                shares: 0.0,
                costbasis: 0.0,
                remaining: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };
        let mut split_record: StockSplitRecord = StockSplitRecord {
            id: 0,
            info: StockSplitInfo {
                split: 0.0,
                ledger_id: 0,
            },
            txn_opt: None,
        };
        if was_stock_purchase_opt.is_none()
            && was_stock_sale_opt.is_none()
            && was_stock_split_opt.is_none()
        {
            return <Self as InterestBearingLedger>::modify_interest_bearing(self, record);
        }

        if was_stock_purchase_opt.is_some() {
            is_stock_purchase = true;
            stock_record = was_stock_purchase_opt.unwrap();
            stock_record.txn_opt = Some(record.info.clone());
        } else if was_stock_sale_opt.is_some() {
            is_stock_sale = true;
            stock_record = was_stock_sale_opt.unwrap();
            stock_record.txn_opt = Some(record.info.clone());
        } else {
            is_stock_split = true;
            split_record = was_stock_split_opt.unwrap();
            split_record.txn_opt = Some(record.info.clone());
        }

        const OPTIONS: [&'static str; 3] = ["Update", "Remove", "None"];
        let modify_choice = Select::new("What would you like to do:", OPTIONS.to_vec())
            .prompt()
            .unwrap();
        match modify_choice {
            "Update" => {
                if is_stock_purchase {
                    ctx.db
                        .remove_stock_purchase(ctx.uid, ctx.aid, stock_record.id)
                        .unwrap();
                    return self.purchase_stock(Some(stock_record), true);
                } else if is_stock_sale {
                    deallocate_sale_stock(ctx, vctx, stock_record.id);
                    ctx.db
                        .remove_stock_sale(ctx.uid, ctx.aid, stock_record.info.ledger_id)
                        .unwrap();
                    return self.sell_stock(Some(stock_record), true);
                } else {
                    // split stock
                    deallocate_stock_split(ctx, vctx, split_record.clone());
                    return self.split_stock(Some(split_record.clone()), true);
                }
            }
            "Remove" => {
                if is_stock_purchase {
                    ctx.db
                        .remove_ledger_item(ctx.uid, ctx.aid, stock_record.info.ledger_id)
                        .unwrap();
                } else if is_stock_sale {
                    deallocate_sale_stock(ctx, vctx, stock_record.clone().id);
                    ctx.db
                        .remove_ledger_item(ctx.uid, ctx.aid, stock_record.info.ledger_id)
                        .unwrap();
                } else {
                    deallocate_stock_split(ctx, vctx, split_record.clone());
                    ctx.db
                        .remove_ledger_item(ctx.uid, ctx.aid, split_record.info.ledger_id)
                        .unwrap();
                }
                return Some(record);
            }
            "None" => {
                return Some(record);
            }
            _ => {
                panic!("Input not recognized!");
            }
        }
    }
}

pub fn initialize_buffer(ctx : &AccountContext, vctx : &VariableAccountContext) -> Option<Vec<StockData>> {

    // this is a quick hack to update the buffer after a stock has been purchased, sold or split
    let earliest_date = ctx.open_date;
    let latest_date = Local::now().date_naive();

    let x = ctx.db.get_positions_by_ledger(ctx.aid, ctx.uid).unwrap();
    if x.is_some() {
        let x = x.unwrap();
        let mut tickers = x.iter().map(|x| x.0.clone()).collect::<Vec<String>>();
        tickers.dedup();
        let mut data: Vec<StockData> = Vec::new();

        let buffer = if let Some(buffer) = vctx.buffer.clone() {
            buffer
        } else {
            Vec::new()
        };

        for ticker in tickers {
            let date_shares = x
                .iter()
                .filter(|data| data.0 == ticker)
                .map(|x: &(String, String, f32)| {
                    (SharesOwned {
                        date: NaiveDate::parse_from_str(&&x.1, "%Y-%m-%d")
                            .expect(format!("Unable to decode {}", &x.1).as_str()),
                        shares: x.2.clone(),
                    })
                })
                .collect::<Vec<SharesOwned>>();
            let quotes = buffer
                .iter()
                .find(|x| x.ticker == ticker)
                .and_then(|x| Some(x.quotes.clone()));
            let quotes = quotes
                .or_else(|| {
                    Some({
                        let pid = ctx.db
                            .get_participant_id(
                                ctx.uid,
                                ctx.aid,
                                ticker.clone(),
                                ParticipantType::Payee,
                            )
                            .unwrap();
                        let manual_prices = ctx.db
                            .check_and_get_stock_price_record_matching_from_participant_id(
                                ctx.uid, ctx.aid, pid,
                            )
                            .unwrap();
                        if manual_prices.is_empty() {
                            get_stock_history(ticker.clone(), earliest_date, latest_date)
                                .unwrap()
                        } else {
                            convert_stock_price_record_to_quotes(&manual_prices)
                        }
                    })
                })
                .unwrap();
            data.push(StockData {
                ticker: ticker.clone(),
                quotes: quotes,
                history: date_shares,
            });
        }
        return Some(data);
    } else {
        return None;
    }
}

pub trait VariableAccountFileIO : AccountFileIO + HasVariableAccountContext {

    fn import_variable_account(&self) {
        let ctx = self.ctx();
        let vctx = self.variable_ctx();
        let g = FilePathHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter::new(),
            validator: MatchingBracketValidator::new(),
            colored_prompt: "".to_owned(),
        };
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Vi)
            .build();
        let mut rl = Editor::with_config(config).unwrap();
        rl.set_helper(Some(g));

        let mut fp = Path::new("~");
        let mut bad_path;
        let mut csv: String = String::new();
        loop {
            csv = rl.readline("Enter path to CSV file: ").unwrap();
            if csv.to_string() == "none" {
                return;
            }
            bad_path = match Path::new(&csv).try_exists() {
                Ok(true) => false,
                Ok(false) => {
                    println!("File {} cannot be found!", Path::new(&csv).display());
                    true
                }
                Err(e) => {
                    println!("File {} cannot be found: {}!", e, Path::new(&csv).display());
                    true
                }
            };
            if !bad_path {
                break;
            } else {
                let try_again = Confirm::new("Continue import?").prompt().unwrap();
                if !try_again {
                    return;
                }
            }
        }
        fp = Path::new(&csv);

        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_path(fp)
            .unwrap();

        let mut ledger_entries = Vec::new();
        for result in rdr.deserialize::<LedgerEntry>() {
            ledger_entries.push(result.unwrap());
        }
        ledger_entries.sort_by(|x, y| {
            (NaiveDate::parse_from_str(&x.date, "%Y-%m-%d").unwrap())
                .cmp(&NaiveDate::parse_from_str(&y.date, "%Y-%m-%d").unwrap())
        });

        for entry in ledger_entries {
            let ptype = if entry.transfer_type
                == shared_lib::TransferType::WithdrawalToExternalAccount
            {
                ParticipantType::Payee
            } else if entry.transfer_type == shared_lib::TransferType::WithdrawalToInternalAccount {
                ParticipantType::Payee
            } else if entry.transfer_type == shared_lib::TransferType::DepositFromExternalAccount {
                ParticipantType::Payer
            } else {
                ParticipantType::Payer
            };

            let lid: u32;
            let txn: LedgerInfo;
            if entry.stock_info.is_some() {
                let s: shared_lib::StockInfo = entry
                    .stock_info
                    .expect("Unable to obtain stock information!");

                if s.is_buy {
                    if s.is_split {
                        // if split, check that we own this symbol
                        let symbols_owned = ctx.db.get_stock_tickers(ctx.uid, ctx.aid).unwrap();
                        let symbol_found = symbols_owned
                            .iter()
                            .any(|i| *i == entry.participant.clone());
                        if !symbol_found {
                            panic!("Attempting to register split of symbol not owned by account!");
                        }

                        txn = LedgerInfo {
                            date: entry.date,
                            amount: entry.amount,
                            transfer_type: entry.transfer_type as TransferType,
                            participant: ctx.db.check_and_add_participant(
                                ctx.uid,
                                ctx.aid,
                                entry.participant.clone(),
                                ptype,
                                false,
                            ),
                            category_id: ctx.db.check_and_add_category(
                                ctx.uid,
                                ctx.aid,
                                entry.category.to_ascii_uppercase(),
                            ),
                            description: entry.description,
                        };

                        lid = ctx.db
                            .add_ledger_entry(ctx.uid, ctx.aid, txn.clone())
                            .unwrap();

                        // get total shares for ticker and divide by split
                        let stocks_owned = ctx.db
                            .get_stocks(ctx.uid, ctx.aid, entry.participant.clone())
                            .unwrap();
                        let all_shares: f32 = stocks_owned.iter().map(|x| x.info.remaining).sum();
                        // lpl takes the split and adds the difference to your account
                        // i.e., if the split is 3:1, it will take your 1 part and add 2 parts
                        let split_factor = (s.shares + all_shares) / all_shares;
                        let stock_split_id = ctx.db
                            .add_stock_split(ctx.uid, ctx.aid, split_factor.clone(), lid)
                            .unwrap();

                        let stock_split_record = StockSplitRecord {
                            id: stock_split_id,
                            info: StockSplitInfo {
                                split: split_factor,
                                ledger_id: lid,
                            },
                            txn_opt: Some(txn),
                        };

                        allocate_stock_split(&ctx, &vctx, stock_split_record);
                    } else {
                        // if buy, confirm it is a public ticker
                        let public_ticker = confirm_public_ticker(entry.participant.clone());

                        let pid = ctx.db.check_and_add_participant(
                            ctx.uid,
                            ctx.aid,
                            entry.participant.clone(),
                            ptype,
                            false,
                        );

                        txn = LedgerInfo {
                            date: NaiveDate::parse_from_str(entry.date.as_str(), "%Y-%m-%d")
                                .unwrap()
                                .format("%Y-%m-%d")
                                .to_string(),
                            amount: entry.amount,
                            transfer_type: entry.transfer_type as TransferType,
                            participant: pid.clone(),
                            category_id: ctx.db.check_and_add_category(
                                ctx.uid,
                                ctx.aid,
                                entry.category.to_ascii_uppercase(),
                            ),
                            description: entry.description,
                        };

                        lid = ctx.db.add_ledger_entry(ctx.uid, ctx.aid, txn).unwrap();

                        let my_s: crate::types::investments::StockInfo = StockInfo {
                            shares: s.shares,
                            costbasis: s.costbasis,
                            remaining: s.remaining,
                            ledger_id: lid,
                        };

                        ctx.db.add_stock_purchase(ctx.uid, ctx.aid, my_s).unwrap();

                        if !public_ticker {
                            let stock_price_info = StockPriceInfo {
                                date: entry.date.clone(),
                                stock_ticker_peer_id: pid,
                                price_per_unit_share: s.costbasis,
                            };

                            ctx.db
                                .add_stock_price(ctx.uid, ctx.aid, stock_price_info)
                                .unwrap();
                        }
                    }
                } else {
                    // if sale, check that we own this symbol
                    let symbols_owned = ctx.db.get_stock_tickers(ctx.uid, ctx.aid).unwrap();
                    let symbol_found = symbols_owned
                        .iter()
                        .any(|i| *i == entry.participant.clone());
                    if !symbol_found {
                        panic!("Attempting to register sale of symbol not owned by account!");
                    }

                    txn = LedgerInfo {
                        date: entry.date,
                        amount: entry.amount,
                        transfer_type: entry.transfer_type as TransferType,
                        participant: ctx.db.check_and_add_participant(
                            ctx.uid,
                            ctx.aid,
                            entry.participant.clone(),
                            ptype,
                            false,
                        ),
                        category_id: ctx.db.check_and_add_category(
                            ctx.uid,
                            ctx.aid,
                            entry.category.to_ascii_uppercase(),
                        ),
                        description: entry.description,
                    };

                    lid = ctx.db
                        .add_ledger_entry(ctx.uid, ctx.aid, txn.clone())
                        .unwrap();

                    let my_s: crate::types::investments::StockInfo = StockInfo {
                        shares: s.shares,
                        costbasis: s.costbasis,
                        remaining: s.remaining,
                        ledger_id: lid,
                    };
                    let sale_id = ctx.db
                        .add_stock_sale(ctx.uid, ctx.aid, my_s.clone())
                        .unwrap();
                    allocate_sale_stock(
                        &ctx,
                        StockRecord {
                            id: sale_id,
                            info: my_s,
                            txn_opt: Some(txn),
                        },
                        "LIFO".to_string(),
                    );
                }
            } else {
                // this is just a normal ledger transaction
                let txn: LedgerInfo = LedgerInfo {
                    date: NaiveDate::parse_from_str(entry.date.as_str(), "%Y-%m-%d")
                        .unwrap()
                        .format("%Y-%m-%d")
                        .to_string(),
                    amount: entry.amount,
                    transfer_type: entry.transfer_type as TransferType,
                    participant: ctx.db.check_and_add_participant(
                        ctx.uid,
                        ctx.aid,
                        entry.participant,
                        ptype,
                        false,
                    ),
                    category_id: ctx.db.check_and_add_category(
                        ctx.uid,
                        ctx.aid,
                        entry.category.to_ascii_uppercase(),
                    ),
                    description: entry.description,
                };

                lid = ctx.db.add_ledger_entry(ctx.uid, ctx.aid, txn).unwrap();
            }
        }
        // let data = initialize_buffer(&ctx, &vctx);
        // vctx.buffer = data;
    }

    fn export_variable_account(&self) {
        let ctx = self.ctx();
        let g = FilePathHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter::new(),
            validator: MatchingBracketValidator::new(),
            colored_prompt: "".to_owned(),
        };
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Vi)
            .build();
        let mut rl = Editor::with_config(config).unwrap();
        rl.set_helper(Some(g));

        let mut wtr =
            csv::Writer::from_path(rl.readline("Enter path to CSV file: ").unwrap()).unwrap();
        let ledger = self.get_ledger();
        if !ledger.is_empty() {
            for record in ledger {
                let stock_record_opt = match record.info.transfer_type {
                    TransferType::ZeroSumChange => {
                        // this is a stock split
                        let stock_split_opt = ctx.db
                            .check_and_get_stock_split_record_matching_from_ledger_id(
                                ctx.uid, ctx.aid, record.id,
                            )
                            .unwrap();
                        if let Some(ss_record) = stock_split_opt {
                            Some(shared_lib::StockInfo {
                                shares: 0.0,
                                costbasis: 0.0,
                                remaining: 0.0,
                                is_buy: true,
                                is_split: true,
                            })
                        } else {
                            None
                        }
                    }
                    TransferType::DepositFromInternalAccount => {
                        // this could either be a sale or a dividend, if dividend than expect to return none
                        let stock_sale_opt = ctx.db
                            .check_and_get_stock_sale_record_matching_from_ledger_id(
                                ctx.uid, ctx.aid, record.id,
                            )
                            .unwrap();
                        if let Some(stock_sale) = stock_sale_opt {
                            Some(shared_lib::StockInfo {
                                shares: stock_sale.info.shares,
                                costbasis: stock_sale.info.costbasis,
                                remaining: 0.0,
                                is_buy: false,
                                is_split: false,
                            })
                        } else {
                            None
                        }
                    }
                    TransferType::WithdrawalToInternalAccount => {
                        // this is purchase
                        let purchase_opt = ctx.db
                            .check_and_get_stock_purchase_record_matching_from_ledger_id(
                                ctx.uid, ctx.aid, ctx.aid,
                            )
                            .unwrap();
                        if let Some(purchase) = purchase_opt {
                            Some(shared_lib::StockInfo {
                                shares: purchase.info.shares,
                                costbasis: purchase.info.costbasis,
                                remaining: 0.0,
                                is_buy: false,
                                is_split: false,
                            })
                        } else {
                            None
                        }
                    }
                    TransferType::DepositFromExternalAccount
                    | TransferType::WithdrawalToExternalAccount => None,
                };

                let csv_ledger_record: shared_lib::LedgerEntry = LedgerEntry {
                    date: record.info.date,
                    amount: record.info.amount,
                    transfer_type: record.info.transfer_type,
                    participant: ctx.db
                        .get_participant(ctx.uid, ctx.aid, record.info.participant)
                        .unwrap(),
                    category: ctx.db
                        .get_category_name(ctx.uid, ctx.aid, record.info.category_id)
                        .unwrap(),
                    description: record.info.description,
                    stock_info: stock_record_opt,
                };
                let flattened = FlatLedgerEntry::from(csv_ledger_record);
                wtr.serialize(flattened).unwrap();
            }
        }
    }
}

