use crate::ledger::Ledger;
use crate::database::DbConn;

mod ledger;
mod tui;
mod database;

fn main() {

    let db = DbConn::new("../db");
    let mut _ledger: Ledger = Ledger::new();
    tui::menu(&mut _ledger);
    
}
