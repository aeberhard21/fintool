use inquire::Select;
use inquire::DateSelect;
use inquire::Confirm;
use inquire::InquireError;

use crate::OffsetDateTime;
use chrono::{Datelike, Days, Local, Utc};
use chrono::{Date, NaiveDate, NaiveTime, NaiveDateTime, Weekday};
use time::{Duration};
use crate::tui::AccountOperations;
use crate::database::DbConn;
use crate::types::accounts;
use crate::types::participants::ParticipantType;
use crate::types::transfer_types;
use crate::types::transfer_types::TransferType;

use super::variable_account;
use super::variable_account::VariableAccount;

pub struct InvestmentAccountManager {
    id    : u32,
    db    : DbConn,
    variable : VariableAccount,
}


impl InvestmentAccountManager {
    pub fn new(id : u32, db : &mut DbConn) -> Self {
        Self { 
            id : id, 
            db : db.clone(),
            variable : VariableAccount::new(id, db)
        }
    }
}

impl AccountOperations for InvestmentAccountManager {
    fn create( account_id : u32, db : &mut DbConn ) {
        let mut acct = Self::new(account_id, db);
        // record several payees and payer types for use
        db.add_participant(account_id, ParticipantType::Payee, "Fixed".to_string());
        db.add_participant(account_id, ParticipantType::Payer, "Fixed".to_string());
        db.add_category(account_id, "Bought".to_string());
        db.add_category(account_id, "Cash Dividend".to_string());
        db.add_category(account_id, "Interest".to_string());
        db.add_category(account_id, "Dividend-Reinvest".to_string());
        db.add_category(account_id, "Sold".to_string());
        db.add_category(account_id, "Deposit".to_string());
        db.add_category(account_id, "Withdrawal".to_string());

        acct.record();
    }

    fn record( &mut self ) {
        loop { 
            let action = Select::new("\nWhat transaction would you like to record?", vec!["Deposit", "Withdrawal", "Purchase", "Sale"])
                .prompt().unwrap().to_string();
            match action.as_str() {
                "Deposit" => {
                    self.variable.fixed.deposit();
                }
                "Withdrawal" => {
                    self.variable.fixed.withdrawal();
                }
                "Purchase" =>  {
                    self.variable.purchase_stock();
                }
                "Sale" => {
                    self.variable.sell_stock();
                }
                _ => {
                    panic!("Unrecognized input!");
                }
            }

            let record_again = Confirm::new("Would you like to record another transaction?").prompt().unwrap();
            if !record_again { 
                return;
            }
        }
    }

    fn modify( &mut self ) {
        
    }

    fn export( &mut self ) {
        
    }

