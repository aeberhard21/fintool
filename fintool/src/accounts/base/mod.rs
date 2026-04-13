#[cfg(feature = "ratatui_support")]
use crate::accounts::base::liquid_account::LiquidAccount;
/* ------------------------------------------------------------------------
  Copyright (C) 2025  Andrew J. Eberhard

  This program is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  This program is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <https://www.gnu.org/licenses/>.
-----------------------------------------------------------------------*/
#[cfg(feature = "ratatui_support")]
use crate::app::app::{App, DisplayValue};
#[cfg(feature = "ratatui_support")]
use crate::app::screen::{ledger_table_constraint_len_calculator, positions_table_constraint_len_calculator};
use crate::database::DbConn;
use crate::types::accounts::AccountRecord;
use crate::types::accounts::AccountType;
use crate::types::ledger::{DisplayableLedgerRecord, LedgerRecord};
#[cfg(feature = "ratatui_support")]
use crate::ui::centered_rect;
use chrono::{naive, NaiveDate, NaiveDateTime};
#[cfg(feature = "ratatui_support")]
use ratatui::symbols::block;
#[cfg(feature = "ratatui_support")]
use ratatui::{
    buffer::Buffer,
    layout::{self, Constraint, Direction, Layout, Rect},
    style::{palette, palette::tailwind, Color, Modifier, Style, Stylize},
    symbols::{self, Marker},
    text::{Line, Span, Text as ratatuiText},
    widgets::{
        Axis, Bar, BarChart, BarGroup, Block, Borders, Cell, Chart, Clear, Dataset, GraphType,
        HighlightSpacing, List, ListItem, Padding, Paragraph, Row, Table, Tabs, Widget, Wrap,
    },
    Frame,
};
use rusqlite::config::DbConfig;
use std::any::Any;
use strum::{Display, EnumIter, EnumString, FromRepr};
use yahoo_finance_api::Quote;

pub mod budget;
pub mod charge_account;
pub mod fixed_account;
pub mod liquid_account;
pub mod variable_account;

pub const KEY_TOTAL_VALUE: &str = "Current Value";
pub const KEY_GROWTH: &str = "Growth";

pub trait AccountCreation {
    fn create(uid: u32, name: String, _db: &DbConn) -> AccountRecord;
}

pub trait AccountOperations {
    fn import(&mut self);
    fn record(&mut self);
    fn modify(&mut self);
    fn export(&self);
    fn report(&self);
    fn link(&self, transacting_account: u32, ledger: LedgerRecord) -> Option<u32>;
}

pub trait AccountData {
    fn get_id(&self) -> u32;
    fn get_name(&self) -> String;
    fn get_ledger(&self) -> Vec<LedgerRecord>;
    fn get_ledger_within_dates(&self, start: NaiveDate, end: NaiveDate) -> Vec<LedgerRecord>;
    fn get_displayable_ledger(&self) -> Vec<DisplayableLedgerRecord>;
    fn get_value(&self) -> f32;
    fn get_value_on_day(&self, day: NaiveDate) -> f32;
    fn get_open_date(&self) -> NaiveDate;
}

#[cfg(feature = "ratatui_support")]
pub trait AccountUI: AccountData {
    fn populate_page_cache_f32(&self, app: &mut App);

    fn render(&self, frame: &mut Frame, area: Rect, app: &mut App);

    fn render_ledger_table(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        let header_style = Style::default()
            .fg(app.ledger_table_colors.header_fg)
            .bg(app.ledger_table_colors.header_bg);

        let selected_row_style = Style::new()
            .add_modifier(Modifier::REVERSED)
            .fg(app.ledger_table_colors.selected_row_style_fg);

        let header = [
            "ID",
            "Date",
            "Type",
            "Amount",
            "Category",
            "Peer",
            "Description",
            "Labels",
        ]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);

