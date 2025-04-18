use crate::database::DbConn;
use rusqlite::{Error, Result};

#[derive(Clone, Copy)]
pub enum AccountType {
    Ledger,
    Investment,
    Bank,
    CD,
    Retirement,
    Health,
    Custom,
}

pub enum AccountFilter {
    Stocks,
    Bank,
    Ledger,
    Budget,
}

impl From<u32> for AccountType {
    fn from(value: u32) -> Self {
        match value {
            0 => AccountType::Ledger,
            1 => AccountType::Investment,
            2 => AccountType::Bank,
            3 => AccountType::CD,
            4 => AccountType::Retirement,
            5 => AccountType::Health,
            6 => AccountType::Custom,
            _ => panic!("Invalid numberic value for AccountType!"),
        }
    }
}
impl From<String> for AccountType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "Ledger" => AccountType::Ledger,
            "Investment" => AccountType::Investment,
            "Bank" => AccountType::Bank,
            "CD" => AccountType::CD,
            "Retirement" => AccountType::Retirement,
            "Health" => AccountType::Health,
            "Custom" => AccountType::Custom,
            _ => panic!("Invalid string type for AccountType!"),
        }
    }
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
    pub fn create_accounts_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS accounts (
                id   INTEGER NOT NULL PRIMARY KEY, 
                type INTEGER NOT NULL, 
                name TEXT NOT NULL,
                stocks BOOL NOT NULL,
                bank   BOOL NOT NULL,
                ledger BOOL NOT NULL,
                budget BOOL NOT NULL,
                uid  INTEGER,
                FOREIGN KEY (uid) REFERENCES users(id)
            )";
        let rs = self.conn.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create: {}", error)
            }
        }
        Ok(())
    }

    pub fn create_account_transaction_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS account_transactions (
            id              INTEGER NOT NULL PRIMARY KEY,
            from_account_id INTEGER NOT NULL, 
            from_ledger_id  INTEGER NOT NULL, 
            to_account_id   INTEGER NOT NULL,
            to_ledger_id    INTEGER NOT NULL, 
            uid             INTEGER NOT NULL, 
            FOREIGN KEY (from_account_id) REFERENCES accounts(id),
            FOREIGN KEY (to_account_id) REFERENCES accounts(id),
            FOREIGN KEY (from_ledger_id) REFERENCES ledgers(id),
            FOREIGN KEY (to_ledger_id) REFERENCES ledgers(id)
            FOREIGN KEY (uid) REFERENCES users(id)
        )";

        let rs = self.conn.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create: {}", error)
            }
        }

        Ok(())
    }

    pub fn add_account(&mut self, uid: u32, info: &AccountInfo) -> Result<u32> {
        let aid = self.get_next_account_id().unwrap();
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
        let sql: &str = "SELECT * FROM accounts WHERE uid = (?1) and name = (?2)";
        let exists = self
            .conn
            .prepare(sql)?
            .exists(rusqlite::params![uid, info.name])?;
        if exists {
            panic!("Account with name {} already exists for user!", info.name);
        }
        let sql: &str = "INSERT INTO accounts (id, type, name, stocks, bank, ledger, budget, uid) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_) => Ok(aid),
            Err(error) => {
                panic!(
                    "Unable to add account {} for user {}: {}!",
                    &info.name, &uid, error
                );
            }
        }
    }

    pub fn add_account_transaction(&mut self, uid : u32, info: AccountTransaction) -> Result<u32> {
        let tid = self.get_next_transaction_id().unwrap();
        let p = rusqlite::params![
            tid,
            info.from_account,
            info.to_account,
            info.from_ledger,
            info.to_ledger,
            uid
        ];
        let sql = "INSERT INTO account_transactions (id, from_account_id, to_account_id, from_ledger_id, to_ledger_id, uid) VALUES (?1, ?2, ?3, ?4, ?5, ?6)";
        let rs = self.conn.execute(sql, p);
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
        &mut self,
        uid : u32,
        id: u32,
    ) -> rusqlite::Result<Option<AccountTransactionRecord>, rusqlite::Error> {
        let p = rusqlite::params![id,  uid];
        let sql = "SELECT * FROM account_transactions WHERE from_ledger_id = (?1) and uid = (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;

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
        &mut self,
        uid : u32, 
        id: u32,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let p = rusqlite::params![id,uid];
        let sql = "DELETE FROM account_transactions WHERE id = ?1 and uid = ?2 VALUES (?1, ?2)";
        let rs = self.conn.execute(sql, p);
        match rs {
            Ok(_usize) => {}
            Err(error) => {
                println!("Unable to remove account transaction: {}", error);
            }
        }
        Ok(id)
    }

    pub fn get_user_account_info(
        &mut self,
        uid: u32,
    ) -> rusqlite::Result<Vec<AccountRecord>, Error> {
        let sql: &str = "SELECT * FROM accounts WHERE uid = (?1)";
        let p = rusqlite::params![uid];
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut accounts: Vec<AccountRecord> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let names: Vec<Result<AccountRecord, Error>> = stmt
                    .query_map(p, |row| {
                        Ok(AccountRecord {
                            id: row.get(0)?,
                            info: AccountInfo {
                                atype: AccountType::from(row.get::<_, u32>(1)? as u32),
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
        &mut self,
        uid: u32,
        atype: AccountType,
    ) -> rusqlite::Result<Vec<String>, Error> {
        let sql: &str = "SELECT name FROM accounts WHERE uid = (?1) AND type = (?2)";
        let p = rusqlite::params![uid, atype as u32];
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut accounts: Vec<String> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
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
                panic!("Account not found!");
            }
        }
    }

    pub fn get_user_accounts_by_filter(
        &mut self,
        uid: u32,
        filter: AccountFilter,
    ) -> rusqlite::Result<Vec<String>, Error> {
        let mut sql: &str;
        match filter {
            AccountFilter::Bank => {
                sql = "SELECT name FROM accounts WHERE uid = (?1) and bank = TRUE";
            }
            AccountFilter::Budget => {
                sql = "SELECT name FROM accounts WHERE uid = (?1) and budget = TRUE";
            }
            AccountFilter::Ledger => {
                sql = "SELECT name FROM accounts WHERE uid = (?1) and ledger = TRUE";
            }
            AccountFilter::Stocks => {
                sql = "SELECT name FROM accounts WHERE uid = (?1) and stocks = TRUE";
            }
        }

        let p = rusqlite::params![uid];
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut accounts: Vec<String> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
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

    pub fn get_account_id(&mut self, uid: u32, aname: String) -> rusqlite::Result<u32, Error> {
        let sql: &str = "SELECT id from accounts WHERE name = (?1) AND uid = (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists((&aname, uid))?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
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
    pub fn get_account_name(&mut self, uid: u32, aid: u32) -> rusqlite::Result<String, Error> {
        let sql: &str = "SELECT name from accounts WHERE id = (?1) AND uid = (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists((&aid, uid))?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
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
    pub fn get_account(&mut self, uid : u32, aid: u32, ) -> rusqlite::Result<AccountRecord, Error> {
        let sql: &str = "SELECT * from accounts WHERE id = (?1) and uid = (?2)";
        let p = rusqlite::params![aid, uid];
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let acct: Result<AccountRecord, Error> = stmt.query_row(p, |row| {
                    Ok(AccountRecord {
                        id: row.get(0)?,
                        info: AccountInfo {
                            atype: AccountType::from(row.get::<_, u32>(1)?),
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
}
