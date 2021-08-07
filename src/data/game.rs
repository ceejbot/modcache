use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::{Cacheable, HasEtag};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModCategory {
    category_id: u16,
    name: String,
    // TODO custom deserialization
    // this is either `false` for the top-level game category or an unsigned int that
    // points to that top-level category_id for the game
    parent_category: serde_json::Value,
}

impl ModCategory {
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl Display for ModCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}: {})", self.name.yellow(), self.category_id)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
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
    etag: String,
    #[serde(skip)]
    category_map: Option<HashMap<u16, ModCategory>>,
}

impl Default for GameMetadata {
    fn default() -> Self {
        GameMetadata {
            approved_date: 0,
            authors: 0,
            categories: Vec::new(),
            domain_name: "".to_string(),
            downloads: 0,
            file_count: 0,
            file_endorsements: 0,
            file_views: 0,
            forum_url: "".to_string(),
            genre: "".to_string(),
            id: 0,
            mods: 0,
            name: "default".to_string(),
            nexusmods_url: "".to_string(),
            category_map: None,
            etag: "".to_string(),
        }
    }
}

impl GameMetadata {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn category_from_id(&mut self, id: u16) -> Option<ModCategory> {
        if self.category_map.is_none() {
            let m: HashMap<u16, ModCategory> = self
                .categories
                .iter()
                .map(|xs| (xs.category_id, xs.clone()))
                .collect();
            self.category_map = Some(m);
        }

        match &self.category_map {
            Some(m) => m.get(&id).cloned(),
            None => None,
        }
    }

    pub fn get(
        name: &str,
        refresh: bool,
        db: &kv::Store,
        nexus: &mut NexusClient,
    ) -> Option<Box<Self>> {
        if refresh {
            super::refresh::<Self, &str>(name, db, nexus)
        } else {
            super::find::<Self, &str>(name, db, nexus)
        }
    }
}

impl HasEtag for GameMetadata {
    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
    }
}

impl Cacheable<&str> for GameMetadata {
    fn bucket_name() -> &'static str {
        "games"
    }

    fn local(key: &str, db: &kv::Store) -> Option<Box<Self>> {
        let bucket = super::bucket::<Self, &str>(db).unwrap();
        let found = bucket.get(key).ok()?;
        found.map(Box::new)
    }

    fn fetch(key: &str, nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>> {
        nexus.gameinfo(key, etag).map(Box::new)
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, &str>(db).unwrap();
        if bucket.set(&*self.domain_name, self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }
}

// TODO really needs to be a macro or something
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
