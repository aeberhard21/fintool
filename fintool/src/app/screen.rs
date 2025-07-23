use ratatui::{layout::Rect, style::palette::tailwind, style::Color, text::Line, Frame};
use shared_lib::LedgerEntry;
use strum::FromRepr;
use unicode_width::UnicodeWidthStr;

use crate::types::ledger::DisplayableLedgerRecord;

pub const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

pub enum CurrentScreen {
    Login,
    Main,
    Accounts,
}

#[derive(Debug, Clone, Copy, FromRepr)]
pub enum CurrentlySelecting {
    AccountTypeTabs,
    AccountTabs,
    Account,
}

#[derive(Debug, Clone, Copy, FromRepr, PartialEq, Eq)]
pub enum UserLoadedState {
    NotLoaded,
    Loading,
    Loaded,
}

pub trait TabMenu {
    fn previous(self) -> Self;
    fn next(self) -> Self;
    fn to_tab_title(value: Self) -> Line<'static>;
    fn render(frame: &mut Frame, area: Rect, selected_tab: usize, title: String);
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

// all table functions copied from table.rs ratatui example
pub struct LedgerColors {
    pub buffer_bg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub row_fg: Color,
    pub selected_row_style_fg: Color,
    pub selected_column_style_fg: Color,
    pub selected_cell_style_fg: Color,
    pub normal_row_color: Color,
    pub alt_row_color: Color,
    pub footer_border_color: Color,
}

impl LedgerColors {
    pub const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

pub fn ledger_table_constraint_len_calculator(
    entries: &[DisplayableLedgerRecord],
) -> (u16, u16, u16, u16, u16, u16, u16) {
    let id_len = entries
        .iter()
        .map(|d| d.id.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let date_len = entries
        .iter()
        .map(|d| d.info.date.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let amt_len = entries
        .iter()
        .map(|d| d.info.amount.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let type_len = entries
        .iter()
        .map(|d| d.info.transfer_type.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let cat_len = entries
        .iter()
        .map(|d| d.info.category.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let peer_len = entries
        .iter()
        .map(|d| d.info.participant.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let desc_len = entries
        .iter()
        .map(|d| d.info.description.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    (
        id_len as u16,
        date_len as u16,
        type_len as u16,
        amt_len as u16,
        cat_len as u16,
        peer_len as u16,
        desc_len as u16,
    )
}
