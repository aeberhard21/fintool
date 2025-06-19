use ratatui::{text::Line, Frame, layout::Rect};
pub enum CurrentScreen { 
    Login,
    Main,
    Accounts
}

pub enum TabBankSelected {
    AccountTypeTabs, 
    AccountTabs
}

pub trait TabMenu { 
    fn previous(self) -> Self;
    fn next(self) -> Self;
    fn to_tab_title(value : Self) -> Line<'static>;
    fn render(frame: &mut Frame, area : Rect, selected_tab : usize, title : String);
}

// impl TabMenu for Option<Vec<String>> {

//     fn next(&mut self) {
//         if self.is_none() { 
//             return
//         }
//         self = self.selected_account.saturating_add(1).min(self.accounts.clone().unwrap().len()-1)
//     }

//     fn previous (&mut self) {
//         if self.accounts.is_none() { 
//             return
//         }
//         self.selected_account = self.selected_account.saturating_sub(1).min(0)
//     }
// }