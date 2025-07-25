use std::any::Any;
#[cfg(feature = "ratatui_support")]
use crate::app::app::App;
#[cfg(feature = "ratatui_support")]
use crate::app::screen::ledger_table_constraint_len_calculator;
use crate::database::DbConn;
use crate::types::accounts::AccountRecord;
use crate::types::accounts::AccountType;
use crate::types::ledger::{DisplayableLedgerRecord, LedgerRecord};
#[cfg(feature = "ratatui_support")]
use crate::ui::centered_rect;
use chrono::{naive, NaiveDate, NaiveDateTime};
use yahoo_finance_api::Quote;
#[cfg(feature = "ratatui_support")]
use ratatui::{
    buffer::Buffer,
    layout::{self, Constraint, Direction, Layout, Rect},
    style::{palette, palette::tailwind, Color, Modifier, Style, Stylize},
    symbols::{self, Marker},
    text::{Line, Span, Text as ratatuiText},
    widgets::{
        Axis, Bar, BarChart, BarGroup, Block, Borders, Cell, Chart, Clear, Dataset, GraphType,
        HighlightSpacing, List, ListItem, Paragraph, Row, Table, Tabs, Widget, Wrap,
    },
    Frame,
};
use rusqlite::config::DbConfig;
use strum::{Display, EnumIter, EnumString, FromRepr};

pub mod charge_account;
pub mod fixed_account;
pub mod variable_account;

pub trait AccountCreation {
    fn create(uid: u32, name: String, _db: &DbConn) -> AccountRecord;
}

pub trait AccountOperations {
    // fn create( account_id : u32, db : &mut DbConn );
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
    fn get_value_on_day(&self, day : NaiveDate) -> f32;
    fn get_open_date(&self) -> NaiveDate;
}
#[cfg(feature = "ratatui_support")]
pub trait AccountUI: AccountData {
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
        ]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);

        let data = self.get_displayable_ledger();
        app.ledger_entries = Some(data.clone());

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
                Constraint::Min(constraint_lens.6 + 1),
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
        frame.render_stateful_widget(t, area, &mut app.ledger_table_state);
    }

    fn render_current_value(&self, frame: &mut Frame, area: Rect, app: &mut App) {
        let value = ratatuiText::styled(
            self.get_value().to_string(),
            Style::default().fg(tailwind::EMERALD.c400).bold(),
        );

        let display = Paragraph::new(value)
            .centered()
            .alignment(layout::Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Current Value")
                    .title_alignment(layout::Alignment::Center),
            )
            .bg(tailwind::SLATE.c900);

        // let centered = centered_rect(10, 10, area);
        // let centered_display = Paragraph::new(value);
        // frame.render_widget(centered_display, centered);
        frame.render_widget(display, area);
    }
}

#[cfg(not(feature = "ratatui_support"))]
pub trait Account: AccountData + AccountOperations {
    fn kind(&self) -> AccountType;
}

#[cfg(feature = "ratatui_support")]
pub trait Account: AccountData + AccountOperations + AccountUI + Any {
    fn kind(&self) -> AccountType;
    fn as_any(&self) -> &dyn Any;
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

// fn render_pi_chart(&self, frame: &mut Frame, area: Rect, app: &App) {

//     // Simulated pie chart using lines (segments)
//     let canvas = Canvas::default()
//         .block(Block::default().title("Simulated Pie Chart").borders(Borders::ALL))
//         .paint(|ctx| {
//             let center = (50.0, 25.0); // x, y
//             let radius = 10.0;

//             // Draw the full circle
//             ctx.draw(&Circle {
//                 x: center.0,
//                 y: center.1,
//                 radius,
//                 color: ratatui::style::Color::White,
//             });

//             // Draw pie segments (example: 3 segments)
//             let segments = vec![
//                 (0.0, 120.0, Color::Yellow),   // degrees
//                 (120.0, 240.0, Color::Blue),
//                 // (240.0, 360.0, Color::Green),
//             ];

//             let mut last: f64 = 0.0;
//             for &(start, end, color) in &segments {

//                 // let mid_angle: f64 = (start + end) / 2.0;
//                 let rads = (last + end).to_radians();
//                 for i in 0..360 {
//                     let rad = rads * (f64::from(i) * (1./360.));
//                 let x = center.0 + radius * rad.cos();
//                 let y = center.1 + radius * rad.sin();
//                 if x < center.0 { 
//                 ctx.draw(&CanvasLine {
//                     x2: center.0,
//                     y2: center.1,
//                     x1: x,
//                     y1: y,
//                     color: color,
//                 })} else { 
//                 ctx.draw(&CanvasLine {
//                     x1: center.0,
//                     y1: center.1,
//                     x2: x,
//                     y2: y,
//                     color: color,
//                 })}
//                 }

//                 last = end;
//             }
//         })
//         .x_bounds([0.0, 100.0])
//         .y_bounds([0.0, 50.0]);

//     frame.render_widget(canvas, area);

// }


#[derive(Debug, Clone)]
struct StockData {
    ticker: String, 
    quotes : Vec<Quote>,
    history : Vec<SharesOwned>
}

#[derive(Debug, Clone)]
struct SharesOwned { 
    date : NaiveDate, 
    shares : f32
}
