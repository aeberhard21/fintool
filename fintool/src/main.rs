use std::fs::{self};
use std::path::{Path, PathBuf};
#[cfg(feature = "ratatui_support")]
use std::{error::Error, io};
#[cfg(feature = "ratatui_support")]
use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};

#[cfg(feature = "ratatui_support")]
use crate::app::app::App;
#[cfg(feature = "ratatui_support")]
use crate::app::ui;
#[cfg(feature = "ratatui_support")]
use crate::app::screen::{CurrentScreen, TabBankSelected};
#[cfg(feature = "ratatui_support")]
use crate::database::DbConn;

mod accounts;
mod database;
mod stocks;
mod tui;
mod types;
#[cfg(feature = "ratatui_support")]
mod app;


fn main() -> Result<(), Box<dyn Error>> {
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

    if cfg!(feature = "ratatui_support") {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // create app and run it
        let mut app = App::new(&mut _db);
        let res = run_app(&mut terminal, &mut app);

        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
    } else { 
        println!("Welcome to FinTool!");
        tui::menu(&mut _db);
    }
    Ok(())
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
                        } else { 
                            app.invalid_input = true;
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => { 
                        return Ok(true)
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
                            }
                            Some(TabBankSelected::AccountTypeTabs) => {
                                // get accounts for filter
                                app.advance_account_type();
                                app.accounts_for_type = Some(app.db.get_user_accounts_by_type(app.user_id.unwrap(), app.selected_atype_tab).unwrap());
                            }
                            _ => {}
                        }

                    }
                    (_, KeyCode::Left) => { 
                        match app.tab_bank_selected { 
                            Some(TabBankSelected::AccountTabs) => {
                                app.retreat_account();
                            }
                            Some(TabBankSelected::AccountTypeTabs) => {
                                app.retreat_account_type();
                                app.accounts_for_type = Some(app.db.get_user_accounts_by_type(app.user_id.unwrap(), app.selected_atype_tab).unwrap());
                                app.get_account();
                            }
                            _ => {}
                        }
                    }
                    (_, KeyCode::Backspace) => { 
                        if let Some(select_mode) = &app.tab_bank_selected { 
                            match select_mode { 
                                TabBankSelected::AccountTabs => app.toggle_selecting(),
                                _ => {}
                            } 
                        }
                    }
                    (_, KeyCode::Enter) => { 
                        if let Some(select_mode) = &app.tab_bank_selected { 
                            match select_mode { 
                                TabBankSelected::AccountTypeTabs => app.toggle_selecting(),
                                _ => {}
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
