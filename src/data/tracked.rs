use owo_colors::OwoColorize;
use prettytable::{cell, row, Table};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use super::{Cacheable, HasEtag};
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

    pub fn get(refresh: bool, db: &kv::Store, nexus: &mut NexusClient) -> Option<Box<Self>> {
        if refresh {
            super::refresh::<Self, ()>((), db, nexus)
        } else {
            super::find::<Self, ()>((), db, nexus)
        }
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
            let length = v.len().to_string();
            table.add_row(row![&length.bold(), k]);
        }
        write!(f, "{}", table)
    }
}

impl HasEtag for Tracked {
    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
    }
}

impl Cacheable<()> for Tracked {
    fn bucket_name() -> &'static str {
        "mod_ref_lists"
    }

    fn local(_key: (), db: &kv::Store) -> Option<Box<Self>> {
        let bucket = super::bucket::<Self, ()>(db).unwrap();
        let found = bucket.get("tracked").ok()?;
        if let Some(modref_list) = found {
            return Some(Box::new(modref_list));
        }
        None
    }

    fn fetch(_key: (), nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>> {
        nexus.tracked(etag).map(Box::new)
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, ()>(db).unwrap();
        if bucket.set("tracked", self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }
}

// this has to be auto-generatable with a macro
impl kv::Value for Tracked {
    fn to_raw_value(&self) -> Result<kv::Raw, kv::Error> {
        let x = serde_json::to_vec(&self)?;
        Ok(x.into())
    }

    fn from_raw_value(r: kv::Raw) -> Result<Self, kv::Error> {
        let x: Self = serde_json::from_slice(&r)?;
        Ok(x)
    }
}
