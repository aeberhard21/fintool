use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use ratatui::{
    buffer::Buffer,
    layout::{self, Constraint, Direction, Layout, Rect},
    style::{palette::{self, tailwind}, Color, Modifier, Style, Stylize},
    symbols::{self, bar, Marker},
    text::{Line, Span, Text},
    widgets::{
        Axis, Bar, BarChart, BarGroup, Block, Borders, Cell, Chart, Clear, Dataset, GraphType, HighlightSpacing, LegendPosition, LineGauge, List, ListItem, Paragraph, Row, Table, Tabs, Widget, Wrap
    },
    Frame,
};

use super::app::App;
use super::screen::{CurrentScreen, TabMenu};
use crate::{accounts::{self, bank_account::BankAccount}, app::screen::UserLoadedState, tui::tui_accounts::{get_total_assets, get_total_liabilities}, types::accounts::AccountType};
use crate::{accounts::base::Account, app::screen::CurrentlySelecting};

pub fn ui(frame: &mut Frame, app: &mut App) {
    // Create the layout sections.
    let chunks = match app.current_screen {
        CurrentScreen::Accounts => Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area()),
        _ => Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area()),
    };

    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());

    let title = Paragraph::new(Text::styled("FINTOOL", Style::default().fg(Color::Green)))
        .block(title_block)
        .centered()
        .bold();

    frame.render_widget(title, chunks[0]);

    let current_navigation_text = vec![
        // The first half of the text
        match app.current_screen {
            CurrentScreen::Login => Span::styled("Login", Style::default().fg(Color::Cyan)),
            CurrentScreen::Accounts => Span::styled("Accounts", Style::default().fg(Color::White)),
            CurrentScreen::Main => Span::styled("Main", Style::default().fg(Color::Yellow)),
        }
        .to_owned(),
    ];

    let mode_footer = Paragraph::new(Line::from(current_navigation_text))
        .block(Block::default().borders(Borders::ALL));

    let current_keys_hint = {
        match app.current_screen {
            CurrentScreen::Login => Span::styled (
                "(q) to quit / (:) Create User / (⏎) Login",
                Style::default().fg(Color::LightBlue),
            ),
            CurrentScreen::Accounts => {
                match app.currently_selected.unwrap() { 
                    CurrentlySelecting::AccountTypeTabs|CurrentlySelecting::AccountTabs => {
                        Span::styled (
                        "(q) to quit / (◀︎) Move Tab Left / (▶︎) Move Tab Right / (⏎) Select / (⌫) Deselect / (c) Create Account / (esc) Exit to Main",
                        Style::default().fg(Color::LightBlue),
                        )
                    }
                    CurrentlySelecting::Account => { 
                        Span::styled (
                        "(q) to quit / (⌫) Deselect / (a) Analyze / (e) Edit Account / (r) Record Entry / (m) Modify Ledger / (i) Import / (j) Advance Row / (k) Retreat Row / (G) Go to Last / (H) Go to First",
                        Style::default().fg(Color::LightBlue),
                        )
                    }
                }
            },
            CurrentScreen::Main => Span::styled("(q, Ctrl-c) to quit / (a) Open Accounts", Style::default().fg(Color::LightBlue)),
        }
    };

    let key_notes_footer =
        Paragraph::new(Line::from(current_keys_hint)).block(Block::default().borders(Borders::ALL));

    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(10), Constraint::Percentage(90)])
        .split(chunks[chunks.len() - 1]);

    frame.render_widget(mode_footer, footer_chunks[0]);
    frame.render_widget(key_notes_footer, footer_chunks[1]);

    if let CurrentScreen::Login = app.current_screen {

        let centered_area = centered_rect(60, 25, frame.area());

        if let UserLoadedState::NotLoaded = app.user_load_state {
            // println!("HERE AT ALL TIMES!: {}", app.user);
            let popup_block = Block::default()
                .title(" Login ")
                .borders(Borders::ALL)
                .style(Style::default().bg(tailwind::EMERALD.c950));

            // prompt for user name
            let mut content = "Username: ".to_string();
            content.push_str(&app.key_input.as_str());
            let username_text = Text::styled(content, Style::default().fg(tailwind::EMERALD.c50));

            let login_paragraph = Paragraph::new(username_text)
                .block(popup_block)
                .wrap(Wrap { trim: false });
            frame.render_widget(login_paragraph, centered_area);

            // display error message when user does not exist
            if app.invalid_input {
                let error_footer = Paragraph::new(Line::from("Unrecognized user id!")).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(tailwind::RED.c500),
                );
                frame.render_widget(error_footer, footer_chunks[0]);
            }
        }

        if let UserLoadedState::Loading = app.user_load_state { 
            let title = Block::default()
                .title(" Loading... ")
                .borders(Borders::ALL)
                .style(Style::default().bg(tailwind::EMERALD.c950));
            let lg = LineGauge::default()
                .block(title)
                .filled_style(Style::default().fg(Color::Blue).bg(tailwind::EMERALD.c400))
                .unfilled_style(Style::default().fg(Color::Red).bg(tailwind::SLATE.c700))
                .ratio(app.load_profile_progress);
                // .render(area, buf);
            frame.render_widget(lg, centered_area);
        } 
    }

    if let CurrentScreen::Accounts = app.current_screen {
        AccountType::render(
            frame,
            chunks[1],
            app.selected_atype_tab as usize,
            " Account Types ".to_string(),
        );
        if app.accounts_for_type.is_none() {
            let content = "No Accounts found!".to_string();
            let display_text = Text::styled(content, Style::default().fg(tailwind::RED.c500));

            let accounts_paragraph = Paragraph::new(display_text).wrap(Wrap { trim: false });
            let area = centered_rect(60, 25, frame.area());
            frame.render_widget(accounts_paragraph, area);
        } else {
            let accts = app.accounts_for_type.clone().unwrap();
            if !accts.is_empty() {
                if let Some(current_selection) = app.currently_selected {
                    match current_selection {
                        CurrentlySelecting::AccountTypeTabs => {
                            render_account_tabs(
                                frame,
                                chunks[2],
                                app.accounts_for_type.clone().unwrap(),
                                app.selected_account_tab,
                                Color::Reset,
                            );
                        }
                        CurrentlySelecting::AccountTabs => render_account_tabs(
                            frame,
                            chunks[2],
                            app.accounts_for_type.clone().unwrap(),
                            app.selected_account_tab,
                            Color::Red,
                        ),
                        CurrentlySelecting::Account => render_account_tabs(
                            frame,
                            chunks[2],
                            app.accounts_for_type.clone().unwrap(),
                            app.selected_account_tab,
                            Color::Green,
                        ),
                    }
                }
                if let Some(acct) = app.account.take() {
                    acct.render(frame, chunks[3], app);
                    app.account = Some(acct);
                } else {
                    let error_footer = Paragraph::new(Line::from(
                        "ERROR: Unable to retrieve account information!",
                    ))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .style(tailwind::ROSE.c500),
                    );
                    frame.render_widget(error_footer, footer_chunks[0]);
                }
            } else {
                let content = "No Accounts found!".to_string();
                let display_text = Text::styled(content, Style::default().fg(tailwind::RED.c500));

                let login_paragraph = Paragraph::new(display_text).wrap(Wrap { trim: false });
                let area = centered_rect(60, 25, frame.area());
                frame.render_widget(login_paragraph, area);
            }
        }

        if app.invalid_input {
            let content = "Account operation invalid!".to_string();
            let display_text = Text::styled(content, Style::default().fg(tailwind::RED.c500));

            let login_paragraph = Paragraph::new(display_text).wrap(Wrap { trim: false });
            let area = centered_rect(60, 25, frame.area());
            frame.render_widget(login_paragraph, area);
        }
    }

    if let CurrentScreen::Main = app.current_screen { 
        let y_axi_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(chunks[1]);
        let upper_quadrants = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(y_axi_split[0]);
        let lower_quadrants = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(y_axi_split[1]);

        let quadrant_0 = upper_quadrants[0];
        let quadrant_1 = upper_quadrants[1];
        let quadrant_2 = lower_quadrants[0];
        let quadrant_3 = lower_quadrants[1];

        render_net_worth(app, frame, quadrant_0);
        render_net_worth_chart(app, frame, quadrant_1);
        render_asset_investment_ratio_chart(app, frame, quadrant_2);

        let last_one = Paragraph::new(Text::styled("Hmmm....I guess!", Style::default().fg(Color::Green)))
            .block(Block::default().borders(Borders::ALL).title("The Last One").style(Style::default()))
            .centered()
            .bold();

        frame.render_widget(last_one, quadrant_3);
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}

