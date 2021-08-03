use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::{Cacheable, HasEtag};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum EndorsementStatus {
    Endorsed,
    Undecided,
    Abstained,
}

impl EndorsementStatus {
    pub fn display_for_tracked(&self) -> String {
        match self {
            EndorsementStatus::Endorsed => "👍🏻".to_string(),
            EndorsementStatus::Undecided => "".to_string(),
            _ => "🚫".to_string(),
        }
    }
}

impl Display for EndorsementStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndorsementStatus::Endorsed => write!(f, " "),
            EndorsementStatus::Undecided => write!(f, "🤔"),
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

    pub fn mod_id(&self) -> u32 {
        self.mod_id
    }

    pub fn get_url(&self) -> String {
        format!(
            "https://www.nexusmods.com/{}/mods/{}",
            self.domain_name, self.mod_id
        )
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
pub struct EndorsementList {
    pub mods: Vec<UserEndorsement>,
    pub etag: String,
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

    pub fn get(refresh: bool, db: &kv::Store, nexus: &mut NexusClient) -> Option<Box<Self>> {
        if refresh {
            super::refresh::<Self, ()>((), db, nexus)
        } else {
            super::find::<Self, ()>((), db, nexus)
        }
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
        )
    }
}

impl HasEtag for EndorsementList {
    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
    }
}

impl Cacheable<()> for EndorsementList {
    fn bucket_name() -> &'static str {
        "endorsements"
    }

    fn local(_key: (), db: &kv::Store) -> Option<Box<Self>> {
        let bucket = super::bucket::<Self, ()>(db).unwrap();
        let found = bucket.get("endorsements").ok()?;
        if let Some(modref_list) = found {
            return Some(Box::new(modref_list));
        }
        None
    }

    fn fetch(
        _key: (),
        nexus: &mut crate::nexus::NexusClient,
        etag: Option<String>,
    ) -> Option<Box<Self>> {
        nexus.endorsements(etag).map(Box::new)
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, ()>(db).unwrap();
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
