use crate::tui::decode_and_init_account_type;
use crate::{accounts::base::Account, app::screen::TabMenu};
use crate::database::DbConn;
use crate::types::accounts::AccountType;

use super::screen::{CurrentlySelecting, CurrentScreen};

pub struct App { 
    pub key_input : String, 
    pub invalid_input : bool,
    pub current_screen : CurrentScreen,
    pub selected_atype_tab : AccountType,
    pub accounts_for_type: Option<Vec<String>>,
    pub selected_account_tab : usize,
    pub currently_selected : Option<CurrentlySelecting>,
    pub db : DbConn,
    pub user_id : Option<u32>, 
    pub account : Option<Box<dyn Account>>,
}

impl App { 
    pub fn new(db: &DbConn) -> App { 
        App { 
            key_input : String::new(), 
            invalid_input : false,
            current_screen: CurrentScreen::Login, 
            selected_atype_tab : AccountType::Bank,
            accounts_for_type : None, 
            selected_account_tab : 0, 
            currently_selected : Some(CurrentlySelecting::AccountTypeTabs),
            db : db.clone(),
            user_id : None,
            account : None,
        }
    }

    pub fn advance_currently_selecting(&mut self) { 
        if let Some(selecting) = &self.currently_selected { 
            self.currently_selected =  Some(selecting.next());
        } else { 
            self.currently_selected = Some(CurrentlySelecting::AccountTabs)
        }
    }

    pub fn retreat_currently_selecting(&mut self) { 
        if let Some(selecting) = &self.currently_selected { 
            self.currently_selected =  Some(selecting.previous());
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
            return
        }
        self.selected_account_tab = self.selected_account_tab.saturating_add(1).min(self.accounts_for_type.clone().unwrap().len()-1)
    }

    pub fn retreat_account(&mut self) {
        if self.accounts_for_type.is_none() { 
            return
        }
        self.selected_account_tab = self.selected_account_tab.saturating_sub(1).min(0)
    }

    pub fn skip_to_last_account(&mut self) { 
        if self.accounts_for_type.is_none() { 
            return
        }
        self.selected_account_tab = self.accounts_for_type.clone().unwrap().len()-1
    }

    pub fn validate_user(&mut self, username : String) -> Option<u32> {
        println!("ATTEMPTING TO VALDIATE user!");
        let users = self.db.get_users().unwrap();
        if users.contains(&username) { 
            return Some(self.db.get_user_id(username).unwrap());
        } else {
            return None;
        }
    }

    pub fn get_account(&mut self) {
        let account_id = self.db.get_account_id(self.user_id.unwrap(), self.accounts_for_type.clone().unwrap()[self.selected_account_tab].clone()).unwrap();
        let acct = self.db.get_account(self.user_id.unwrap(), account_id).unwrap();
        self.account = Some(decode_and_init_account_type(self.user_id.unwrap(), &mut self.db, &acct));
    }

}