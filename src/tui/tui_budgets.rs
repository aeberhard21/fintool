use crate::database::DbConn;
use crate::database::budget::BudgetItem;
use inquire::*;

pub fn create_budget(_aid : u32, _db: &mut DbConn) {
    let mut categories = _db.get_categories(_aid).unwrap();
    if categories.len() == 0 {
        loop {
            let item = prompt_new_budget_item(_aid, _db);
            _db.add_budget_item(_aid, item);
            let another = Confirm::new("Add another budget category?").with_default(false).prompt().unwrap();
            if another == false {
                break;
            }
        }    
    } else {
        for category in categories { 
            let cid = _db.get_category_id(_aid, category.clone()).unwrap();
            let value = set_budget_value(category.clone());
            let mut ignore = false;
            if value == 0.0 { 
                let prompt = format!("Ignore {} from budget?", category.clone());
                ignore = Confirm::new(prompt.as_str()).with_default(false).prompt().unwrap();
            }
            if !ignore {
                _db.add_budget_item(_aid, BudgetItem{ category_id : cid , value : value});
            }
        }

        let add_more = Confirm::new("Add additional budget categories?").with_default(false).prompt().unwrap();
        if add_more { 
            loop {
                let item = prompt_new_budget_item(_aid, _db);
                _db.add_budget_item(_aid, item);
                let another = Confirm::new("Add another budget category?").with_default(false).prompt().unwrap();
                if another == false {
                    break;
                }
            }    
        }
    }
}

pub fn prompt_new_budget_item(_aid : u32, _db: &mut DbConn) -> BudgetItem { 
    let category = Text::new("Enter budget category: ")
        .prompt()
        .unwrap()
        .to_string();
    let value = set_budget_value(category.clone());
    let cid = _db.add_category(_aid, category).unwrap();
    return BudgetItem { category_id : cid, value : value };
}

pub fn set_budget_value( category : String ) -> f32 { 
    let prompt = format!("Enter budgeted amount [{}]", category);
    let value = CustomType::<f32>::new(prompt.as_str())
        .with_placeholder("00000.00")
        .with_error_message("Please type a valid amount!")
        .prompt()
        .unwrap();
    return value;
}

pub fn amend_budget (_aid : u32, _db : &mut DbConn ) { 
    loop {
        let mut categories = _db.get_budget_categories(_aid).unwrap();
        categories.extend(vec!["New".to_string(), "None".to_string()]);
        let category = Select::new("Select category:", categories.clone())
            .prompt()
            .unwrap()
            .to_string();

        match category.clone().as_str() { 
            "None" => return,
            "New" => {
                let item = prompt_new_budget_item(_aid, _db);
                _db.add_budget_item(_aid, item);
            }
            _ => {
                let cid = _db.get_category_id(_aid, category.clone()).unwrap();
                let value = set_budget_value(category.clone());
                let mut delete = false;
                if value == 0.0 { 
                    let prompt = format!("Delete {} from budget?", category);
                    delete = Confirm::new(prompt.as_str()).with_default(false).prompt().unwrap();
                }
                if !delete {
                    _db.update_budget_item(_aid, BudgetItem{ category_id : cid, value : value});
                } else {
                    _db.delete_budget_item(_aid, cid);
                }
            }
        }

        let another = Confirm::new("Amend another budget category (y/n)").with_default(false).prompt().unwrap();
        if another == false {
            break;
        }
    }
}