use std::collections::HashMap;
use std::fmt::Display;

use kv::Json;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::formatting::pluralize_mod;
use crate::nexus::NexusClient;
use crate::Cacheable;

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

    pub fn url(&self) -> String {
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

    pub fn listkey() -> &'static str {
        "endorsements"
    }
}

impl Display for EndorsementList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mapping = self.get_game_map();
        writeln!(
            f,
            "\n{} opinionated upon for {} games\n",
            pluralize_mod(self.mods.len()),
            mapping.len().blue()
        )
    }
}

impl Cacheable for EndorsementList {
    type K = &'static str;

    fn bucket_name() -> &'static str {
        "endorsements"
    }

    fn get(key: &&'static str, refresh: bool, nexus: &mut NexusClient) -> Option<Box<Self>> {
        super::get::<Self>(key, refresh, nexus)
    }

    fn fetch(
        _key: &&'static str,
        nexus: &mut NexusClient,
        etag: Option<String>,
    ) -> Option<Box<Self>> {
        nexus.endorsements(etag).map(Box::new)
    }

    fn key(&self) -> &'static str {
        EndorsementList::listkey()
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
