use log::{debug, error, info, warn};
use owo_colors::OwoColorize;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use std::fmt::Display;

use crate::nexus::NexusClient;

pub mod modinfo;
pub mod user;
pub mod tracked;

pub use modinfo::*;
pub use user::*;
pub use tracked::*;

// Nexus mod data structs and trait implementations, plus caching layer.
// More complex structures are broken out into separate files.

#[derive(Debug, Clone)]
pub enum Key {
    Name(String),
    IntId(u32),
    NameIdPair { name: String, id: u32 },
    Unused,
}

pub trait Cacheable {
    /// Get the item, trying the cache first.
    // this api is still wrong, but getting closer
    fn fetch(id: Key, db: &Connection, nexus: &mut NexusClient) -> Option<Box<Self>>;
    /// Try to find the item in the cache.
    fn lookup(id: Key, db: &Connection) -> Option<Box<Self>>;
    // Store an item in the cache.
    fn cache(&self, db: &Connection) -> anyhow::Result<usize>;
    /// Inflate a single instance of this item from a db row. Implementation detail.
    fn from_row(row: &Row) -> Result<Box<Self>, rusqlite::Error>;
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModCategory {
    category_id: u16,
    name: String,
    // TODO custom deserialization
    // this is either `false` for the top-level game category or an unsigned int that
    // points to that top-level category_id for the game
    parent_category: serde_json::Value,
}

impl Display for ModCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}: {})", self.name.yellow(), self.category_id)
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GameMetadata {
    approved_date: u64,
    authors: u32,
    categories: Vec<ModCategory>,
    domain_name: String,
    downloads: u64,
    file_count: u32,
    file_endorsements: u32,
    file_views: u64,
    forum_url: String,
    genre: String,
    id: u32,
    mods: u32,
    name: String,
    nexusmods_url: String,
}

impl Cacheable for GameMetadata {
    fn fetch(key: Key, db: &Connection, nexus: &mut NexusClient) -> Option<Box<Self>> {
        let hit = Self::lookup(key.clone(), db);
        if hit.is_some() {
            return hit;
        }

        let id = match key {
            Key::Name(v) => v,
            _ => {
                return None;
            }
        };

        match nexus.gameinfo(&id) {
            Err(_) => None,
            Ok(game) => {
                if game.cache(db).is_ok() {
                    warn!("stored {}!", game.domain_name.bright_yellow());
                }
                Some(Box::new(game))
            }
        }
    }

    fn lookup(key: Key, db: &Connection) -> Option<Box<GameMetadata>> {
        let id = match key {
            Key::Name(v) => v,
            _ => {
                return None;
            }
        };

        let mut game = match db.query_row(
            "SELECT * FROM games WHERE domain_name=$1",
            params![id],
            |row| GameMetadata::from_row(row),
        ) {
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                info!("cache miss");
                return None;
            }
            Err(e) => {
                error!("error reading{:?}", e);
                return None;
            }
            Ok(v) => v,
        };

        let mut stmt = db
            .prepare("SELECT category_id, name FROM categories WHERE domain_name = $1")
            .ok()?;
        match stmt.query_map(params![game.domain_name], |row| {
            Ok(ModCategory {
                category_id: row.get(0)?,
                name: row.get(1)?,
                parent_category: serde_json::Value::Null,
            })
        }) {
            Err(e) => {
                error!("error reading categories: {:?}", e);
            }
            Ok(rows) => {
                game.categories = rows.filter_map(|xs| xs.ok()).collect();
            }
        };

        debug!("cache hit for {}", game.domain_name);
        Some(game)
    }

    fn cache(&self, db: &Connection) -> anyhow::Result<usize> {
        // TODO on conflict update count fields
        let count = db.execute(r#"INSERT INTO games
                (id, domain_name, name, approved_date, authors, downloads, file_count, file_endorsements, file_views, forum_url, genre, mods, nexusmods_url)
                VALUES (?1, ?2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                ON CONFLICT(domain_name) DO NOTHING"#,
            params![self.id, self.domain_name, self.name, self.approved_date, self.authors,
                self.downloads, self.file_count, self.file_endorsements, self.file_views,
                self.forum_url, self.genre, self.mods, self.nexusmods_url],
        )?;
        info!("{} stored", self.domain_name.yellow().bold());

        // Cleanup on aisle 4 please.
        self.categories.clone().into_iter().for_each(|category| {
            match db.execute(
                r#"
                INSERT INTO categories
                    (category_id, domain_name, name)
                    VALUES ($1, $2, $3)
                ON CONFLICT(category_id, domain_name) DO UPDATE SET name=excluded.name"#,
                params![category.category_id, self.domain_name, category.name],
            ) {
                Ok(_) => info!("   + category {}", category.name.green()),
                Err(e) => error!("{:?}", e),
            };
        });

        Ok(count)
    }

    fn from_row(row: &Row) -> Result<Box<GameMetadata>, rusqlite::Error> {
        let game = GameMetadata {
            id: row.get(0)?,
            domain_name: row.get(1)?,
            name: row.get(2)?,
            approved_date: row.get(3)?,
            authors: row.get(4)?,
            downloads: row.get(5)?,
            file_count: row.get(6)?,
            file_endorsements: row.get(7)?,
            file_views: row.get(8)?,
            forum_url: row.get(9)?,
            genre: row.get(10)?,
            mods: row.get(11)?,
            nexusmods_url: row.get(12)?,
            categories: Vec::new(),
        };
        Ok(Box::new(game))
    }
}

// write!(f, "    {} <{}\n    https://www.nexusmods.com/{}/mods/categories/{}\n    {}>",

#[derive(Serialize, Deserialize, Debug)]
pub struct UserEndorsement {
    date: u64,
    domain_name: String,
    mod_id: u32,
    status: EndorsementStatus,
    version: String,
}

impl UserEndorsement {
    pub fn status(&self) -> &EndorsementStatus {
        &self.status
    }
}

impl Display for UserEndorsement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}/{}",
            self.status,
            self.domain_name.yellow().bold(),
            self.mod_id
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct EndorsementList {
    pub mods: Vec<UserEndorsement>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct ModInfoList {
    pub mods: Vec<ModInfoFull>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModReference {
    pub domain_name: String,
    pub mod_id: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct ModReferenceList {
    pub mods: Vec<ModReference>,
}

