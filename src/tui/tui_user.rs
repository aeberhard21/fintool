use chrono::{NaiveDate, Weekday};
use inquire::*;

use crate::database;
use crate::database::DbConn;
use crate::user;

// pub fn create_user(id: u32 , db: &mut DbConn) -> user::User<'_> {
pub fn create_user(id: u32) -> user::User {
    let name: String = Text::new("Enter user name:").prompt().unwrap();
    let admin: bool = Confirm::new("Elevate user to administrator:")
        .with_default(false)
        .prompt()
        .unwrap();
    return user::User::new(id, name.as_str(), admin);
}
