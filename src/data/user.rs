use log::{error, info};
use owo_colors::OwoColorize;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::{Cacheable, Key};

#[derive(Deserialize, Serialize, Debug)]
pub struct AuthenticatedUser {
    email: String,
    is_premium: bool,
    is_supporter: bool,
    name: String,
    profile_url: String,
    user_id: u32,
    #[serde(flatten)]
    ignored: Option<HashMap<String, serde_json::Value>>,
}

impl Default for AuthenticatedUser {
    fn default() -> Self {
        AuthenticatedUser {
            name: "example".to_string(),
            user_id: 1,
            email: "foo@example.com".to_string(),
            is_premium: false,
            is_supporter: false,
            profile_url: "".to_string(),
            ignored: None,
        }
    }
}

impl Cacheable for AuthenticatedUser {
    fn fetch(_id: Key, db: &Connection, nexus: &mut NexusClient) -> Option<Box<Self>> {
        // We just always hit the nexus to validate the token.
        match nexus.validate() {
            Err(_) => None,
            Ok(user) => {
                if user.cache(db).is_ok() {
                    info!("stored your user record!");
                }
                Some(Box::new(user))
            }
        }
    }

    fn cache(&self, db: &Connection) -> anyhow::Result<usize> {
        let count = db.execute(
            r#"
            INSERT INTO authn_user
                (user_id, email, is_premium, is_supporter, name, profile_url)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT(user_id) DO NOTHING
            "#,
            params![
                self.user_id,
                self.email,
                self.is_premium,
                self.is_supporter,
                self.name,
                self.profile_url
            ],
        )?;
        Ok(count)
    }

    fn from_row(row: &Row) -> Result<Box<AuthenticatedUser>, rusqlite::Error> {
        let user = AuthenticatedUser {
            user_id: row.get(0)?,
            email: row.get(1)?,
            is_premium: row.get(2)?,
            is_supporter: row.get(3)?,
            name: row.get(4)?,
            profile_url: row.get(5)?,
            ..Default::default()
        };
        Ok(Box::new(user))
    }

    fn lookup(key: Key, db: &Connection) -> Option<Box<AuthenticatedUser>> {
        let id = match key {
            Key::IntId(v) => v,
            _ => {
                return None;
            }
        };

        // TODO handle specific errors
        match db.query_row(
            "SELECT * FROM authn_user WHERE user_id=$1",
            params![id],
            |row| AuthenticatedUser::from_row(row),
        ) {
            Err(e) => {
                error!("db query error! {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }
}

impl Display for AuthenticatedUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "    {} <{}>\n    https://www.nexusmods.com/users/{}\n    {}>",
            self.name.bold(),
            self.email.yellow(),
            self.user_id,
            self.profile_url
        )
    }
}
