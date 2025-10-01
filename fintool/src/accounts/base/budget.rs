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
use std::collections::HashMap;
use std::hash::Hash;
use std::{rc, string};

use crate::accounts::base::AnalysisPeriod;
use crate::database::budget::{BudgetItem, BudgetRecord};
use crate::database::DbConn;
use crate::types::categories::CategoryAutoCompleter;
use chrono::{Duration, NaiveDate};
use inquire::autocompletion::Replacement;
use inquire::*;

pub struct Budget {
    uid: u32,
    aid: u32,
    db: DbConn,
}

impl Budget {
    pub fn new(uid: u32, aid: u32, db: &DbConn) -> Self {
        let budget = Self {
            uid: uid,
            aid: aid,
            db: db.clone(),
        };
        budget
    }

    pub fn create_budget(&self) {
        loop {
            let item = self.prompt_new_budget_item();
            self.db.add_budget_item(self.uid, self.aid, item);
            let another = Confirm::new("Add another budget category?")
                .with_default(false)
                .prompt()
                .unwrap();
            if another == false {
                break;
            }
        }
    }

    pub fn prompt_new_budget_item(&self) -> BudgetItem {
        let category = Text::new("Enter budget category: ")
            .with_autocomplete(CategoryAutoCompleter {
                uid: self.uid,
                aid: self.aid,
                db: self.db.clone(),
                cats: None,
            })
            .prompt()
            .unwrap()
            .to_string();
        let value = self.set_budget_value(category.clone());
        let cid = self.db.add_category(self.uid, self.aid, category).unwrap();
        return BudgetItem {
            category_id: cid,
            value: value,
        };
    }

    pub fn set_budget_value(&self, category: String) -> f32 {
        let prompt = format!("Enter budgeted amount [{}]", category);
        let value = CustomType::<f32>::new(prompt.as_str())
            .with_placeholder("00000.00")
            .with_error_message("Please type a valid amount!")
            .prompt()
            .unwrap();
        return value;
    }

    fn select_budget_element(&self) -> Option<BudgetRecord> {
        let records = self.db.get_budget(self.uid, self.aid).unwrap();
        if let Some(records) = records {
            if records.is_empty() {
                println!("Budget not found for account: '{}'!", self.aid);
                return None;
            }

            let mut strings: Vec<String> = Vec::new();
            let mut mapped_records: HashMap<u32, BudgetItem> = HashMap::new();
            let mut entries: HashMap<String, u32> = HashMap::new();
            for rcrd in records {
                let v = format!(
                    "{} | {}",
                    self.db
                        .get_category_name(self.uid, self.aid, rcrd.item.category_id)
                        .unwrap(),
                    rcrd.item.value
                );
                strings.push(v.clone());
                entries.insert(v.clone(), rcrd.id);
                mapped_records.insert(rcrd.id, rcrd.item);
            }

            strings.push("None".to_string());

            let selected = Select::new("What item would you like to modify:", strings)
                .prompt()
                .unwrap()
                .to_string();

            if selected == "None".to_string() {
                None
            } else {
                let id = *entries
                    .get(&selected)
                    .expect(format!("Unable to find matching ID for {}", selected).as_str());
                let selected_record = BudgetRecord {
                    id: id,
                    item: mapped_records
                        .get(&id)
                        .expect(format!("Budget item not found matching {}!", id).as_str())
                        .to_owned(),
                };
                Some(selected_record)
            }
        } else {
            None
        }
    }

    pub fn record(&self) {
        const OPTIONS: [&'static str; 3] = ["Full Budget", "Budget Item", "None"];
        let record_choice = Select::new("What would you like to do:", OPTIONS.to_vec())
            .prompt()
            .unwrap();
        match record_choice {
            "Full Budget" => {
                let budget_opt = self.db.get_budget(self.uid, self.aid).unwrap();
                if let Some(budget) = budget_opt {
                    if !budget.is_empty() {
                        let go_ahead = Confirm::new("This action will delete current budget data. Do you want to continue (y/n)?")
                            .with_default(false)
                            .prompt()
                            .unwrap();

                        if !go_ahead {
                            return;
                        }

                        for item in budget {
                            self.db.remove_budget_item(self.uid, self.aid, item.id);
                        }
                    }
                }
                self.create_budget();
            }
            "Budget Item" => {
                let new = self.prompt_new_budget_item();
                let _ = self.db.add_budget_item(self.uid, self.aid, new);
            }
            "None" => {}
            _ => {
                panic!("Unrecognized input: '{}'!", record_choice);
            }
        }
    }

    pub fn modify(&self) {
        const OPTIONS: [&'static str; 3] = ["Update", "Remove", "None"];
        loop {
            let record_opt = self.select_budget_element();
            if let Some(selected_record) = record_opt {
                let modify_choice = Select::new("What would you like to do:", OPTIONS.to_vec())
                    .prompt()
                    .unwrap();

                match modify_choice {
                    "Update" => {
                        let updated_value = self.set_budget_value(
                            self.db
                                .get_category_name(
                                    self.uid,
                                    self.aid,
                                    selected_record.item.category_id,
                                )
                                .unwrap(),
                        );
                        let updated_record = BudgetRecord {
                            id: selected_record.id,
                            item: BudgetItem {
                                category_id: selected_record.item.category_id,
                                value: updated_value,
                            },
                        };
                        self.db
                            .update_budget_item(self.uid, self.aid, updated_record);
                    }
                    "Remove" => {
                        self.db
                            .remove_budget_item(self.uid, self.aid, selected_record.id);
                    }
                    "None" => {}
                    _ => {
                        panic!("Unrecognized input: '{}'", modify_choice);
                    }
                };

                let another = Confirm::new("Amend another budget category (y/n)")
                    .with_default(false)
                    .prompt()
                    .unwrap();
                if another == false {
                    break;
                }
            } else {
                return;
            }
        }
    }

    pub fn get_budget(&self) -> Vec<BudgetRecord> {
        let budget = self.db.get_budget(self.uid, self.aid).unwrap();
        if let Some(budget) = budget {
            budget
        } else {
            Vec::new()
        }
    }

    pub fn get_budget_categories(&self) -> Vec<String> {
        let categories = self.db.get_budget_categories(self.uid, self.aid).unwrap();
        if let Some(categories) = categories {
            categories
        } else {
            Vec::new()
        }
    }
}

pub fn scale_budget_value_to_analysis_period(value: f32, start: NaiveDate, end: NaiveDate) -> f32 {
    let diff = (end - start).num_days() as f32;
    return value * diff / 31.;
}
