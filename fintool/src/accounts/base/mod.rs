use rusqlite::config::DbConfig;
#[cfg(feature = "ratatui_support")]
use ratatui::{
    layout::Rect,
    Frame
};
use strum::{Display, EnumIter, EnumString, FromRepr};
#[cfg(feature = "ratatui_support")]
use crate::app::app::App;
use crate::database::DbConn;
use crate::types::accounts::AccountRecord;
use crate::types::ledger::{DisplayableLedgerRecord, LedgerRecord};

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
    fn get_ledger(&self) -> Vec<LedgerRecord>;
    fn get_displayable_ledger(&self) -> Vec<DisplayableLedgerRecord>;
    fn get_value(&self) -> f32;
}
#[cfg(feature = "ratatui_support")]
pub trait AccountUI { 
    fn render(&self, frame: &mut Frame, area : Rect, app: &mut App);
    fn render_ledger_table(&self, frame : &mut Frame, area : Rect, app: &mut App);
    fn render_current_value(&self, frame : &mut Frame, area : Rect, app: &mut App);
}

#[cfg(not(feature = "ratatui_support"))]
pub trait Account: AccountData + AccountOperations {}

#[cfg(feature = "ratatui_support")]
pub trait Account: AccountData + AccountOperations + AccountUI {}

#[derive(Display, Debug, FromRepr, EnumIter, EnumString)]
pub enum AnalysisPeriod { 
    #[strum(to_string = "1 Day")]
    OneDay,
    #[strum(to_string = "1 Week")]
    OneWeek, 
    #[strum(to_string = "1 Month")]
    OneMonth, 
    #[strum(to_string = "3 Months")]
    ThreeMonths, 
    #[strum(to_string = "6 Months")]
    SixMonths, 
    #[strum(to_string = "1 Year")]
    OneYear, 
    #[strum(to_string = "2 Years")]
    TwoYears, 
    #[strum(to_string = "5 Years")]
    FiveYears,
    #[strum(to_string = "10 Years")]
    TenYears, 
    #[strum(to_string = "YTD")]
    YTD, 
    #[strum(to_string = "Custom")]
    Custom
}

impl AnalysisPeriod {
    pub fn to_menu_selection(value : Self) -> String { 
        format!("{value}")
    }
}