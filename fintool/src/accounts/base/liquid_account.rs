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
use chrono::NaiveDate;
use crate::accounts::{HasContext, base::LedgerOps};

pub trait LiquidAccount : HasContext + LedgerOps {
    fn get_positive_cash_flow(&self, start: NaiveDate, end: NaiveDate) -> f32 {
        let ctx = self.ctx();
        let ledger = self.get_ledger_entries_between_timestamps(start, end);
        if ledger.is_empty() {
            return 0.0;
        }

        let mut amt = 0.0;
        for txn in ledger {
            if !txn.info.transfer_type.is_deposit() {
                continue;
            }

            if let Some(link) = ctx.db
                .check_and_get_account_transaction_record_matching_to_ledger_id(
                    ctx.uid, ctx.aid, txn.id,
                )
                .unwrap()
            {
                // if linked, checked to see that the account is not another liquid account (if liquid, then skip because cash is still available)
                let peer_account = ctx.db
                    .get_account(ctx.uid, link.info.from_account)
                    .unwrap();
                if peer_account.is_liquid_account() {
                    continue;
                }
            }
            amt = amt + txn.info.amount;
        }

        amt
    }    
    fn get_negative_cash_flow(&self, start: NaiveDate, end: NaiveDate) -> f32 {
        let ctx = self.ctx();
        let ledger = self.get_ledger_entries_between_timestamps(start, end);
        if ledger.is_empty() {
            return 0.0;
        }

        let mut amt = 0.0;
        for txn in ledger {
            if !txn.info.transfer_type.is_withdrawal() {
                continue;
            }

            if let Some(link) = ctx.db
                .check_and_get_account_transaction_record_matching_to_ledger_id(
                    ctx.uid, ctx.aid, txn.id,
                )
                .unwrap()
            {
                // if linked, checked to see that the account is not another liquid account (if liquid, then skip because cash is still available)
                let peer_account = ctx.db
                    .get_account(ctx.uid, link.info.from_account)
                    .unwrap();
                if peer_account.is_liquid_account() {
                    continue;
                }
            }
            amt = amt + txn.info.amount;
        }

        amt
    }
    fn get_cash_flow(&self, start: NaiveDate, end: NaiveDate) -> f32 {
        return self.get_positive_cash_flow(start, end) - self.get_negative_cash_flow(start, end);
    }
}
