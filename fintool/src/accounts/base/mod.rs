use crate::types::accounts::AccountInfo;
use crate::types::ledger::LedgerRecord;

pub mod fixed_account;
pub mod variable_account;


pub trait AccountCreation {
    fn create() -> AccountInfo;
}

pub trait AccountOperations {
    // fn create( account_id : u32, db : &mut DbConn );
    fn import(&mut self);
    fn record(&mut self);
    fn modify(&mut self);
    fn export(&mut self);
    fn report(&mut self);
    fn link(&mut self, transacting_account: u32, ledger: LedgerRecord) -> Option<u32>;
}