        if let Some(ledger) = app.ledger_entries.clone() {
            let data = ledger;

            let rows = data.iter().enumerate().map(|(i, record)| {
                let color = match i % 2 {
                    0 => app.ledger_table_colors.normal_row_color,
                    _ => app.ledger_table_colors.alt_row_color,
                };
                let item = [
                    &record.id.to_string(),
                    &record.info.date,
                    &record.info.transfer_type,
                    &record.info.amount.to_string(),
                    &record.info.category,
                    &record.info.participant.to_string(),
                    &record.info.description,
                    &record.info.labels,
                ];
                item.into_iter()
                    .map(|content| Cell::from(ratatuiText::from(format!("\n{content}\n"))))
                    .collect::<Row>()
                    .style(Style::new().fg(app.ledger_table_colors.row_fg).bg(color))
                    .height(4)
            });

            let bar: &'static str = " █ ";
            let constraint_lens = ledger_table_constraint_len_calculator(&data);
            let t = Table::new(
                rows,
                [
                    Constraint::Length(constraint_lens.0 + 1),
                    Constraint::Min(constraint_lens.1 + 1),
                    Constraint::Min(constraint_lens.2 + 1),
                    Constraint::Min(constraint_lens.3 + 1),
                    Constraint::Min(constraint_lens.4 + 1),
                    Constraint::Min(constraint_lens.5 + 1),
                    // don't take more than 25% of screen when display descriptions
                    Constraint::Min(area.width / 4),
                    Constraint::Min(constraint_lens.7 + 1),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Transactions")
                    .title_alignment(layout::Alignment::Center),
            )
            .row_highlight_style(selected_row_style)
            .highlight_symbol(ratatuiText::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .bg(app.ledger_table_colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

            app.ledger_entries = Some(data);

            frame.render_stateful_widget(t, area, &mut app.ledger_table_state);
        } else {
            let value = ratatuiText::styled(
                "No data to display!",
                Style::default().fg(tailwind::ROSE.c400).bold(),
            );

            let display = Paragraph::new(value)
                .centered()
                .alignment(layout::Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Value Over Time")
                        .title_alignment(layout::Alignment::Center)
                        .padding(Padding::new(
                            0,
                            0,
                            (if area.height > 4 {
                                area.height / 2 - 2
                            } else {
                                0
                            }),
                            0,
                        )),
                )
                .bg(tailwind::SLATE.c900);

            frame.render_widget(display, area);
        }
    }

    fn render_current_value(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        let current_value = app
            .page_cache_f32
            .as_ref()
            .expect("Account's page has not been cached!")
            .get(KEY_TOTAL_VALUE)
            .and_then(DisplayValue::as_f32)
            .expect("Could not find current value!");

        let value = ratatuiText::styled(
            current_value.to_string(),
            Style::default().fg(tailwind::EMERALD.c400).bold(),
        );

        let display = Paragraph::new(value)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Current Value")
                    .title_alignment(layout::Alignment::Center)
                    .padding(Padding::new(
                        0,
                        0,
                        (if area.height > 4 {
                            area.height / 2 - 2
                        } else {
                            0
                        }),
                        0,
                    )),
            )
            .bg(tailwind::SLATE.c900);

        frame.render_widget(display, area);
    }
}

#[cfg(feature = "ratatui_support")]
pub trait VariableAccountUI : AccountData {
    fn render_positions_table(&self, frame: &mut Frame, area: Rect, app: &mut App) {

        let block_title = "Positions";

        let header_style = Style::default()
            .fg(app.ledger_table_colors.header_fg)
            .bg(app.ledger_table_colors.header_bg);

        let selected_row_style = Style::new()
            .add_modifier(Modifier::REVERSED)
            .fg(app.ledger_table_colors.selected_row_style_fg);

        let header = [
            DisplayablePositionStatistics::get_ticker_str(),
            DisplayablePositionStatistics::get_quantity_str(),
            DisplayablePositionStatistics::get_value_str(),
            DisplayablePositionStatistics::get_price_str(),
            DisplayablePositionStatistics::get_total_cost_basis_str(),
            DisplayablePositionStatistics::get_unit_cost_str(),
            DisplayablePositionStatistics::get_unrealized_gl_str(),
            DisplayablePositionStatistics::get_unrealized_gl_per_str()
        ]        
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);

