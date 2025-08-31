use super::DbConn;
use rusqlite::{params, Error};

impl DbConn {
    pub fn create_users_id_table(&self) -> rusqlite::Result<()> {
        let sql = "
            CREATE TABLE IF NOT EXISTS user_ids (
            next_user_id INTEGER NOT NULL PRIMARY KEY
        )";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("{}", error);
            }
        }
        let sql: &str = "SELECT * FROM user_ids";
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(())?;
        if !exists {
            let sql = "INSERT INTO user_ids (next_user_id) VALUES (?1)";
            match conn_lock.execute(sql, [0]) {
                Ok(_rows_inserted) => {}
                Err(error) => {
                    panic!("Unable to initialize user_ids table: {}", error);
                }
            }
        }
        Ok(())
    }

    pub fn get_next_user_id(&self) -> rusqlite::Result<u32> {
        let sql = "SELECT next_user_id FROM user_ids";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(())?;
        match exists {
            true => {
                let id = stmt.query_row((), |row| row.get::<_, u32>(0))?;
                let sql = "UPDATE user_ids SET next_user_id = next_user_id + 1";
                conn_lock.execute(sql, ())?;
                Ok(id)
            }
            false => {
                panic!("The next user ID within table 'info' does not exist.");
            }
        }
    }

    pub fn create_user_table(&self) -> rusqlite::Result<()> {
        let sql: &str;
        sql = "CREATE TABLE IF NOT EXISTS users (
                id          INTEGER NOT NULL PRIMARY KEY, 
                name        TEXT    NOT NULL,
                admin       BOOL    NOT NULL
            )";
        let conn_lock = self.conn.lock().unwrap();
        let rs = conn_lock.execute(sql, ());
        match rs {
            Ok(_) => {}
            Err(error) => {
                panic!("Unable to create users table: {}!", error);
            }
        }
        Ok(())
    }

    pub fn add_user(&self, name: String, admin: bool) -> rusqlite::Result<u32, Error> {
        let sql: &str = "INSERT INTO users (id, name, admin) VALUES ( ?1, ?2, ?3)";
        let id = self.get_next_user_id().unwrap();
        let p = rusqlite::params![id, name, admin];
        {
            let conn_lock = self.conn.lock().unwrap();
            let rs = conn_lock.execute(sql, p);
            match rs {
                Ok(_rows_inserted) => {}
                Err(error) => {
                    panic!("Unable to allocate user: '{}'!", error);
                }
            }
        }
        self.initialize_user_account_table(id).unwrap();
        Ok(id)
    }

    pub fn get_users(&self) -> rusqlite::Result<Vec<String>, rusqlite::Error> {
        let sql: &str = "SELECT * FROM users";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists(())?;
        let mut users: Vec<String> = Vec::new();
        match exists {
            true => {
                let sql: &str = "SELECT name from users";
                let mut rs: rusqlite::Statement<'_> = conn_lock.prepare(sql).unwrap();
                let names: Vec<Result<String, Error>> = rs
                    .query_map([], |row| Ok(row.get(0)?))
                    .unwrap()
                    .collect::<Vec<_>>();

                for name in names {
                    users.push(name.unwrap());
                }
                return Ok(users);
            }
            false => {
                return Ok(users);
            }
        }
    }

    pub fn get_user_id(&self, name: String) -> rusqlite::Result<u32, rusqlite::Error> {
        let sql: &str = "SELECT id FROM users WHERE name = (?1)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists((&name,))?;
        match exists {
            true => {
                let sql: &str = "SELECT id from users WHERE name = (?1)";
                let mut stmt = conn_lock.prepare(sql)?;
                let id = stmt.query_row((&name,), |row| row.get::<_, u32>(0));
                match id {
                    Ok(id) => {
                        return Ok(id);
                    }
                    Err(err) => {
                        panic!("Unable to retrieve id for user {}: {}", &name, err);
                    }
                }
            }
            false => {
                panic!("Unable to find user {}!", name);
            }
        }
    }

    pub fn is_admin(&self, uid: u32) -> rusqlite::Result<bool, Error> {
        let sql: &str = "SELECT admin FROM users WHERE id = (?1)";
        let conn_lock = self.conn.lock().unwrap();
        let mut stmt = conn_lock.prepare(sql)?;
        let exists = stmt.exists((&uid,))?;
        match exists {
            true => {
                let admin = stmt.query_row((&uid,), |row| row.get::<_, bool>(0))?;
                Ok(admin)
            }
            false => {
                panic!("Unable to find user {}!", uid);
            }
        }
    }
}
