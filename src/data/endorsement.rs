use log::error;
use owo_colors::OwoColorize;
use prettytable::{cell, row, Table};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::{Cacheable, Key};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum EndorsementStatus {
    Endorsed,
    Undecided,
    Abstained,
}

impl Display for EndorsementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndorsementStatus::Endorsed => write!(f, "👍🏻"),
            EndorsementStatus::Undecided => write!(f, "🤨"),
            _ => write!(f, "🚫"),
        }
    }
}

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

impl EndorsementList {
    // Note similarities with the functions for Tracked.

    pub fn get_game_map(&self) -> HashMap<String, Vec<UserEndorsement>> {
        let mut mapping: HashMap<String, Vec<UserEndorsement>> = HashMap::new();
        self.mods.iter().for_each(|item| {
            if !mapping.contains_key(&item.domain_name) {
                mapping.insert(item.domain_name.clone(), Vec::new());
            }
            mapping
                .get_mut(&item.domain_name)
                .unwrap()
                .push(item.clone());
        });

        mapping
    }

    pub fn by_game(&self, game: String) -> Vec<&UserEndorsement> {
        let result: Vec<&UserEndorsement> = self
            .mods
            .iter()
            .filter(|item| item.domain_name == game)
            .collect();
        result
    }

    pub fn all(db: &kv::Store, nexus: &mut NexusClient) -> Option<Box<Self>> {
        super::find::<Self>(Key::Unused, db, nexus)
    }
}

impl Display for EndorsementList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mapping = self.get_game_map();
        writeln!(
            f,
            "\n{} mods opinionated upon for {} games\n",
            self.mods.len().red(),
            mapping.len().blue()
        )?;

        // This display is pretty useless, but leaving it for now.
        for (k, v) in mapping.iter() {
            // TODO look up the game referenced. Which requires figuring out how to get the data *here*.
            let mut table = Table::new();
            table.set_format(*prettytable::format::consts::FORMAT_CLEAN);
            let countstr = if v.len() == 1 {
                "one mod".to_string()
            } else {
                format!("{} mods", v.len().bold())
            };
            table.add_row(row![k.yellow().bold(), countstr]);
            v.iter().for_each(|opinion| {
                // TODO! Look up the mod referenced.
                table.add_row(row![
                    format!("{}", opinion.status()),
                    format!(
                        "https://www.nexusmods.com/{}/mods/{}",
                        opinion.domain_name, opinion.mod_id
                    ),
                ]);
            });
            writeln!(f, "{}", table)?;
        }
        Ok(())
    }
}

impl Cacheable for EndorsementList {
    fn bucket(db: &kv::Store) -> Option<kv::Bucket<'static, &'static str, Self>> {
        match db.bucket::<&str, EndorsementList>(Some("endorsements")) {
            Err(e) => {
                error!("Can't open bucket for endorsements {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }

    fn local(_key: Key, db: &kv::Store) -> Option<Box<Self>> {
        let bucket = EndorsementList::bucket(db).unwrap();
        let found = bucket.get("endorsements").ok()?;
        if let Some(modref_list) = found {
            return Some(Box::new(modref_list));
        }
        None
    }

    fn fetch(_key: Key, nexus: &mut crate::nexus::NexusClient) -> Option<Box<Self>> {
        match nexus.endorsements() {
            Err(_) => None,
            Ok(v) => Some(Box::new(v)),
        }
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = EndorsementList::bucket(db).unwrap();
        if bucket.set("endorsements", self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }
}

// this has to be auto-generatable with a macro
impl kv::Value for EndorsementList {
    fn to_raw_value(&self) -> Result<kv::Raw, kv::Error> {
        let x = serde_json::to_vec(&self)?;
        Ok(x.into())
    }

    fn from_raw_value(r: kv::Raw) -> Result<Self, kv::Error> {
        let x: Self = serde_json::from_slice(&r)?;
        Ok(x)
    }
}