        let position_entries = app.positions_entries.take();
        if let Some(ledger) = position_entries.as_ref() {
            let data = ledger;

            let rows = data.iter().enumerate().map(|(i, record)| {
                let color = match i % 2 {
                    0 => app.ledger_table_colors.normal_row_color,
                    _ => app.ledger_table_colors.alt_row_color,
                };
                let item = [
                    &record.ticker,
                    &record.quantity,
                    &record.value,
                    &record.price,
                    &record.total_cost_basis,
                    &record.unit_cost,
                    &record.unrealized_gl,
                    &record.unrealized_gl_per,
                ];
                item.into_iter()
                    .enumerate()
                    .map(|content| {
                        let index = content.0;
                        let value = content.1;
                        match index {
                            0|1|3|5 => {
                                Cell::from(ratatuiText::from(format!("\n{value}\n")).style(tailwind::WHITE))
                            }
                            _ => {
                                if value.parse::<f32>().unwrap() < 0.0 { 
                                    Cell::from(ratatuiText::from(format!("\n{value}\n")).style(tailwind::ROSE.c500))
                                } else { 
                                    Cell::from(ratatuiText::from(format!("\n{value}\n")).style(tailwind::EMERALD.c500))
                                }
                            }
                        }
                    })
                    .collect::<Row>()
                    .style(Style::new().fg(app.ledger_table_colors.row_fg).bg(color))
                    .height(4)
            });

            let bar: &'static str = " █ ";
            let constraint_lens = positions_table_constraint_len_calculator(&data);
            let t = Table::new(
                rows,
                [
                    Constraint::Length(constraint_lens.0 + 1),
                    Constraint::Min(constraint_lens.1 + 1),
                    Constraint::Min(constraint_lens.2 + 1),
                    Constraint::Min(constraint_lens.3 + 1),
                    Constraint::Min(constraint_lens.4 + 1),
                    Constraint::Min(constraint_lens.5 + 1),
                    // don't take more than 25% of screen when display descriptions
                    Constraint::Min(area.width / 4),
                    Constraint::Min(constraint_lens.7 + 1),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(block_title)
                    .title_alignment(layout::Alignment::Center),
            )
            .row_highlight_style(selected_row_style)
            .highlight_symbol(ratatuiText::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .bg(app.ledger_table_colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);

            frame.render_stateful_widget(t, area, &mut app.ledger_table_state);
        } else {
            let value = ratatuiText::styled(
                "No data to display!",
                Style::default().fg(tailwind::ROSE.c400).bold(),
            );

            let display = Paragraph::new(value)
                .centered()
                .alignment(layout::Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(block_title)
                        .title_alignment(layout::Alignment::Center)
                        .padding(Padding::new(
                            0,
                            0,
                            (if area.height > 4 {
                                area.height / 2 - 2
                            } else {
                                0
                            }),
                            0,
                        )),
                )
                .bg(tailwind::SLATE.c900);

            frame.render_widget(display, area);
        }

        app.positions_entries = position_entries;
    }
}

#[cfg(not(feature = "ratatui_support"))]
pub trait Account: AccountData + AccountOperations + Any {
    fn kind(&self) -> AccountType;
    fn has_budget(&self) -> bool;
    fn set_budget(&self);
}

#[cfg(feature = "ratatui_support")]
pub trait Account: AccountData + AccountOperations + AccountUI + Any {
    fn kind(&self) -> AccountType;
    fn as_any(&self) -> &dyn Any;
    fn has_budget(&self) -> bool;
    fn set_budget(&self);
    fn as_liquid_account(&self) -> Option<&dyn LiquidAccount> { 
        return None;
    }
    fn as_variable_account(&self) -> Option<&dyn VariableAccountUI> { 
        return None;
    }
    fn renders_tables(&self) -> Vec<String> { 
        return vec!["Transactions".to_string()];
    }
}

#[derive(Clone, Display, Debug, FromRepr, EnumIter, EnumString)]
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
    #[strum(to_string = "All Time")]
    AllTime,
    #[strum(to_string = "Custom")]
    Custom,
}

impl AnalysisPeriod {
    pub fn to_menu_selection(value: Self) -> String {
        format!("{value}")
    }
}

#[derive(Debug, Clone)]
struct StockData {
    ticker: String,
    quotes: Vec<Quote>,
    history: Vec<SharesOwned>,
}

#[derive(Debug, Clone)]
struct SharesOwned {
    date: NaiveDate,
    shares: f32,
}

#[derive(Debug, Clone)]
pub struct DisplayablePositionStatistics { 
    pub ticker : String,
    pub quantity : String, 
    pub value : String, 
    pub price : String,
    pub total_cost_basis : String, 
    pub unit_cost : String, 
    pub unrealized_gl : String,
    pub unrealized_gl_per : String,
}

impl DisplayablePositionStatistics {
    pub fn get_ticker_str() -> String { 
        "Ticker".to_string()
    }
    pub fn get_quantity_str() -> String { 
        "Quantity (UoM)".to_string()
    }
    pub fn get_value_str() -> String { 
        "Value ($)".to_string()
    }    
    pub fn get_price_str() -> String { 
        "Price ($)".to_string()
    }    
    pub fn get_total_cost_basis_str() -> String { 
        "Total Cost Basis ($)".to_string()
    }
    pub fn get_unit_cost_str() -> String { 
        "Unit Cost ($)".to_string()
    }
    pub fn get_unrealized_gl_str() -> String { 
        "Unrealized G/L ($)".to_string()
    }
    pub fn get_unrealized_gl_per_str() -> String { 
        "Unrealized G/L (%)".to_string()
    }   
}

pub fn render_table_tabs(
    frame: &mut Frame,
    area: Rect,
    tab_names: Vec<String>,
    selected_tab: usize,
    highlight_color: Color,
) {
    let atype_tabs = Tabs::new(tab_names.into_iter())
        .highlight_style(highlight_color)
        .select(selected_tab)
        .block(
            Block::bordered()
                .title(" Tables ")
                .style(Style::new().bg(tailwind::SLATE.c900)),
        )
        .padding("", "")
        .divider(" | ");
    frame.render_widget(atype_tabs, area);
}