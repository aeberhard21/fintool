use ratatui::{text::Line, Frame, layout::Rect};
use strum::FromRepr;
pub enum CurrentScreen { 
    Login,
    Main,
    Accounts
}

#[derive(Debug, Clone, Copy, FromRepr)]
pub enum CurrentlySelecting {
    AccountTypeTabs, 
    AccountTabs,
    Account
}

pub trait TabMenu { 
    fn previous(self) -> Self;
    fn next(self) -> Self;
    fn to_tab_title(value : Self) -> Line<'static>;
    fn render(frame: &mut Frame, area : Rect, selected_tab : usize, title : String);
}

impl CurrentlySelecting { 
    pub fn previous(self) -> Self { 
        let current = self.clone() as usize;
        let prev = current.saturating_sub(1);
        Self::from_repr(prev).unwrap_or(self)
    }
    pub fn next(self) -> Self { 
        let current = self.clone() as usize;
        let next = current.saturating_add(1);
        Self::from_repr(next).unwrap_or(self)
    }
}