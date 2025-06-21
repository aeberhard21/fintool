use rusqlite::config::DbConfig;
#[cfg(feature = "ratatui_support")]
use ratatui::{
    layout::Rect,
    Frame
};
#[cfg(feature = "ratatui_support")]
use crate::app::app::App;
use crate::database::DbConn;
use crate::types::accounts::AccountRecord;
use crate::types::ledger::LedgerRecord;

pub mod fixed_account;
pub mod variable_account;
pub mod charge_account;

pub trait AccountCreation {
    fn create(uid : u32, name: String, _db : &DbConn) -> AccountRecord;
}

pub trait AccountOperations {
    // fn create( account_id : u32, db : &mut DbConn );
    fn import(&self);
    fn record(&self);
    fn modify(&self);
    fn export(&self);
    fn report(&self);
    fn link(&self, transacting_account: u32, ledger: LedgerRecord) -> Option<u32>;
}

pub trait AccountData {
    fn get_id(&self) -> u32;
    fn get_name(&self) -> String;
}
#[cfg(feature = "ratatui_support")]
pub trait AccountUI { 
    fn render(&self, frame: &mut Frame, area : Rect, app: &App);
}

#[cfg(not(feature = "ratatui_support"))]
pub trait Account: AccountData + AccountOperations {}

#[cfg(feature = "ratatui_support")]
pub trait Account: AccountData + AccountOperations + AccountUI {}