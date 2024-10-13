// use super::{db_banks::BankRecord, db_investments::StockRecord, DbConn, SQLITE_WILDCARD};
// use rusqlite::{Error, Result};

// pub struct HsaRecord {
//     pub fixed: BankRecord,
//     pub investments: Vec<StockRecord>,
// }

// impl DbConn {
//     pub fn record_hsa_account(&mut self, aid: u32, record: HsaRecord) -> Result<(), Error> {
//         let sql: &str = "INSERT INTO banks (date, fixed, aid) VALUES (?1, ?2, ?3)";
//         let fmsg = format!("Unable to record HSA bank record for account: {}", aid);
//         self.conn
//             .execute(
//                 sql,
//                 rusqlite::params!(record.fixed.date, record.fixed.amount, aid),
//             )
//             .expect(fmsg.as_str());
//         // let sql: &str = "INSERT INTO investments (date, ticker, shares, costbasis, aid)
//         let fmsg = format!("Unable to record HSA stock record for account: {}", aid);
//         for stock in record.investments {
//             self.add_stock(aid, stock).expect(fmsg.as_str());
//         }
//         Ok(())
//     }

//     pub fn get_hsa_value(&mut self, aid: u32) -> Result<HsaRecord, rusqlite::Error> {
//         let bank = self
//             .get_bank_value(aid)
//             .expect("Unable to retrieve HSA account record!");
//         // let tickers = self.get_stocks(aid).expect("Unable to retrieve HSA stocks!");
//         Ok(HsaRecord {
//             fixed: bank,
//             investments: self.cumulate_stocks(aid, SQLITE_WILDCARD.to_string()),
//         })
//     }
// }
