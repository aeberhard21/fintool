use chrono::{Datelike, Local, NaiveDate};
use ratatui::widgets::{ScrollbarState, TableState};

use crate::{accounts, is_account_type};
use crate::accounts::base::AnalysisPeriod;
use crate::app::screen::PALETTES;
use crate::database::DbConn;
use crate::tui::decode_and_init_account_type;
use crate::types::accounts::AccountType;
use crate::types::ledger::{DisplayableLedgerRecord, LedgerRecord};
use crate::{accounts::base::Account, app::screen::TabMenu};

use super::screen::{CurrentScreen, CurrentlySelecting, LedgerColors, UserLoadedState};

const ITEM_HEIGHT: usize = 2;

pub struct App {
    pub key_input: String,
    pub invalid_input: bool,
    pub current_screen: CurrentScreen,
    pub selected_atype_tab: AccountType,
    pub accounts_for_type: Option<Vec<String>>,
    pub selected_account_tab: usize,
    pub account_index_to_restore : usize,
    pub currently_selected: Option<CurrentlySelecting>,
    pub db: DbConn,
    pub user_id: Option<u32>,
    pub account: Option<Box<dyn Account>>,
    pub accounts: Option<Vec<Box<dyn Account>>>,
    pub ledger_table_state: TableState,
    pub ledger_table_colors: LedgerColors,
    pub ledger_entries: Option<Vec<DisplayableLedgerRecord>>,
    pub analysis_period: AnalysisPeriod,
    pub analysis_start : NaiveDate, 
    pub analysis_end : NaiveDate,
    pub user_load_state : UserLoadedState,
    pub load_profile_progress : f64,
}

impl App {
    pub fn new(db: &DbConn) -> App {
        App {
            key_input: String::new(),
            invalid_input: false,
            current_screen: CurrentScreen::Login,
            selected_atype_tab: AccountType::Bank,
            accounts_for_type: None,
            selected_account_tab: 0,
            account_index_to_restore: 0,
            currently_selected: Some(CurrentlySelecting::AccountTypeTabs),
            db: db.clone(),
            user_id: None,
            account: None,
            accounts : None,
            ledger_table_state: TableState::default().with_selected(0),
            ledger_table_colors: LedgerColors::new(&PALETTES[1]),
            ledger_entries: None,
            analysis_period: AnalysisPeriod::YTD,
            analysis_start : NaiveDate::from_ymd_opt(Local::now().year(), 1, 1).unwrap(),
            analysis_end : Local::now().date_naive(),
            user_load_state : UserLoadedState::NotLoaded,
            load_profile_progress : 0.0,
        }
    }

    pub fn advance_currently_selecting(&mut self) {
        // self.restore_account();
        if let Some(selecting) = &self.currently_selected {
            self.currently_selected = Some(selecting.next());
        } else {
            self.currently_selected = Some(CurrentlySelecting::AccountTabs)
        }
    }

    pub fn retreat_currently_selecting(&mut self) {
        self.restore_account();
        if let Some(selecting) = &self.currently_selected {
            self.currently_selected = Some(selecting.previous());
        } else {
            self.currently_selected = Some(CurrentlySelecting::AccountTabs)
        }
    }

    pub fn advance_account_type(&mut self) {
        self.restore_account();
        self.selected_atype_tab = self.selected_atype_tab.next();
    }

    pub fn retreat_account_type(&mut self) {
        self.restore_account();
        self.selected_atype_tab = self.selected_atype_tab.previous();
    }

    pub fn advance_account(&mut self) {
        self.restore_account();
        if self.accounts_for_type.is_none() {
            return;
        }
        self.selected_account_tab = self
            .selected_account_tab
            .saturating_add(1)
            .min(self.accounts_for_type.clone().unwrap().len() - 1)
    }

    pub fn retreat_account(&mut self) {
        self.restore_account();
        if self.accounts_for_type.is_none() {
            return;
        }
        self.selected_account_tab = self.selected_account_tab.saturating_sub(1).max(0)
    }

    pub fn skip_to_last_account(&mut self) {
        self.restore_account();
        if self.accounts_for_type.is_none() {
            return;
        }
        self.selected_account_tab = self.accounts_for_type.clone().unwrap().len() - 1
    }

    pub fn validate_user(&mut self, username: String) -> Option<u32> {
        let users: Vec<String> = self.db.get_users().unwrap();
        if users.contains(&username) {
            return Some(self.db.get_user_id(username).unwrap());
        } else {
            return None;
        }
    }

    pub fn get_account(&mut self) {
        self.account = if let Some(accounts) = &mut self.accounts { 
            let matching_indexes : Vec<usize> = accounts
                .iter()
                .enumerate()
                .filter(|(_, acc)| is_account_type(acc, self.selected_atype_tab))
                .map(|(i, _)| i)
                .collect();
            if let Some(&original_index) = matching_indexes.get(self.selected_account_tab) {
                let account = accounts.remove(original_index);
                self.account_index_to_restore = original_index;
                Some(account)
            } else {
                None
            }
        } else { 
            None
        };
    }

    pub fn restore_account(&mut self) {
        if let (Some(account), index) = (self.account.take(), self.account_index_to_restore) {
            // self.accounts.insert(index, account); // ✅ restores order
            if let Some(mut accounts) = self.accounts.take() { 
                accounts.insert(index, account);
                self.accounts = Some(accounts);
            } else { 
                self.accounts = None
            }
        }
    }

    pub fn advance_ledger_table_row(&mut self) {
        let i = match self.ledger_table_state.selected() {
            Some(i) => Some(
                i.saturating_add(1)
                    .min(self.ledger_entries.clone().unwrap().len() - 1),
            ),
            None => Some(0),
        };
        self.ledger_table_state.select(i);
        // set scroll position
    }

    pub fn go_to_last_ledger_table_row(&mut self) {
        self.ledger_table_state
            .select(Some(self.ledger_entries.clone().unwrap().len() - 1));
        // set scroll position
    }

    pub fn go_to_first_ledger_table_row(&mut self) {
        self.ledger_table_state.select(Some(0));
        // set scroll position
    }

    pub fn retreat_ledger_table_row(&mut self) {
        let i = match self.ledger_table_state.selected() {
            Some(i) => Some(i.saturating_sub(1)),
            None => Some(0),
        };
        self.ledger_table_state.select(i);
        // set scroll position
    }
}
