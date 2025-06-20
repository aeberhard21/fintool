use std::fs::{self};
use std::path::{Path, PathBuf};
use std::result;
use std::io::Error;
use std::io;
#[cfg(feature = "ratatui_support")]
use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        cursor::MoveTo,
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
    },
    Terminal,
};

#[cfg(feature = "ratatui_support")]
use crate::app::app::App;
#[cfg(feature = "ratatui_support")]
use crate::app::ui;
#[cfg(feature = "ratatui_support")]
use crate::app::screen::{CurrentScreen, TabBankSelected};
use crate::database::DbConn;
use crate::tui::tui_user::create_user;

mod accounts;
mod database;
mod stocks;
mod tui;
mod types;
#[cfg(feature = "ratatui_support")]
mod app;


fn main() -> Result<(), std::io::Error> {
    let db_dir: String = String::from("./db");

    let mut _db: DbConn;
    match Path::new(&db_dir).try_exists() {
        Ok(true) => {}
        Ok(false) => {
            fs::create_dir(&db_dir);
        }
        Err(_) => {
            panic!("Unable to verify existence of database directory!");
        }
    }

    let mut db = PathBuf::new();
    db.push(&db_dir);
    db.push("finances.db");
    match Path::new(&db_dir).join(&db).try_exists() {
        Ok(_) => {
            // nothing to do
            _db = DbConn::new(db.clone()).unwrap();
        }
        Err(_) => {
            panic!("Unable to verify existence of the database!");
        }
    }

    #[cfg(feature = "ratatui_support")]
    init_and_run_app(&mut _db)?;

    #[cfg(not(feature = "ratatui_support"))]
    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
    #[cfg(not(feature = "ratatui_support"))]
    println!("Welcome to FinTool!");
    #[cfg(not(feature = "ratatui_support"))]
    tui::menu(&mut _db);

    Ok(())
}

#[cfg(feature = "ratatui_support")]
fn init_and_run_app(_db: &mut DbConn) -> io::Result<bool> { 
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new(_db);
    let res = run_app(&mut terminal, &mut app)?;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(res)
}

#[cfg(feature = "ratatui_support")]
fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<bool> {
    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                // Skip events that are not KeyEventKind::Press
                continue;
            }
            match app.current_screen {
                CurrentScreen::Login => match (key.modifiers, key.code) { 
                    (_, KeyCode::Enter) => {
                        if let Some(id) = app.validate_user(app.key_input.to_string()) {
                            app.user_id = Some(id);
                            app.current_screen = CurrentScreen::Accounts;
                            app.accounts_for_type = app.db.get_user_accounts_by_type(app.user_id.unwrap(), app.selected_atype_tab).unwrap();
                            if app.accounts_for_type.is_some() {
                                app.get_account();
                            }
                        } else { 
                            app.invalid_input = true;
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => { 
                        return Ok(true)
                    }
                    (_, KeyCode::Char(':')) => {
                        disable_raw_mode()?;
                        execute!(io::stdout(), Clear(ratatui::crossterm::terminal::ClearType::All), MoveTo(0,0)).unwrap();
                        create_user(&mut app.db);
                        enable_raw_mode()?;
                        terminal.clear().unwrap();
                    }
                    (_, KeyCode::Char(value)) => { 
                        app.key_input.push(value);
                    }
                    (_, KeyCode::Backspace) => {
                        app.key_input.pop();
                    }
                    _ => {}
                },
                CurrentScreen::Accounts => match (key.modifiers, key.code) {
                    (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C'))  => { 
                        return Ok(true)
                    }
                    (_, KeyCode::Right) => { 
                        match app.tab_bank_selected { 
                            Some(TabBankSelected::AccountTabs) => {
                                app.advance_account();
                                if app.accounts_for_type.is_some() {
                                    app.get_account();
                                }
                            }
                            Some(TabBankSelected::AccountTypeTabs) => {
                                // get accounts for filter
                                app.advance_account_type();
                                app.accounts_for_type = app.db.get_user_accounts_by_type(app.user_id.unwrap(), app.selected_atype_tab).unwrap();
                                if app.accounts_for_type.is_some() {
                                    app.get_account();
                                }
                            }
                            _ => {}
                        }

                    }
                    (_, KeyCode::Left) => { 
                        match app.tab_bank_selected { 
                            Some(TabBankSelected::AccountTabs) => {
                                app.retreat_account();
                                if app.accounts_for_type.is_some() {
                                    app.get_account();
                                }
                            }
                            Some(TabBankSelected::AccountTypeTabs) => {
                                app.retreat_account_type();
                                app.accounts_for_type = app.db.get_user_accounts_by_type(app.user_id.unwrap(), app.selected_atype_tab).unwrap();
                                if app.accounts_for_type.is_some() {
                                    app.get_account();
                                }
                            }
                            _ => {}
                        }
                    }
                    (_, KeyCode::Backspace) => { 
                        if let Some(select_mode) = &app.tab_bank_selected { 
                            match select_mode { 
                                TabBankSelected::AccountTabs => {
                                    app.toggle_selecting();
                                    app.accounts_for_type = app.db.get_user_accounts_by_type(app.user_id.unwrap(), app.selected_atype_tab).unwrap();
                                    if app.accounts_for_type.is_some() {
                                        app.get_account();
                                    }
                                    // upon move out of account type, reset default tab to the first one
                                    app.selected_account_tab = 0;
                                }
                                _ => {}
                            } 
                        }
                    }
                    (_, KeyCode::Enter) => { 
                        if app.accounts_for_type.is_none() {} 
                        else {
                            if let Some(select_mode) = &app.tab_bank_selected { 
                                match select_mode { 
                                    TabBankSelected::AccountTypeTabs => app.toggle_selecting(),
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
                _ => {}
            }
        }
    }
}