fn render_account_tabs(
    frame: &mut Frame,
    area: Rect,
    tab_names: Vec<String>,
    selected_tab: usize,
    highlight_color: Color,
) {
    let atype_tabs = Tabs::new(tab_names.into_iter())
        .highlight_style(highlight_color)
        .select(selected_tab)
        .block(Block::bordered().title(" Accounts "))
        .padding("", "")
        .divider(" | ");
    frame.render_widget(atype_tabs, area);
}

fn render_net_worth( app: &App, frame : &mut Frame, area : Rect ) { 

    let net_worth_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(37)
            ])
            .split(area);
    let net_worth_area = net_worth_areas[0];
    let assets_area = net_worth_areas[1];
    let liabilities_area = net_worth_areas[2];

    let (assets, liabilities) = if let Some(accounts) = &app.accounts { 
        ( get_total_assets(accounts), get_total_liabilities(accounts) )
    } else { 
        ( 0., 0. )
    };
    let net_worth = assets - liabilities;

    let net_worth_widget = Paragraph::new(Text::styled(format!("$ {:.2}", net_worth), Style::default().fg(tailwind::EMERALD.c500)))
        .block(Block::default().borders(Borders::ALL).title("Net Worth").style(Style::default()))
        .centered()
        .bold();
    let total_assets_widget = Paragraph::new(Text::styled(format!("$ {:.2}", assets), Style::default().fg(tailwind::EMERALD.c500)))
        .block(Block::default().borders(Borders::ALL).title("Total Assets").style(Style::default()))
        .centered()
        .bold();
    let liabilities_widget = Paragraph::new(Text::styled(format!("$ {:.2}", liabilities), Style::default().fg(tailwind::ROSE.c500)))
        .block(Block::default().borders(Borders::ALL).title("Total Liability").style(Style::default()))
        .centered()
        .bold();

    frame.render_widget(net_worth_widget, net_worth_area);
    frame.render_widget(total_assets_widget, assets_area);
    frame.render_widget(liabilities_widget, liabilities_area);

}

