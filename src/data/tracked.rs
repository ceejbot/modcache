use log::{error, info};
use owo_colors::OwoColorize;
use prettytable::{cell, row, Table};

use std::collections::HashMap;
use std::fmt::Display;

use super::{Cached, Key, ModReference, ModReferenceList};
use crate::nexus::NexusClient;

// Store and retrieve your list of tracked mods.

pub type Tracked = ModReferenceList;
pub type TrackedMod = ModReference;

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

    pub fn by_game() {
        todo!()
    }

    pub fn all(db: &kv::Store, nexus: &mut NexusClient) -> Option<Self> {
        let bucket = TrackedMod::bucket(db).unwrap();

        let mut items: Vec<TrackedMod> = Vec::with_capacity(2500);

        for item in bucket.iter() {
            let item = item.ok()?;
            let key: String = item.key().ok()?;
            let value = item.value::<TrackedMod>().ok()?;
            println!("key: {}, value: {}", key, value);
            items.push(value);
        }

        // TODO a way to refresh this list
        if !items.is_empty() {
            return Some(Tracked { mods: items });
        }

        match nexus.tracked() {
            Err(_) => None,
            Ok(tracked) => {
                let total = tracked.mods.iter().fold(0, |acc, tracked_mod| {
                    if tracked_mod.store(db).is_ok() {
                        acc + 1
                    } else {
                        acc
                    }
                });
                info!("stored {} tracked mods", total);
                Some(tracked)
            }
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

impl Cached for TrackedMod {
    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = TrackedMod::bucket(db).unwrap();
        let compound = format!("{}/{}", self.domain_name, self.mod_id);
        if bucket.set(&*compound, self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn find(key: Key, db: &kv::Store, _nexus: &mut NexusClient) -> Option<Box<Self>> {
        let (game, mod_id) = match key {
            Key::NameIdPair { name, id } => (name, id),
            _ => {
                return None;
            }
        };
        let bucket = TrackedMod::bucket(db).unwrap();
        let compound = format!("{}/{}", game, mod_id);
        let found = bucket.get(&*compound).ok()?;
        if let Some(modref) = found {
            return Some(Box::new(modref));
        }
        None
    }

    fn bucket(store: &kv::Store) -> Option<kv::Bucket<'static, &'static str, TrackedMod>> {
        match store.bucket::<&str, TrackedMod>(Some("tracked")) {
            Err(e) => {
                error!("Can't open bucket for tracked mods list {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }
}

impl Display for TrackedMod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.domain_name.yellow(), self.mod_id.blue())
    }
}

// this has to be auto-generatable with a macro
impl kv::Value for TrackedMod {
    fn to_raw_value(&self) -> Result<kv::Raw, kv::Error> {
        let x = serde_json::to_vec(&self)?;
        Ok(x.into())
    }

    fn from_raw_value(r: kv::Raw) -> Result<Self, kv::Error> {
        let x: Self = serde_json::from_slice(&r)?;
        Ok(x)
    }
}
