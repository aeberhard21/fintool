use crate::ledger::{Ledger, LedgerEntry};
use crate::database::*;

pub struct User<'a>{
    id      : u32,
    name    : String,
    ledgers : Vec<String>,
    is_admin: bool,
    db      : &'a mut DbConn,
}

impl User<'_> {
    pub fn new<'a>(id: u32, name: &str, admin: bool, db: &'a mut DbConn) -> User<'a> {
        User {
            id          : id,
            ledgers     : Vec::new(),
            name        : name.to_string(),
            is_admin    : admin,
            db          : db,
        }
    }
    // pub fn set_db(mut self, db: &mut DbConn) {
    //     self.db = db;
    // }

    // create a ledger 
    pub fn create_ledger(&mut self, name: String) {
        self.ledgers.push(name.to_string());
        self.db.create_ledger(name);
    }

    pub fn add_ledger_entry(&mut self, ledger: String, entry: LedgerEntry) {
        self.db.add_ledger_entry(ledger, entry);
    }

    pub fn get_name(self,) -> String {
        return self.name;
    }

    pub fn get_id(self) -> u32 {
        return self.id;
    }

    pub fn get_ledgers(&mut self) -> Vec<String> {
        self.ledgers.clone()
    }
}