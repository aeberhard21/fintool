#[cfg(feature = "ratatui_support")]
use ratatui::{
    layout::Rect,
    style::Color,
    text::Line,
    widgets::{Block, Tabs},
    Frame,
};
use rusqlite::{Error, Result};
use strum::{Display, EnumIter, EnumString, FromRepr, IntoEnumIterator};

#[cfg(feature = "ratatui_support")]
use crate::app::screen::TabMenu;
use crate::database::DbConn;

use super::ledger;

#[derive(Clone, Copy, Display, EnumIter, EnumString, FromRepr, Default)]
pub enum AccountType {
    #[default]
    #[strum(to_string = "Bank Account")]
    Bank,
    #[strum(to_string = "Investment Account")]
    Investment,
    #[strum(to_string = "Credit Card")]
    CreditCard,
    #[strum(to_string = "Wallet")]
    Wallet,
    #[strum(to_string = "Certificate of Deposit")]
    CD,
    // #[strum(to_string = "401k Account")]
    // Retirement401k,
    #[strum(to_string = "Roth IRA Account")]
    RetirementRothIra,
    #[strum(to_string = "Health Savings Account")]
    HealthSavingsAccount,
}

impl AccountType {
    pub fn to_menu_selection(value: Self) -> String {
        format!("{value}")
    }
}

#[cfg(feature = "ratatui_support")]
impl TabMenu for AccountType {
    fn previous(self) -> Self {
        let current = self as usize;
        let prev = current.saturating_sub(1);
        Self::from_repr(prev).unwrap_or(self)
    }
    fn next(self) -> Self {
        let current = self as usize;
        let next = current.saturating_add(1);
        Self::from_repr(next).unwrap_or(self)
    }
    fn to_tab_title(value: Self) -> Line<'static> {
        let text = format!("  {value}  ");
        text.into()
    }
    fn render(frame: &mut Frame, area: Rect, selected_tab: usize, title: String, color : Color) {
        let atype_tabs = Tabs::new(AccountType::iter().map(AccountType::to_tab_title))
            .highlight_style(color)
            .select(selected_tab)
            .block(Block::bordered().title(title))
            .padding("", "")
            .divider(" ");
        frame.render_widget(atype_tabs, area);
    }
}

pub enum AccountFilter {
    Stocks,
    Bank,
    Wallet,
    Budget,
}

#[derive(Clone)]
pub struct AccountInfo {
    pub atype: AccountType,
    pub name: String,
    pub has_stocks: bool,
    pub has_bank: bool,
    pub has_ledger: bool,
    pub has_budget: bool,
}

#[derive(Clone)]
pub struct AccountRecord {
    pub id: u32,
    pub info: AccountInfo,
}

pub struct AccountTransaction {
    pub from_account: u32,
    pub to_account: u32,
    pub from_ledger: u32,
    pub to_ledger: u32,
}

pub struct AccountTransactionRecord {
    pub id: u32,
    pub info: AccountTransaction,
}