fn render_net_worth_chart( app: &App, frame : &mut Frame, area : Rect) {

    if let Some(accounts) = &app.accounts { 
        // get earliest start date
        let mut start_date = Local::now().date_naive();
        let today = start_date;
        for account in accounts { 
            start_date = start_date.min(account.get_open_date());
        }
        let start_eoy = NaiveDate::from_ymd_opt(start_date.year(), 12, 31).unwrap();
        let mut date = start_eoy;
        let mut data : Vec<(f64, f64)> = Vec::new();
        let mut tstamp_min = f64::MAX;
        let mut min_total= f64::MAX;
        let mut max_total = f64::MIN;
        while date < today {
            let aggregate : f64 = accounts.iter().map(|acct| acct.get_value_on_day(date) as f64).sum();
            let timestamp = date
                    .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                    .and_utc()
                    .timestamp_millis() as f64;
            min_total = min_total.min(aggregate);
            max_total = max_total.max(aggregate);
            tstamp_min = tstamp_min.min(timestamp);

            data.push((timestamp, aggregate));

            date = NaiveDate::from_ymd_opt(date.year() + 1, 12, 31).unwrap();
        }
        // get for today
        let aggregate : f64 = accounts.iter().map(|acct| acct.get_value_on_day(today) as f64).sum();
        let timestamp = today
                .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc()
                .timestamp_millis() as f64;
        let tstamp_max = timestamp;
        min_total = min_total.min(aggregate);
        max_total = max_total.max(aggregate);
        tstamp_min = tstamp_min.min(timestamp);
        data.push((timestamp, aggregate));

        let datasets = vec![               
            Dataset::default()
            .name("Time Period Investment")
            .marker(symbols::Marker::HalfBlock)
            .style(Style::default().fg(tailwind::LIME.c400))
            .graph_type(GraphType::Line)
            .data(&data)];

        let net_worth_chart = Chart::new(datasets)
            .block(
                Block::bordered().title(Line::from(" Growth Over Time ").cyan().bold().centered()),
            )
            .legend_position(Some(LegendPosition::TopLeft))
            .x_axis(
                Axis::default()
                    .title("Time")
                    .style(Style::default().gray())
                    .bounds([tstamp_min, tstamp_max])
                    .labels([start_eoy.to_string(), today.to_string()]),
            )
            .y_axis(
                Axis::default()
                    .title("Value (💰)")
                    .style(Style::default().gray())
                    .bounds([min_total, max_total])
                    .labels(
                        float_range(min_total, max_total, (max_total - min_total) / 5.0)
                            .into_iter()
                            .map(|x| format!("{:.2}", x)),
                    ),
            );
        frame.render_widget(net_worth_chart, area);
    } else { 
       let net_worth_chart = Paragraph::new(Text::styled(format!("No data to display!"), Style::default().fg(tailwind::ROSE.c500)))
            .block(Block::default().borders(Borders::ALL).title("").style(Style::default()))
            .centered()
            .bold();

        frame.render_widget(net_worth_chart, area);
    }

}

