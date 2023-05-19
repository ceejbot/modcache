use std::collections::HashMap;
use std::fmt::Display;

use itertools::Itertools;
use kv::Json;
use num_format::{Locale, ToFormattedString};
use owo_colors::OwoColorize;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use unicase::UniCase;

use super::{Cacheable, ModInfoFull, ModStatus};
use crate::nexus::NexusClient;

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
    pub fn name(&self) -> &str {
        &self.name
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

    pub fn categories(&self) -> &Vec<ModCategory> {
        &self.categories
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

    /// Display full information about a game, its categories, and any mods in cache for it.
    pub fn emit_fancy(&self, _db: &kv::Store) {
        println!("{}", self.name().yellow().bold());
        println!(
            "{} mods by {} authors",
            self.mods.to_formatted_string(&Locale::en).bold(),
            self.authors.bold()
        );
        println!(
            "{} downloads",
            self.downloads.to_formatted_string(&Locale::en).bold()
        );
        println!();

        let cats: Vec<String> = self
            .categories()
            .iter()
            .sorted_by(|left, right| left.name().cmp(right.name()))
            .map(|cat| format!("    {}", cat.name().purple()))
            .collect();
        crate::formatting::print_in_grid(cats, 2);
    }

    /// Get all mods cached for this game.
    pub fn mods(&self, db: &kv::Store) -> Vec<ModInfoFull> {
        let prefix = format!("{}/", &self.domain_name);
        ModInfoFull::by_prefix(&prefix, db)
            .into_iter()
            .sorted_by(|left, right| UniCase::new(left.name()).cmp(&UniCase::new(right.name())))
            .collect()
    }

    /// Get all mods for this game with names that match the given filter pattern.
    /// Case-insensitive, but otherwise a very naive match.
    pub fn mods_name_match(&self, filter: &str, db: &kv::Store) -> Vec<ModInfoFull> {
        let prefix = format!("{}/", &self.domain_name);
        let candidates = ModInfoFull::by_prefix(&prefix, db);
        let patt = RegexBuilder::new(filter)
            .case_insensitive(true)
            .build()
            .unwrap();
        candidates
            .into_iter()
            .filter(|modinfo| patt.is_match(&modinfo.name()))
            .collect()
    }

    /// Get all mods for this game with names or summaries that match the given filter pattern.
    /// Case-insensitive, but otherwise a very naive match.
    // Note repetition with previous function. Searching needs some abstractions.
    pub fn mods_match_text(&self, filter: &str, db: &kv::Store) -> Vec<ModInfoFull> {
        let prefix = format!("{}/", &self.domain_name);
        let candidates = ModInfoFull::by_prefix(&prefix, db);
        let patt = RegexBuilder::new(filter)
            .case_insensitive(true)
            .build()
            .unwrap();
        candidates
            .into_iter()
            .filter(|modinfo| {
                patt.is_match(&modinfo.name())
                    || patt.is_match(modinfo.summary())
                    || patt.is_match(modinfo.uploaded_by())
                    || patt.is_match(modinfo.author())
            })
            .sorted_by(|left, right| UniCase::new(left.name()).cmp(&UniCase::new(right.name())))
            .collect()
    }

    // I learned a surprising thing about rust when I tried to make a single function
    // to which I pass the enum variant I want to match against.
    pub fn mods_hidden(&self, db: &kv::Store) -> Vec<ModInfoFull> {
        let prefix = format!("{}/", &self.domain_name);
        let candidates = ModInfoFull::by_prefix(&prefix, db);
        candidates
            .into_iter()
            .filter(|modinfo| matches!(modinfo.status(), ModStatus::Hidden))
            .sorted_by(|left, right| left.mod_id().cmp(&right.mod_id()))
            .collect()
    }

    pub fn mods_removed(&self, db: &kv::Store) -> Vec<ModInfoFull> {
        let prefix = format!("{}/", &self.domain_name);
        let candidates = ModInfoFull::by_prefix(&prefix, db);
        candidates
            .into_iter()
            .filter(|modinfo| matches!(modinfo.status(), ModStatus::Removed))
            .sorted_by(|left, right| left.mod_id().cmp(&right.mod_id()))
            .collect()
    }

    pub fn mods_wastebinned(&self, db: &kv::Store) -> Vec<ModInfoFull> {
        let prefix = format!("{}/", &self.domain_name);
        let candidates = ModInfoFull::by_prefix(&prefix, db);
        candidates
            .into_iter()
            .filter(|modinfo| matches!(modinfo.status(), ModStatus::Wastebinned))
            .sorted_by(|left, right| left.mod_id().cmp(&right.mod_id()))
            .collect()
    }
}

impl Cacheable for GameMetadata {
    type K = String;

    fn bucket_name() -> &'static str {
        "games"
    }

    fn get(
        key: &String,
        refresh: bool,
        store: &kv::Store,
        nexus: &mut NexusClient,
    ) -> Option<Box<Self>> {
        super::get::<Self>(key, refresh, store, nexus)
    }

    fn fetch(key: &String, nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>> {
        nexus.gameinfo(key, etag).map(Box::new)
    }

    fn key(&self) -> String {
        self.domain_name.clone()
    }

    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self>(db).unwrap();
        bucket.set(&&*self.domain_name, &Json(self.clone()))?;
        bucket.flush()?;
        Ok(1)
    }

    fn update(&self, other: &Self) -> Self {
        other.clone()
    }
}
