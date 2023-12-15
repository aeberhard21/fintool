use crate::database::*;
use crate::ledger::{Ledger, LedgerEntry};

#[derive(Clone)]
// pub struct User<'a>{
pub struct User {
    pub id: u32,
    pub name: String,
    pub ledgers: Vec<String>,
    pub is_admin: bool,
    // pub db      : Option<&'a mut DbConn>,
}

// impl User<'_> {
impl User {
    // pub fn new<'a>(id: u32, name: &str, admin: bool, db: &'a mut DbConn) -> User<'a> {
    pub fn new(id: u32, name: &str, admin: bool) -> User {
        User {
            id: id,
            ledgers: Vec::new(),
            name: name.to_string(),
            is_admin: admin,
            //db          : Some(db),
        }
    }
    // pub fn set_db(mut self, db: &mut DbConn) {
    //     self.db = db;
    // }

    // create a ledger
    pub fn create_ledger(&mut self, db: &mut DbConn, name: String) {
        self.ledgers.push(name.to_string());
        println!("number of ledgers is now: {}", self.get_ledgers().len());
        db.create_ledger(name);
    }

    pub fn add_ledger_entry(&mut self, ledger: String, db: &mut DbConn, entry: LedgerEntry) {
        db.add_ledger_entry(ledger, entry);
    }

    pub fn get_name(&mut self) -> String {
        return String::from(self.name.as_str());
    }

    pub fn get_id(&mut self) -> u32 {
        return self.id;
    }

    pub fn get_ledgers(&mut self) -> Vec<String> {
        self.ledgers.clone()
    }

    pub fn print_ledger(&mut self, db: &mut DbConn, name: String) {
        db.read_ledger(name);
    }

    pub fn get_admin(&mut self) -> bool {
        return self.is_admin;
    }
}
