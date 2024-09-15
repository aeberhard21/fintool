use super::DbConn;
use rusqlite::Result;

#[derive(Clone, Copy)]
pub enum PeopleType {
    Payee,
    Payer,
}

impl PeopleType {
    fn to_string(self) -> String {
        match self {
            Self::Payee => "Payee".to_string(),
            Self::Payer => "Payer".to_string(),
        }
    }
}

impl DbConn {
    pub fn create_people_table(&mut self) -> Result<()> {
        let sql: &str = "CREATE TABLE IF NOT EXISTS people ( 
                id          INTEGER NOT NULL PRIMARY KEY,
                aid         INTEGER NOT NULL,
                type        INTEGER NOT NULL, 
                name        TEXT NOT NULL,
                uid         INTEGER,
                FOREIGN KEY(aid) REFERENCES accounts(id)
                FOREIGN KEY(uid) REFERENCES users(id)
            )";

        self.conn
            .execute(sql, ())
            .expect("Unable to initialize people table!");
        Ok(())
    }

    pub fn add_person(&mut self, aid: u32, ptype: PeopleType, people: String) -> Result<u32> {
        let id = self.get_next_people_id().unwrap();
        let p = rusqlite::params!(id, aid, ptype as u32, people);
        let sql = "INSERT INTO people (id, aid, type, name) VALUES (?1, ?2, ?3, ?4)";
        match self.conn.execute(sql, p) {
            Ok(_) => Ok(id),
            Err(error) => {
                panic!(
                    "Unable to add {} {} for account {}: {}",
                    ptype.to_string(),
                    people,
                    aid,
                    error
                );
            }
        }
    }

    pub fn get_people(
        &mut self,
        aid: u32,
        ptype: PeopleType,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let p = rusqlite::params![aid, ptype as u32];
        let sql = "SELECT name FROM people WHERE aid = (?1) and type = (?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        let mut people: Vec<String> = Vec::new();
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let peeps = stmt
                    .query_map(p, |row| Ok(row.get(0)?))
                    .unwrap()
                    .collect::<Vec<_>>();
                for peep in peeps {
                    people.push(peep.unwrap());
                }
            }
            false => {}
        }
        Ok(people)
    }

    pub fn get_person_id(
        &mut self,
        aid: u32,
        name: String,
    ) -> rusqlite::Result<u32, rusqlite::Error> {
        let sql: &str = "SELECT id FROM people WHERE aid = (?1) AND name = (?2)";
        let p = rusqlite::params![aid, name];
        let mut stmt = self.conn.prepare(sql)?;
        let exists = stmt.exists(p)?;
        match exists {
            true => {
                stmt = self.conn.prepare(sql)?;
                let id = stmt.query_row(p, |row| row.get::<_, u32>(0));
                match id {
                    Ok(id) => {
                        return Ok(id);
                    }
                    Err(err) => {
                        panic!("Unable to retrieve id for account {}: {}", aid, err);
                    }
                }
            }
            false => {
                panic!("Unable to find account matching {}", aid);
            }
        }
    }
}
