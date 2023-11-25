use rusqlite::Error;

use crate::ledger::Ledger;
use crate::database::DbConn;

mod ledger;
mod tui;
mod database;

fn main() {

    let mut _db : DbConn  = DbConn::new("../db/checking.db").unwrap();
    let mut _ledger: Ledger = Ledger::new("checking", &mut _db);
    tui::menu(&mut _db,&mut _ledger);
    _db.close();

    // save_to_db(_db, &mut _ledger);
}


// fn save_to_db(_db: DbConn, _ledger: &mut Ledger) {
//     _db.save_ledger();
// }