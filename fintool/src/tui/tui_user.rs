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
use crate::database::DbConn;
use inquire::*;

pub fn create_user(_db: &mut DbConn) -> u32 {
    let mut name: String;
    loop {
        name = Text::new("Enter user name:").prompt().unwrap();
        if name.len() == 0 {
            println!("Invalid user name!");
        } else {
            break;
        }
    }
    let admin: bool = Confirm::new("Elevate user to administrator:")
        .with_default(false)
        .prompt()
        .unwrap();
    _db.add_user(name, admin).unwrap()
}

pub fn create_admin(_db: &mut DbConn) -> u32 {
    let mut name: String;
    loop {
        name = Text::new("Enter admin name:").prompt().unwrap();
        if name.len() == 0 {
            println!("Invalid administrator name!");
        } else {
            break;
        }
    }
    _db.add_user(name, true).unwrap()
}

pub fn tui_set_user(_db: &mut DbConn) -> u32 {
    let id: u32;
    let users = _db.get_users().unwrap();
    if users.is_empty() {
        id = create_admin(_db);
    } else {
        let name: String = Select::new("Select current user:", users.to_vec())
            .prompt()
            .unwrap()
            .to_string();
        println!("Welcome {}!", name);
        let rid = _db.get_user_id(name);
        match rid {
            Ok(rid) => {
                id = rid;
            }
            Err(error) => {
                panic!("Error is {}", error);
            }
        }
    }
    return id;
}
