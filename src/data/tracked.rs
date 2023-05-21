use std::collections::HashMap;
use std::fmt::Display;

use kv::Json;
use owo_colors::OwoColorize;
use prettytable::{row, Table};
use serde::{Deserialize, Serialize};

use super::Cacheable;
use crate::formatting::pluralize_mod;
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
            "\n{} tracked for {} games\n",
            pluralize_mod(self.mods.len()),
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

impl Cacheable for Tracked {
    type K = &'static str;

    fn bucket_name() -> &'static str {
        "mod_ref_lists"
    }

    fn get(_key: &&str, refresh: bool, nexus: &mut NexusClient) -> Option<Box<Self>> {
        super::get::<Self>(&"tracked", refresh, nexus)
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

    fn store(&self) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self>().unwrap();
        bucket.set(&self.key(), &Json(self.clone()))?;
        bucket.flush()?;
        Ok(1)
    }

    fn update(&self, other: &Self) -> Self {
        other.clone()
    }
}
