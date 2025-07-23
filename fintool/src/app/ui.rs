use ratatui::{
    buffer::Buffer,
    layout::{self, Constraint, Direction, Layout, Rect},
    style::{
        palette::{self, tailwind},
        Color, Style, Stylize,
    },
    text::{Line, Span, Text},
    widgets::{
        Bar, BarChart, BarGroup, Block, Borders, Clear, List, ListItem, LineGauge, Paragraph, Tabs, Widget,
        Wrap,
    },
    Frame,
};

use super::app::App;
use super::screen::{CurrentScreen, TabMenu};
use crate::{app::screen::UserLoadedState, types::accounts::AccountType};
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
                .label("Background:")
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
