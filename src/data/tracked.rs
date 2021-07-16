use log::error;
use owo_colors::OwoColorize;
use prettytable::{cell, row, Table};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use super::{Cacheable, Key};
use crate::nexus::NexusClient;

// Store and retrieve the tracked mods list.

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModReference {
    pub domain_name: String,
    pub mod_id: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct Tracked {
    pub mods: Vec<ModReference>,
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

    pub fn by_game(&self, game: String) -> Vec<&ModReference> {
        let result: Vec<&ModReference> = self
            .mods
            .iter()
            .filter(|item| item.domain_name == game)
            .collect();
        result
    }

    pub fn all(db: &kv::Store, nexus: &mut NexusClient) -> Option<Box<Self>> {
        super::find::<Tracked>(Key::Unused, db, nexus)
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

impl Cacheable for Tracked {
    fn bucket(store: &kv::Store) -> Option<kv::Bucket<'static, &'static str, Tracked>> {
        match store.bucket::<&str, Tracked>(Some("mod_ref_lists")) {
            Err(e) => {
                error!("Can't open bucket for mod reference lists {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }

    fn local(_key: Key, db: &kv::Store) -> Option<Box<Self>> {
        let bucket = Tracked::bucket(db).unwrap();
        let found = bucket.get("tracked").ok()?;
        if let Some(modref_list) = found {
            return Some(Box::new(modref_list));
        }
        None
    }

    fn fetch(_key: Key, nexus: &mut NexusClient) -> Option<Box<Self>> {
        match nexus.tracked() {
            Err(_) => None,
            Ok(tracked) => Some(Box::new(tracked)),
        }
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = Tracked::bucket(db).unwrap();
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
