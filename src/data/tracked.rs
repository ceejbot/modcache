use log::{error, info};
use owo_colors::OwoColorize;
use prettytable::{cell, row, Table};
use rusqlite::{params, Connection, Row};

use std::collections::HashMap;
use std::fmt::Display;

use crate::nexus::NexusClient;
use super::{Cacheable, Key, ModReference, ModReferenceList};

// Store and retrieve your list of tracked mods.

pub type Tracked = ModReferenceList;

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
}

impl Cacheable for Tracked {
    // This experiment in warping my API might not pan out.
    fn fetch(key: Key, db: &Connection, nexus: &mut NexusClient) -> Option<Box<ModReferenceList>> {
        // NOTE that this never updates data ever so it's not perfect. As one says.
        let hit = Self::lookup(key, db);
        if hit.is_some() {
            return hit;
        }

        match nexus.tracked() {
            Err(_) => None,
            Ok(tracked) => {
                if let Ok(total) = tracked.cache(db) {
                    info!("stored {} tracked mods", total);
                }
                Some(Box::new(tracked))
            }
        }
    }

    fn lookup(_id: Key, db: &Connection) -> Option<Box<Self>> {
        let mut result = Tracked { mods: Vec::new() };

        let mut stmt = db
            .prepare(r#"SELECT domain_name, mod_id FROM tracked"#)
            .ok()?;
        if let Ok(mut rows) = stmt.query([]) {
            while let Some(row) = rows.next().ok()? {
                result.mods.push(ModReference {
                    domain_name: row.get(0).ok()?,
                    mod_id: row.get(1).ok()?,
                });
            }
        } else {
            return None;
        }

        Some(Box::new(result))
    }

    fn cache(&self, db: &Connection) -> anyhow::Result<usize> {
        let mut total = 0;
        let mut stmt = db.prepare(
            "INSERT INTO tracked (domain_name, mod_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )?;
        self.mods.iter().for_each(|item| {
            let count = match stmt.execute(params![item.domain_name, item.mod_id]) {
                Err(e) => {
                    error!("{:#?}", e);
                    0
                }
                Ok(v) => v,
            };
            total += count;
        });
        Ok(total)
    }

    fn from_row(row: &Row) -> Result<Box<ModReferenceList>, rusqlite::Error> {
        let one = ModReference {
            domain_name: row.get(0)?,
            mod_id: row.get(1)?,
        };
        let mut this_is_bad = ModReferenceList { mods: Vec::new() };
        this_is_bad.mods.push(one);

        Ok(Box::new(this_is_bad))
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
