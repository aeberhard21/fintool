use chrono::{NaiveDate, Weekday};
use inquire::*;

use crate::database::DbConn;
use crate::user;
use crate::database;

pub fn create_user(id: u32 , db: &mut DbConn) -> user::User<'_> {
    let name : String = Text::new("Enter user name:").prompt().unwrap();
    let admin : bool = Confirm::new("Elevate user to administrator:")
        .with_default(false)
        .prompt()
        .unwrap();
    return user::User::new(id, name.as_str(), admin, db);
}