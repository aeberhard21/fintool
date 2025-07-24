#[cfg(feature = "ratatui_support")]
use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        cursor::MoveTo,
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
            KeyModifiers,
        },
        execute,
        terminal::{
            disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
            LeaveAlternateScreen,
        },
    },
    Terminal,
};
use std::fs::{self};
use std::io;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::result;

#[cfg(feature = "ratatui_support")]
use crate::app::app::App;
#[cfg(feature = "ratatui_support")]
use crate::app::screen::{CurrentScreen, CurrentlySelecting, UserLoadedState};
#[cfg(feature = "ratatui_support")]
use crate::app::ui;
use crate::database::DbConn;
use crate::tui::tui_user::create_user;
use crate::tui::*;
use crate::accounts::base::Account;
use crate::types::accounts::AccountType;

mod accounts;
#[cfg(feature = "ratatui_support")]
mod app;
mod database;
mod stocks;
mod tui;
mod types;

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
        app.invalid_input = false;
        terminal.draw(|f| ui::ui(f, app))?;

        if let UserLoadedState::Loading = app.user_load_state {
            while app.user_load_state != UserLoadedState::Loaded {
                let mut profiles_loaded = 0;
                let account_records = app.db.get_user_accounts(app.user_id.unwrap()).unwrap();
                let number_of_accounts = account_records.len();
                let mut accounts = Vec::new();
                for record in account_records { 
                    let acct = decode_and_init_account_type(app.user_id.unwrap(), &app.db, &record);
                    profiles_loaded = profiles_loaded + 1;
                    app.load_profile_progress = (profiles_loaded as f64 / number_of_accounts as f64);
                    
                    // force update to progress bar
                    terminal.draw(|f| ui::ui(f, app))?;
                    accounts.push(acct);
                }
                app.accounts = Some(accounts);

                terminal.draw(|f: &mut ratatui::Frame<'_>| ui::ui(f, app))?;

                if profiles_loaded == number_of_accounts { 
                    app.current_screen = CurrentScreen::Main;
                    app.user_load_state = UserLoadedState::Loaded;
                }
            }
            // force quick turnaround
            continue;
        }

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
                            app.user_load_state = UserLoadedState::Loading;
                            // app.current_screen = CurrentScreen::Loading;
                        } else {
                            app.invalid_input = true;
                        }
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                        return Ok(true)
                    }
                    (_, KeyCode::Char(':')) => {
                        disable_raw_mode()?;
                        execute!(
                            io::stdout(),
                            Clear(ratatui::crossterm::terminal::ClearType::All),
                            MoveTo(0, 0)
                        )
                        .unwrap();

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
                CurrentScreen::Main => match (key.modifiers, key.code) { 
                    (_, KeyCode::Char('q'))
                    | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                        return Ok(true)
                    }
                    (_, KeyCode::Char('a')) => { 
                        app.current_screen = CurrentScreen::Accounts;
                        app.accounts_for_type = if let Some(accounts) = &app.accounts {
                            let filtered_accounts = accounts.iter().filter(|x| {
                                    is_account_type(x, app.selected_atype_tab)
                            }).collect::<Vec<&Box<dyn Account>>>();
                            let names = filtered_accounts.iter().map(|x| x.get_name()).collect::<Vec<String>>();
                            Some(names)
                        } else { 
                            None
                        };
                        if app.accounts_for_type.is_some() {
                            app.get_account();
                        }
                    }
                    _ => {}
                }
                CurrentScreen::Accounts => match (key.modifiers, key.code) {
                    (_, KeyCode::Char('q'))
                    | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                        return Ok(true)
                    }
                    (_, KeyCode::Right | KeyCode::Char('l')) => {
                        match app.currently_selected {
                            Some(CurrentlySelecting::AccountTabs) => {
                                // app.restore_account();
                                app.advance_account();
                                if app.accounts_for_type.is_some() {
                                    app.get_account();
                                }
                            }
                            Some(CurrentlySelecting::AccountTypeTabs) => {
                                // get accounts for filter
                                app.advance_account_type();
                                app.accounts_for_type = if let Some(accounts) = &app.accounts {
                                    let filtered_accounts = accounts.iter().filter(|x| {
                                            is_account_type(x, app.selected_atype_tab)
                                    }).collect::<Vec<&Box<dyn Account>>>();
                                    let names = filtered_accounts.iter().map(|x| x.get_name()).collect::<Vec<String>>();
                                    Some(names)
                                } else { 
                                    None
                                };
                                if app.accounts_for_type.is_some() {
                                    app.get_account();
                                }
                            }
                            _ => {}
                        }
                    }
                    (_, KeyCode::Left | KeyCode::Char('h')) => match app.currently_selected {
                        Some(CurrentlySelecting::AccountTabs) => {
                            // app.restore_account();
                            app.retreat_account();
                            if app.accounts_for_type.is_some() {
                                app.get_account();
                            }
                        }
                        Some(CurrentlySelecting::AccountTypeTabs) => {
                            app.retreat_account_type();
                            app.accounts_for_type = if let Some(accounts) = &app.accounts {
                                let filtered_accounts = accounts.iter().filter(|x| {
                                    is_account_type(x, app.selected_atype_tab)
                                }).collect::<Vec<&Box<dyn Account>>>();
                                let names = filtered_accounts.iter().map(|x| x.get_name()).collect::<Vec<String>>();
                                Some(names)
                            } else { 
                                None
                            };
                            if app.accounts_for_type.is_some() {
                                app.get_account();
                            }
                        }
                        _ => {}
                    },
                    (_, KeyCode::Backspace) => {
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::AccountTabs => {
                                    app.retreat_currently_selecting();
                                    app.accounts_for_type = if let Some(accounts) = &app.accounts {
                                        let filtered_accounts = accounts.iter().filter(|x| {
                                            is_account_type(x, app.selected_atype_tab)
                                        }).collect::<Vec<&Box<dyn Account>>>();
                                        let names = filtered_accounts.iter().map(|x| x.get_name()).collect::<Vec<String>>();
                                        Some(names)
                                    } else { 
                                        None
                                    };
                                    if app.accounts_for_type.is_some() {
                                        app.get_account();
                                    }
                                    // upon move out of account type, reset default tab to the first one
                                    app.selected_account_tab = 0;
                                }
                                CurrentlySelecting::Account => {
                                    app.retreat_currently_selecting();
                                }
                                _ => {}
                            }
                        }
                    }
                    (_, KeyCode::Enter) => {
                        if app.accounts_for_type.is_none() {
                        } else {
                            if let Some(select_mode) = &app.currently_selected {
                                match select_mode {
                                    CurrentlySelecting::AccountTypeTabs => {
                                        app.advance_currently_selecting()
                                    }
                                    CurrentlySelecting::AccountTabs => {
                                        app.advance_currently_selecting()
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    (_, KeyCode::Char('c')) => {
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::AccountTypeTabs
                                | CurrentlySelecting::AccountTabs => {
                                    disable_raw_mode()?;
                                    execute!(
                                        io::stdout(),
                                        Clear(ratatui::crossterm::terminal::ClearType::All),
                                        MoveTo(0, 0)
                                    )
                                    .unwrap();

                                    let new_account = create_account(
                                        app.user_id.unwrap(),
                                        app.selected_atype_tab,
                                        &app.db,
                                    );

                                    if let Some(account) = new_account { 
                                        if let Some(mut accounts) = app.accounts.take() { 
                                            accounts.push(account.0);
                                            app.accounts = Some(accounts);
                                        }
                                    }

                                    enable_raw_mode()?;
                                    terminal.clear().unwrap();

                                    app.accounts_for_type = if let Some(accounts) = &app.accounts {
                                        let filtered_accounts = accounts.iter().filter(|x| {
                                                is_account_type(x, app.selected_atype_tab)
                                        }).collect::<Vec<&Box<dyn Account>>>();
                                        let names = filtered_accounts.iter().map(|x| x.get_name()).collect::<Vec<String>>();
                                        Some(names)
                                    } else { 
                                        None
                                    };

                                    // app.restore_account();
                                    app.skip_to_last_account();
                                    app.get_account();
                                }
                                _ => {}
                            }
                        }
                    }
                    (_, KeyCode::Char('e')) => {
                        // edit account
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::Account => {
                                    disable_raw_mode()?;
                                    execute!(
                                        io::stdout(),
                                        Clear(ratatui::crossterm::terminal::ClearType::All),
                                        MoveTo(0, 0)
                                    )
                                    .unwrap();

                                    if let Some(acct) = &app.account {
                                        rename_account(
                                            &app.db,
                                            app.user_id.unwrap(),
                                            acct.get_id(),
                                        );
                                    }

                                    // update accounts for type
                                    app.accounts_for_type = app
                                        .db
                                        .get_user_accounts_by_type(
                                            app.user_id.unwrap(),
                                            app.selected_atype_tab,
                                        )
                                        .unwrap();

                                    enable_raw_mode()?;
                                    terminal.clear().unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                    (_, KeyCode::Char('m')) => {
                        // modify ledger
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::Account => {
                                    disable_raw_mode()?;
                                    execute!(
                                        io::stdout(),
                                        Clear(ratatui::crossterm::terminal::ClearType::All),
                                        MoveTo(0, 0)
                                    )
                                    .unwrap();

                                    if let Some(acct) = &mut app.account {
                                        acct.modify();
                                    } else {
                                        app.invalid_input = true;
                                    }
                                    enable_raw_mode()?;
                                    terminal.clear().unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                    (_, KeyCode::Char('r')) => {
                        // record transaction
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::Account => {
                                    disable_raw_mode()?;
                                    execute!(
                                        io::stdout(),
                                        Clear(ratatui::crossterm::terminal::ClearType::All),
                                        MoveTo(0, 0)
                                    )
                                    .unwrap();

                                    if let Some(acct) = &mut app.account {
                                        acct.record();
                                    } else {
                                        app.invalid_input = true;
                                    }

                                    enable_raw_mode()?;
                                    terminal.clear().unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                    (_, KeyCode::Char('i')) => {
                        // import transactions
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::Account => {
                                    disable_raw_mode()?;
                                    execute!(
                                        io::stdout(),
                                        Clear(ratatui::crossterm::terminal::ClearType::All),
                                        MoveTo(0, 0)
                                    )
                                    .unwrap();

                                    if let Some(acct) = &mut app.account {
                                        acct.import();
                                    } else {
                                        app.invalid_input = true;
                                    }

                                    enable_raw_mode()?;
                                    terminal.clear().unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                    (_, KeyCode::Char('a')) => {
                        // analysis period
                        disable_raw_mode()?;
                        execute!(
                            io::stdout(),
                            Clear(ratatui::crossterm::terminal::ClearType::All),
                            MoveTo(0, 0)
                        )
                        .unwrap();
                        // app.analysis_period = select_analysis_period();
                        if let Some(acct) = &app.account {
                            (app.analysis_start, app.analysis_end, app.analysis_period) = query_user_for_analysis_period(acct.get_open_date());
                        }
                        enable_raw_mode()?;
                        terminal.clear().unwrap();
                    }
                    (_, KeyCode::Char('j')) => {
                        // decrement table row
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::Account => {
                                    app.advance_ledger_table_row();
                                }
                                _ => {}
                            }
                        }
                    }
                    (_, KeyCode::Char('k')) => {
                        // decrement table row
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::Account => {
                                    app.retreat_ledger_table_row();
                                }
                                _ => {}
                            }
                        }
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                        // decrement table row
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::Account => {
                                    app.go_to_last_ledger_table_row();
                                }
                                _ => {}
                            }
                        }
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char('H')) => {
                        // decrement table row
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode {
                                CurrentlySelecting::Account => {
                                    app.go_to_first_ledger_table_row();
                                }
                                _ => {}
                            }
                        }
                    }
                    (_, KeyCode::Esc) => { 
                        if let Some(select_mode) = &app.currently_selected {
                            match select_mode { 
                                CurrentlySelecting::AccountTypeTabs|CurrentlySelecting::AccountTabs => {
                                    app.current_screen = CurrentScreen::Main;
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

#[cfg(feature = "ratatui_support")]
pub fn is_account_type( x : &Box<dyn Account>, atype : AccountType) -> bool { 
    use crate::accounts::bank_account::BankAccount;
    use crate::accounts::certificate_of_deposit::CertificateOfDepositAccount;
    use crate::accounts::credit_card_account::CreditCardAccount;
    use crate::accounts::investment_account_manager::InvestmentAccountManager;
    use crate::accounts::wallet::Wallet;
    use crate::types::accounts::AccountType;
    match atype { 
        AccountType::Bank => x.as_any().is::<BankAccount>(),                                    
        AccountType::Investment => x.as_any().is::<InvestmentAccountManager>(),
        AccountType::Wallet => x.as_any().is::<Wallet>(),
        AccountType::CD => x.as_any().is::<CertificateOfDepositAccount>(),
        AccountType::CreditCard => x.as_any().is::<CreditCardAccount>(),
    }
}
