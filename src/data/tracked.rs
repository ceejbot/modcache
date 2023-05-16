use kv::Json;
use owo_colors::OwoColorize;
use prettytable::{row, Table};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use super::Cacheable;
use crate::nexus::NexusClient;

// Store and retrieve the tracked mods list.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModReference {
    pub domain_name: String,
    pub mod_id: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tracked {
    pub mods: Vec<ModReference>,
    pub etag: String,
}

impl Display for ModReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.domain_name.yellow(), self.mod_id.blue())
    }
}

impl Tracked {
    pub fn get_game_map(&self) -> HashMap<String, Vec<u32>> {
        let mut mapping: HashMap<String, Vec<u32>> = HashMap::new();
        self.mods.iter().for_each(|item| {
            if !mapping.contains_key(&item.domain_name) {
                mapping.insert(item.domain_name.clone(), Vec::new());
            }
            mapping
                .get_mut(&item.domain_name)
                .unwrap()
                .push(item.mod_id);
        });

        mapping
    }

    pub fn by_game(&self, game: &str) -> Vec<&ModReference> {
        let result: Vec<&ModReference> = self
            .mods
            .iter()
            .filter(|item| item.domain_name == game)
            .collect();
        result
    }

    pub fn listkey() -> &'static str {
        "tracked"
    }
}

impl Display for Tracked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mapping = self.get_game_map();
        writeln!(
            f,
            "\n{} mods tracked for {} games\n",
            self.mods.len().red(),
            mapping.len().blue()
        )?;

        let mut table = Table::new();
        table.set_format(*prettytable::format::consts::FORMAT_CLEAN);
        for (k, v) in mapping.iter() {
            table.add_row(row![v.len().bold(), k]);
        }
        write!(f, "{table}")
    }
}

impl Cacheable<&str> for Tracked {
    fn bucket_name() -> &'static str {
        "mod_ref_lists"
    }

    fn get(
        key: &&str,
        refresh: bool,
        db: &kv::Store,
        nexus: &mut NexusClient,
    ) -> Option<Box<Self>> {
        super::get::<Self, &str>(key, refresh, db, nexus)
    }

    fn fetch(_key: &&str, nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>> {
        nexus.tracked(etag).map(Box::new)
    }

    fn key(&self) -> &'static str {
        "tracked"
    }

    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, &str>(db).unwrap();
        if bucket.set(&self.key(), &Json(self.clone())).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn update(&self, other: &Self) -> Self {
        other.clone()
    }
}
