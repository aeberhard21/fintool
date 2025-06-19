use rusqlite::config::DbConfig;
#[cfg(feature = "ratatui_support")]
use ratatui::{
    layout::Rect,
    Frame
};

use crate::app::app::App;
use crate::database::DbConn;
use crate::types::accounts::AccountRecord;
use crate::types::ledger::LedgerRecord;

pub mod fixed_account;
pub mod variable_account;
pub mod charge_account;

pub trait AccountCreation {
    fn create(uid : u32, name: String, _db : &mut DbConn) -> AccountRecord;
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

pub trait AccountData {
    fn get_id(&mut self) -> u32;
}

pub trait AccountUI { 
    fn render(&self, frame: &mut Frame, area : Rect, app: &App);
}

#[cfg(not(feature = "ratatui_support"))]
pub trait Account: AccountData + AccountOperations {}

#[cfg(feature = "ratatui_support")]
pub trait Account: AccountData + AccountOperations + AccountUI {}