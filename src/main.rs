use crate::ledger::Ledger;

mod ledger;
mod tui;

fn main() {

    let mut _ledger: Ledger = Ledger::new();
    tui::menu(&mut _ledger);
    
}
