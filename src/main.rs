// use core::panic;

use crate::ledger::Ledger;
// use crate::tui::*;
// use crate::ledger::LedgerEntry;
// use chrono::{NaiveDate, Weekday};
// use inquire::*;

mod ledger;
mod tui;

fn main() {

    let mut _ledger: Ledger = Ledger::new();
    tui::tui(&mut _ledger);
    
}