fn render_asset_investment_ratio_chart( app: &App, frame : &mut Frame, area : Rect) {

    if let Some(accounts) = &app.accounts { 

        let total_assets = get_total_assets(accounts);
        let mut cash: f32 = 0.0;
        let mut liquid_investment : f32 = 0.0;
        let mut long_term_investments : f32 = 0.0;
        let mut retirement : f32 = 0.0;
        let mut health : f32 = 0.0;

        for account in accounts { 
            match account.kind() { 
                AccountType::Bank|AccountType::Wallet => {
                    cash = cash + account.get_value();
                }
                AccountType::CD => {
                    liquid_investment = liquid_investment + account.get_value();
                }
                AccountType::Investment => {
                    long_term_investments = long_term_investments + account.get_value();
                }
                _ => {}
            }
        }

        let mut data : Vec<(String, u64, Color)> = Vec::new();
        data.push(("Cash".to_string(), (cash/total_assets * 100.) as u64, tailwind::AMBER.c500));
        data.push(("Liquid Investments".to_string(), (liquid_investment/total_assets * 100.) as u64, tailwind::FUCHSIA.c500));
        data.push(("Long Term Investments".to_string(), (long_term_investments/total_assets * 100.) as u64 , tailwind::INDIGO.c500));
        data.push(("Retirement".to_string(), (retirement/total_assets * 100.) as u64, tailwind::LIME.c500));
        data.push(("Health".to_string(), (health/total_assets * 100.) as u64, tailwind::ORANGE.c500));

        let bars = data.iter().map(|x| {
            Bar::default()
                .value(x.1)
                .label(Line::from(format!("{}", x.0)))
                .text_value(format!("{}%", x.1))
                .style(Style::new().fg(x.2))
                .value_style((Style::new().fg(x.2).reversed()))
        }).collect::<Vec<Bar>>();

        let chart =     BarChart::default()
            .data(BarGroup::default().bars(&bars))
            .block(Block::bordered().title_top("Asset Investment Ratio"))
            .bar_width(10)
            .bar_gap(10);

        frame.render_widget(chart, area);
    } else { 
       let chart = Paragraph::new(Text::styled(format!("No data to display!"), Style::default().fg(tailwind::ROSE.c500)))
            .block(Block::default().borders(Borders::ALL).title("").style(Style::default()))
            .centered()
            .bold();

        frame.render_widget(chart, area);
    }
    
}

pub fn float_range(start: f64, end: f64, step: f64) -> Vec<f64> {
    let mut vec = Vec::new();
    let mut current = start;
    while current <= end {
        vec.push(current);
        current += step;
    }
    vec
}