    fn report( &mut self ) {

        let periods: Vec<&str> = vec!["1 Day", "1 Week", "1 Month", "3 Months", "6 Months", "1 Year", "2 Year", "10 Year", "YTD", "Custom" ];
        let command: String = Select::new("What period would you like to analyze:", periods)
            .prompt()
            .unwrap()
            .to_string();

        let period_end = OffsetDateTime::from_unix_timestamp(Utc::now().timestamp()).unwrap();
        let mut period_start = period_end;

        match command.as_str() {
            "1 Day" => {
                period_start = period_start.checked_sub(Duration::days(1)).unwrap();
            },
            "1 Week" => {
                period_start = period_start.checked_sub(Duration::days(7)).unwrap();
            },
            "1 Month" => {
                let mut year = period_end.year();
                let mut month = period_end.month() as i32 - 1;
                let mut day = period_end.day();
                month -= 1;
                while month < 0 {
                    year -= 1;
                    month += 12;
                }
                let month_as_enum = time::Month::try_from((month+1) as u8).ok().unwrap();
                let last_day_of_month = time::util::days_in_year_month(year, month_as_enum) as i32;
                day = if day > last_day_of_month as u8 {
                    last_day_of_month as u8
                } else {
                    day
                };

                period_start = period_start.replace_year(year).unwrap();
                period_start = period_start.replace_month(month_as_enum).unwrap();
                period_start = period_start.replace_day(day).unwrap();
            },
            "3 Months" => {
                let mut year = period_end.year();
                let mut month = period_end.month() as i32 - 1;
                let mut day = period_end.day();
                month -= 3;
                while month < 0 {
                    year -= 1;
                    month += 12;
                }
                let month_as_enum = time::Month::try_from((month+1) as u8).ok().unwrap();
                let last_day_of_month = time::util::days_in_year_month(year, month_as_enum) as i32;
                day = if day > last_day_of_month as u8 {
                    last_day_of_month as u8
                } else {
                    day
                };

                period_start = period_start.replace_year(year).unwrap();
                period_start = period_start.replace_month(month_as_enum).unwrap();
                period_start = period_start.replace_day(day).unwrap();
            },
            "6 Months" => {
                let mut year = period_end.year();
                let mut month = period_end.month() as i32 - 1;
                let mut day = period_end.day();
                month -= 6;
                while month < 0 {
                    year -= 1;
                    month += 12;
                }
                let month_as_enum = time::Month::try_from((month+1) as u8).ok().unwrap();
                let last_day_of_month = time::util::days_in_year_month(year, month_as_enum) as i32;
                day = if day > last_day_of_month as u8 {
                    last_day_of_month as u8
                } else {
                    day
                };

                period_start = period_start.replace_year(year).unwrap();
                period_start = period_start.replace_month(month_as_enum).unwrap();
                period_start = period_start.replace_day(day).unwrap();        
            },
            "1 Year" => {
                let month = period_start.month();
                let year = period_start.year();
                let day = period_start.day();
                if month == time::Month::February && time::util::is_leap_year(year) && day == 29 {
                    // this handles the case of leap day
                    period_start = period_start.replace_month(time::Month::March).unwrap();
                    period_start = period_start.replace_day(1).unwrap();
                }
                period_start = period_start.replace_year(period_start.year()-1).unwrap();
            },
            "2 Year" => {
                let month = period_start.month();
                let year = period_start.year();
                let day = period_start.day();
                if month == time::Month::February && time::util::is_leap_year(year) && day == 29 {
                    // this handles the case of leap day
                    period_start = period_start.replace_month(time::Month::March).unwrap();
                    period_start = period_start.replace_day(1).unwrap();
                }
                period_start = period_start.replace_year(period_start.year()-2).unwrap();
            },
            "5 Year" => {
                let month = period_start.month();
                let year = period_start.year();
                let day = period_start.day();
                if month == time::Month::February && time::util::is_leap_year(year) && day == 29 {
                    // this handles the case of leap day
                    period_start = period_start.replace_month(time::Month::March).unwrap();
                    period_start = period_start.replace_day(1).unwrap();
                }
                period_start = period_start.replace_year(period_start.year()-5).unwrap();
            },
            "10 Year" => {
                let month = period_start.month();
                let year = period_start.year();
                let day = period_start.day();
                if month == time::Month::February && time::util::is_leap_year(year) && day == 29 {
                    // this handles the case of leap day
                    period_start = period_start.replace_month(time::Month::March).unwrap();
                    period_start = period_start.replace_day(1).unwrap();
                }
                period_start = period_start.replace_year(period_start.year()-10).unwrap();
            }
            "YTD" => {
                period_start = period_start.replace_month(time::Month::January).unwrap().replace_day(1).unwrap();
            },
            // "Custom" | _ => {
            //     let date_input: NaiveDate = DateSelect::new("Enter date").prompt().unwrap();
            //     period_start = date_input;
            //     // let time = NaiveTime::from_hms_opt(0,0,0).unwrap();
            //     // let date_time = NaiveDateTime::new(date_input.unwrap(), time);
            //     // period_start = OffsetDateTime::from_unix_timestamp(Utc.from_utc_datetime(&date_time).timestamp()).unwrap();
            // }
            _ => {
                panic!("Not found!");
            }
        }
        println!("Account value: {}", self.variable.get_current_value());
        let twr = self.variable.time_weighted_return(
            NaiveDate::parse_from_str(period_start.date().to_string().as_str(), "%Y-%m-%d").unwrap(), 
            NaiveDate::parse_from_str(period_end.date().to_string().as_str(), "%Y-%m-%d").unwrap()
        );
        println!("TWR = {}", twr);
    }

}


