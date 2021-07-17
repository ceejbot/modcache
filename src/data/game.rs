use log::error;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::Cacheable;

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

impl GameMetadata {
    pub fn name(&self) -> String {
        self.name.clone()
    }
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

impl Cacheable<&str> for GameMetadata {
    fn bucket(db: &kv::Store) -> Option<kv::Bucket<'static, &'static str, Self>> {
        match db.bucket::<&str, Self>(Some("games")) {
            Err(e) => {
                error!("Can't open bucket for game metadata! {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }

    fn local(key: &str, db: &kv::Store) -> Option<Box<Self>> {
        let bucket = GameMetadata::bucket(db).unwrap();
        let found = bucket.get(key).ok()?;
        found.map(Box::new)
    }

    fn fetch(key: &str, nexus: &mut NexusClient) -> Option<Box<Self>> {
        if let Ok(game) = nexus.gameinfo(key) {
            Some(Box::new(game))
        } else {
            None
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
