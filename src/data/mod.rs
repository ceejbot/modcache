use log::{error, info};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use std::fmt::Display;

use crate::nexus::NexusClient;

pub mod modinfo;
pub mod tracked;
pub mod user;

pub use modinfo::*;
pub use tracked::*;
pub use user::*;

// Nexus mod data structs and trait implementations, plus caching layer.
// More complex structures are broken out into separate files.

pub trait Cacheable
where
    Self: kv::Value,
{
    fn find(id: Key, store: &kv::Store, nexus: &mut NexusClient) -> Option<Box<Self>>;
    fn bucket(store: &kv::Store) -> Option<kv::Bucket<'static, &'static str, Self>>;
    fn store(&self, db: &kv::Store) -> anyhow::Result<usize>;
}

#[derive(Debug, Clone)]
pub enum Key {
    Name(String),
    IntId(u32),
    NameIdPair { name: String, id: u32 },
    Unused,
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

#[derive(Deserialize, Serialize, Debug, Clone)]
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

impl kv::Value for GameMetadata {
    fn to_raw_value(&self) -> Result<kv::Raw, kv::Error> {
        let x = serde_json::to_vec(&self)?;
        Ok(x.into())
    }

    fn from_raw_value(r: kv::Raw) -> Result<Self, kv::Error> {
        let x: Self = serde_json::from_slice(&r)?;
        Ok(x)
    }
}

impl Cacheable for GameMetadata {
    fn find(key: Key, db: &kv::Store, nexus: &mut NexusClient) -> Option<Box<Self>> {
        let id = match key {
            Key::Name(v) => v,
            _ => {
                return None;
            }
        };
        let bucket = GameMetadata::bucket(db).unwrap();
        let found = bucket.get(&*id).ok()?;
        if let Some(game) = found {
            info!("cache hit for {}", id);
            return Some(Box::new(game));
        }

        if let Ok(game) = nexus.gameinfo(&id) {
            if game.store(db).is_ok() {
                info!("cached record for {}", id);
            }
            return Some(Box::new(game));
        }

        None
    }

    fn bucket(db: &kv::Store) -> Option<kv::Bucket<'static, &'static str, Self>> {
        match db.bucket::<&str, Self>(Some("games")) {
            Err(e) => {
                error!("Can't open bucket for game metadata! {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }
    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = GameMetadata::bucket(db).unwrap();
        if bucket.set(&*self.domain_name, self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }
}

// write!(f, "    {} <{}\n    https://www.nexusmods.com/{}/mods/categories/{}\n    {}>",

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct EndorsementList {
    pub mods: Vec<UserEndorsement>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct ModInfoList {
    pub mods: Vec<ModInfoFull>,
}
