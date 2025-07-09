use ratatui::widgets::{ScrollbarState, TableState};

use crate::accounts::base::AnalysisPeriod;
use crate::app::screen::PALETTES;
use crate::database::DbConn;
use crate::tui::decode_and_init_account_type;
use crate::types::accounts::AccountType;
use crate::types::ledger::{DisplayableLedgerRecord, LedgerRecord};
use crate::{accounts::base::Account, app::screen::TabMenu};

use super::screen::{CurrentScreen, CurrentlySelecting, LedgerColors};

const ITEM_HEIGHT: usize = 2;

pub struct App {
    pub key_input: String,
    pub invalid_input: bool,
    pub current_screen: CurrentScreen,
    pub selected_atype_tab: AccountType,
    pub accounts_for_type: Option<Vec<String>>,
    pub selected_account_tab: usize,
    pub currently_selected: Option<CurrentlySelecting>,
    pub db: DbConn,
    pub user_id: Option<u32>,
    pub account: Option<Box<dyn Account>>,
    pub ledger_table_state: TableState,
    pub ledger_table_colors: LedgerColors,
    pub ledger_entries: Option<Vec<DisplayableLedgerRecord>>,
    pub analysis_period: AnalysisPeriod,
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
            currently_selected: Some(CurrentlySelecting::AccountTypeTabs),
            db: db.clone(),
            user_id: None,
            account: None,
            ledger_table_state: TableState::default().with_selected(0),
            ledger_table_colors: LedgerColors::new(&PALETTES[1]),
            ledger_entries: None,
            analysis_period: AnalysisPeriod::YTD,
        }
    }

    pub fn advance_currently_selecting(&mut self) {
        if let Some(selecting) = &self.currently_selected {
            self.currently_selected = Some(selecting.next());
        } else {
            self.currently_selected = Some(CurrentlySelecting::AccountTabs)
        }
    }

    pub fn retreat_currently_selecting(&mut self) {
        if let Some(selecting) = &self.currently_selected {
            self.currently_selected = Some(selecting.previous());
        } else {
            self.currently_selected = Some(CurrentlySelecting::AccountTabs)
        }
    }

    pub fn advance_account_type(&mut self) {
        self.selected_atype_tab = self.selected_atype_tab.next();
    }

    pub fn retreat_account_type(&mut self) {
        self.selected_atype_tab = self.selected_atype_tab.previous();
    }

    pub fn advance_account(&mut self) {
        if self.accounts_for_type.is_none() {
            return;
        }
        self.selected_account_tab = self
            .selected_account_tab
            .saturating_add(1)
            .min(self.accounts_for_type.clone().unwrap().len() - 1)
    }

    pub fn retreat_account(&mut self) {
        if self.accounts_for_type.is_none() {
            return;
        }
        self.selected_account_tab = self.selected_account_tab.saturating_sub(1).min(0)
    }

    pub fn skip_to_last_account(&mut self) {
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
        let account_id = self
            .db
            .get_account_id(
                self.user_id.unwrap(),
                self.accounts_for_type.clone().unwrap()[self.selected_account_tab].clone(),
            )
            .unwrap();
        let acct = self
            .db
            .get_account(self.user_id.unwrap(), account_id)
            .unwrap();
        let account = decode_and_init_account_type(self.user_id.unwrap(), &self.db, &acct);
        self.account = Some(account);
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