impl DbConn {
    pub fn create_accounts_id_table(&self) -> rusqlite::Result<()> {
        let sql = "
            CREATE TABLE IF NOT EXISTS account_ids (
                uid INTEGER NOT NULL PRIMARY KEY,
                next_account_id INTEGER NOT NULL,
                next_account_transaction_id INTEGER NOT NULL,
                next_label_id INTEGER NOT NULL,
                FOREIGN KEY (uid) REFERENCES users(id)
            )   
        ";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create account_ids table: {}", error)
            }
        }
        Ok(())
    }

    pub fn initialize_user_account_table(&self, uid: u32) -> rusqlite::Result<()> {
        let p = rusqlite::params![uid, 0, 0, 0];
        let sql: &str = "
            INSERT INTO account_ids 
                (uid, next_account_id, next_account_transaction_id, next_label_id) 
            VALUES 
                ( ?1, ?2, ?3, ?4)
        ";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!(
                    "Unable to intialize account ids table for for user {}:\n\t{}",
                    uid, error
                );
            }
        }
        Ok(())
    }

    pub fn get_next_account_id(&self, uid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT next_account_id FROM account_ids WHERE uid = (?1)";
        let p = rusqlite::params![uid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE account_ids SET next_account_id = next_account_id + 1 WHERE uid = (?1)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next account ID within table 'account_ids' does not exist.");
            }
        }
    }

    pub fn get_next_transaction_id(&self, uid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT next_account_transaction_id FROM account_ids WHERE uid = (?1)";
        let p = rusqlite::params![uid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE account_ids SET next_account_transaction_id = next_account_transaction_id + 1  WHERE uid = (?1)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next transaction ID within table 'account_ids' does not exist.");
            }
        }
    }

    pub fn get_next_label_id(&self, uid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT next_label_id FROM account_ids WHERE uid = (?1)";
        let p = rusqlite::params![uid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE account_ids SET next_label_id = next_label_id + 1  WHERE uid = (?1)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next label ID within table 'account_ids' does not exist.");
            }
        }
    }

    pub fn create_accounts_table(&self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS accounts (
                id   INTEGER NOT NULL, 
                type INTEGER NOT NULL, 
                name TEXT NOT NULL,
                stocks BOOL NOT NULL,
                bank   BOOL NOT NULL,
                ledger BOOL NOT NULL,
                budget BOOL NOT NULL,
                uid  INTEGER NOT NULL,
                PRIMARY KEY (uid, id),
                FOREIGN KEY (uid) REFERENCES users(id)
            )";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create: {}", error)
            }
        }
        Ok(())
    }

    pub fn account_with_name_exists(
        &self,
        uid: u32,
        name: String,
    ) -> Result<bool, rusqlite::Error> {
        let p = rusqlite::params![uid, name];
        let sql = "
            SELECT * FROM
                accounts
            WHERE  
                uid = (?1) AND
                name = (?2)
        ";
        let exists = self.conn.lock().unwrap().prepare(sql)?.exists(p);
        return exists;
    }

    pub fn add_account(&self, uid: u32, info: &AccountInfo) -> Result<u32> {
        let aid = self.get_next_account_id(uid).unwrap();
        let p = rusqlite::params![
            aid,
            info.atype as usize,
            info.name,
            info.has_stocks,
            info.has_bank,
            info.has_ledger,
            info.has_budget,
            uid
        ];
        let sql: &str = "INSERT INTO accounts (id, type, name, stocks, bank, ledger, budget, uid) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_) => {
                std::mem::drop(conn_lock);
                self.initialize_user_account_info_table(uid, aid).unwrap();
                Ok(aid)
            }
            Err(error) => {
                panic!(
                    "Unable to add account {} for user {}: {}!",
                    &info.name, &uid, error
                );
            }
        }
    }

    pub fn update_account(&self, uid: u32, aid : u32, updated_info: &AccountInfo) -> Result<u32> {
        let p = rusqlite::params![
            aid,
            uid,
            updated_info.atype as usize,
            updated_info.name,
            updated_info.has_stocks,
            updated_info.has_bank,
            updated_info.has_ledger,
            updated_info.has_budget,
        ];
        let sql: &str = "
            UPDATE accounts 
            SET 
                type = (?3), name = (?4), stocks = (?5) , bank = (?6), ledger = (?7), budget = (?8) 
            WHERE
                id = (?1) and uid = (?2)
        ";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update account: {}", error);
            }
        }
        Ok(aid)
    }


    pub fn create_account_transaction_table(&self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS account_transactions (
            id              INTEGER NOT NULL,
            from_account_id INTEGER NOT NULL, 
            from_ledger_id  INTEGER NOT NULL, 
            to_account_id   INTEGER NOT NULL,
            to_ledger_id    INTEGER NOT NULL, 
            uid             INTEGER NOT NULL,
            PRIMARY KEY (uid, id),
            FOREIGN KEY (uid, from_account_id) REFERENCES accounts(uid, id),
            FOREIGN KEY (uid, to_account_id) REFERENCES accounts(uid, id),
            FOREIGN KEY (uid, from_account_id, from_ledger_id) REFERENCES ledgers(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN KEY (uid, to_account_id, to_ledger_id) REFERENCES ledgers(uid, aid, id) ON DELETE CASCADE ON UPDATE CASCADE,
            FOREIGN KEY (uid) REFERENCES users(id)
        )";

        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create: {}", error)
            }
        }

        Ok(())
    }

    pub fn add_account_transaction(&self, uid: u32, info: AccountTransaction) -> Result<u32> {
        let tid = self.get_next_transaction_id(uid).unwrap();
        let p = rusqlite::params![
            tid,
            info.from_account,
            info.to_account,
            info.from_ledger,
            info.to_ledger,
            uid
        ];
        let sql = "INSERT INTO account_transactions (id, from_account_id, to_account_id, from_ledger_id, to_ledger_id, uid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_) => Ok(tid),
            Err(error) => {
                panic!(
                    "Unable to add transaction between {} and {}!",
                    &info.from_account, &info.to_account
                );
            }
        }
    }

    pub fn check_and_get_account_transaction_record_matching_from_ledger_id(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
    ) -> rusqlite::Result<Option<AccountTransactionRecord>, rusqlite::Error> {
        let p = rusqlite::params![id, uid, aid];
        let sql = "SELECT id, from_account_id, to_account_id, from_ledger_id, to_ledger_id FROM account_transactions WHERE from_ledger_id = (?1) and from_account_id = (?3) and uid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(AccountTransactionRecord {
                        id: row.get(0)?,
                        info: AccountTransaction {
                            from_account: row.get(1)?,
                            to_account: row.get(2)?,
                            from_ledger: row.get(3)?,
                            to_ledger: row.get(4)?,
                        },
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }

    pub fn check_and_get_account_transaction_record_matching_to_ledger_id(
        &self,
        uid: u32,
        aid: u32,
        id: u32,
    ) -> rusqlite::Result<Option<AccountTransactionRecord>, rusqlite::Error> {
        let p = rusqlite::params![id, uid, aid];
        let sql = "SELECT id, from_account_id, to_account_id, from_ledger_id, to_ledger_id FROM account_transactions WHERE to_ledger_id = (?1) and to_account_id = (?3) and uid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;

                let record = stmt.query_row(p, |row| {
                    Ok(AccountTransactionRecord {
                        id: row.get(0)?,
                        info: AccountTransaction {
                            from_account: row.get(1)?,
                            to_account: row.get(2)?,
                            from_ledger: row.get(3)?,
                            to_ledger: row.get(4)?,
                        },
                    })
                });
                Ok(Some(record.unwrap()))
            }
            false => Ok(None),
        }
    }

    pub fn remove_account_transaction(
        &self,
        uid: u32,
        id: u32,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let p = rusqlite::params![id, uid];
        let sql = "DELETE FROM account_transactions WHERE id = ?1 and uid = ?2";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove account transaction: {}", error);
            }
        }

        let sql = "UPDATE account_transactions SET id = id-1 WHERE id > ?1 and uid = ?2";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!(
                    "Unable to update 'id' within account transactions: {}",
                    error
                );
            }
        }

        let p = rusqlite::params![uid];
        let sql = "UPDATE account_ids SET next_account_transaction_id = next_account_transaction_id-1 WHERE uid = ?1";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to update 'next_account_transaction_id': {}", error);
            }
        }

        Ok(id)
    }

    pub fn remove_account_transaction_matching_ledger_id(
        &self,
        uid: u32,
        ledger_id: u32,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let p = rusqlite::params![ledger_id, uid];
        let sql = "DELETE FROM account_transactions WHERE from_ledger_id = ?1 and uid = ?2 VALUES (?1, ?2)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove account transaction: {}", error);
            }
        }
        Ok(ledger_id)
    }

    pub fn get_user_accounts(&self, uid: u32) -> rusqlite::Result<Vec<AccountRecord>, Error> {
        let sql: &str = "SELECT * FROM accounts WHERE uid = (?1)";
        let p = rusqlite::params![uid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut accounts: Vec<AccountRecord> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let names: Vec<Result<AccountRecord, Error>> = stmt
                    .query_map(p, |row| {
                        Ok(AccountRecord {
                            id: row.get(0)?,
                            info: AccountInfo {
                                atype: AccountType::from_repr(row.get::<_, u32>(1)? as usize)
                                    .unwrap(),
                                name: row.get(2)?,
                                has_stocks: row.get(3)?,
                                has_bank: row.get(4)?,
                                has_ledger: row.get(5)?,
                                has_budget: row.get(6)?,
                            },
                        })
                    })
                    .unwrap()
                    .collect::<Vec<_>>();
                for name in names {
                    accounts.push(name.unwrap())
                }
                return Ok(accounts);
            }
            false => {
                return Ok(accounts);
            }
        }
    }

    pub fn get_user_accounts_by_type(
        &self,
        uid: u32,
        atype: AccountType,
    ) -> rusqlite::Result<Option<Vec<String>>, Error> {
        let sql: &str = "SELECT name FROM accounts WHERE uid = (?1) AND type = (?2)";
        let p = rusqlite::params![uid, atype as u32];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut accounts: Vec<String> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let names: Vec<Result<String, Error>> = stmt
                    .query_map(p, |row| Ok(row.get(0)?))
                    .unwrap()
                    .collect::<Vec<_>>();
                for name in names {
                    accounts.push(name.unwrap())
                }
                return Ok(Some(accounts));
            }
            false => return Ok(None),
        }
    }

    pub fn get_user_accounts_by_filter(
        &self,
        uid: u32,
        filter: AccountFilter,
    ) -> rusqlite::Result<Vec<String>, Error> {
        let sql: &str;
        match filter {
            AccountFilter::Bank => {
                sql = "SELECT name FROM accounts WHERE uid = (?1) and bank = TRUE";
            }
            AccountFilter::Budget => {
                sql = "SELECT name FROM accounts WHERE uid = (?1) and budget = TRUE";
            }
            AccountFilter::Stocks => {
                sql = "SELECT name FROM accounts WHERE uid = (?1) and stocks = TRUE";
            }
            _ => {
                sql = "SELECT name FROM accounts WHERE uid = (?1) and ledger = TRUE";
            }
        }

        let p = rusqlite::params![uid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut accounts: Vec<String> = Vec::new();
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let names: Vec<Result<String, Error>> = stmt
                    .query_map(p, |row| Ok(row.get(0)?))
                    .unwrap()
                    .collect::<Vec<_>>();
                for name in names {
                    accounts.push(name.unwrap())
                }
                return Ok(accounts);
            }
            false => {
                return Ok(accounts);
            }
        }
    }

    pub fn get_account_id(&self, uid: u32, aname: String) -> rusqlite::Result<u32, Error> {
        let sql: &str = "SELECT id from accounts WHERE name = (?1) AND uid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists((&aname, uid))?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let id = stmt.query_row((&aname, uid), |row| row.get::<_, u32>(0));
                match id {
                    Ok(id) => {
                        return Ok(id);
                    }
                    Err(err) => {
                        panic!("Unable to retrieve id for account {}: {}", &aname, err);
                    }
                }
            }
            false => {
                panic!("Unable to find account matching {}", aname);
            }
        }
    }
    pub fn get_account_name(&self, uid: u32, aid: u32) -> rusqlite::Result<String, Error> {
        let sql: &str = "SELECT name from accounts WHERE id = (?1) AND uid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists((&aid, uid))?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let rs = stmt.query_row((&aid, uid), |row| row.get::<_, String>(0));
                match rs {
                    Ok(name) => {
                        return Ok(name);
                    }
                    Err(err) => {
                        panic!("Unable to retrieve name for account {}: {}", &aid, err);
                    }
                }
            }
            false => {
                panic!("Unable to find account matching {}", aid);
            }
        }
    }
    pub fn get_account(&self, uid: u32, aid: u32) -> rusqlite::Result<AccountRecord, Error> {
        let sql: &str = "SELECT * from accounts WHERE id = (?1) and uid = (?2)";
        let p = rusqlite::params![aid, uid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = conn_lock.prepare(sql)?;
                let acct: Result<AccountRecord, Error> = stmt.query_row(p, |row| {
                    Ok(AccountRecord {
                        id: row.get(0)?,
                        info: AccountInfo {
                            atype: AccountType::from_repr(row.get::<_, u32>(1)? as usize).unwrap(),
                            name: row.get(2)?,
                            has_stocks: row.get(3)?,
                            has_bank: row.get(4)?,
                            has_ledger: row.get(5)?,
                            has_budget: row.get(6)?,
                        },
                    })
                });
                return acct;
            }
            false => {
                panic!("Unable to find account matching {}", aid);
            }
        }
    }

    pub fn create_user_account_info_table(&self) -> rusqlite::Result<()> {
        let sql = "CREATE TABLE IF NOT EXISTS user_account_info (
            uid     INTEGER NOT NULL,
            aid     INTEGER NOT NULL,
            spid    INTEGER NOT NULL,
            ssid    INTEGER NOT NULL,
            said    INTEGER NOT NULL,
            cid     INTEGER NOT NULL,
            pid     INTEGER NOT NULL,
            bid     INTEGER NOT NULL, 
            lid     INTEGER NOT NULL, 
            splid   INTEGER NOT NULL,
            ccid    INTEGER NOT NULL,
            cdid    INTEGER NOT NULL,
            stock_split_allocation_id INTEGER NOT NULL,
            label_allocation_id INTEGER NOT NULL,
            roth_ira_id INTEGER NOT NULL,
            hsa_id INTEGER NOT NULL, 
            PRIMARY KEY(uid, aid)
            FOREIGN KEY(uid) REFERENCES users(id)
            FOREIGN KEY(uid,aid) REFERENCES accounts(uid, id) ON DELETE CASCADE ON UPDATE CASCADE
        )";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create account_ids table: {}", error)
            }
        }
        Ok(())
    }

    pub fn initialize_user_account_info_table(&self, uid: u32, aid: u32) -> rusqlite::Result<()> {
        let p = rusqlite::params![uid, aid, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let sql: &str = "
            INSERT INTO user_account_info 
                (uid, aid, spid, ssid, said, cid, pid, bid, lid, splid, ccid, cdid, stock_split_allocation_id, label_allocation_id, roth_ira_id, hsa_id) 
            VALUES 
                ( ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        ";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!(
                    "Unable to intialize user account info for user {} account {}:\n\t{}",
                    uid,
                    self.get_account_name(uid, aid).unwrap(),
                    error
                );
            }
        }
        Ok(())
    }

    pub fn get_next_stock_purchase_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT spid FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET spid = spid + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!(
                    "The next stock purchase ID within table 'user_account_info' does not exist."
                );
            }
        }
    }

    pub fn get_next_stock_sale_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT ssid FROM user_account_info  WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET ssid = ssid + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next stock sale ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_stock_sale_allocation_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT said FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET said = said + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next stock sale allocation ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_category_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT cid FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET cid = cid + 1  WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next category ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_people_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT pid FROM user_account_info  WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET pid = pid + 1  WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next people ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_budget_item_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT bid FROM user_account_info  WHERE uid = (?1) and aid = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let p = rusqlite::params![uid, aid];
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET bid = bid + 1  WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next budget ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_ledger_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT lid FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET lid = lid + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next ledger ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_stock_split_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT splid FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE user_account_info SET splid = splid + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next stock split ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_stock_split_allocation_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT stock_split_allocation_id FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE user_account_info SET stock_split_allocation_id = stock_split_allocation_id + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next stock split ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_label_allocation_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT label_allocation_id FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE user_account_info SET label_allocation_id = label_allocation_id + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next label allocation ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_credit_card_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT ccid FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET ccid = ccid + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next credit card ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_roth_ira_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT roth_ira_id FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET roth_ira_id = roth_ira_id + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next Roth IRA ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_hsa_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT hsa_id FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET hsa_id = hsa_id + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next Roth IRA ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn get_next_certificate_of_deposit_id(&self, uid: u32, aid: u32) -> rusqlite::Result<u32> {
        let sql = "SELECT cdid FROM user_account_info WHERE uid = (?1) and aid = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0))?;
                let sql =
                    "UPDATE user_account_info SET cdid = cdid + 1 WHERE uid = (?1) and aid = (?2)";
                conn_lock.execute(sql, p)?;
                Ok(id)
            }
            false => {
                panic!("The next certificate of deposit ID within table 'user_account_info' does not exist.");
            }
        }
    }

    pub fn remove_account(&self, uid: u32, aid: u32) -> rusqlite::Result<u32, rusqlite::Error> {
        let sql = "DELETE FROM accounts WHERE uid = (?1) and id = (?2)";
        let p = rusqlite::params![uid, aid];
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!("Unable to remove account: {}!", error);
            }
        }
        let sql = "UPDATE accounts SET id = id -1 WHERE id > (?2) and uid = (?1)";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!("Unable to remove account: {}!", error);
            }
        }

        let p = rusqlite::params![uid];
        let sql = "UPDATE account_ids SET next_account_id = next_account_id -1 WHERE uid = (?1)";
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!("Unable to remove account: {}!", error);
            }
        }
        Ok(aid)
    }

    pub fn rename_account(
        &self,
        uid: u32,
        aid: u32,
        new_name: String,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let p = rusqlite::params![uid, aid, new_name];
        let sql = "UPDATE accounts SET name = (?3) WHERE uid = (?1) and id = (?2)";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                panic!("Unable to name account: {}!", error);
            }
        }
        Ok(aid)
    }
}
