use crate::ledger::Ledger;
use crate::ledger::LedgerEntry;

mod ledger;

fn main() {
    println!("Hello, world!");
    let mut _ledger: Ledger = Ledger::new();
    let entry = LedgerEntry {
        date: String::from("November 9, 2022"), 
        amount: 32000.42,
        deposit: false, 
        description: String::from("car payment")
    };
    _ledger.add(entry);
    println!("My new ledger sum is {0}", _ledger.sum());
}
