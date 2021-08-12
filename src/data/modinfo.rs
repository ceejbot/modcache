// All structs and trait impls supporting the full mod info response from the Nexus.

use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use terminal_size::*;

use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::{Cacheable, EndorsementStatus};

#[derive(serde::Deserialize, Serialize, Debug, Clone)]
pub struct ModAuthor {
    member_group_id: u16,
    member_id: u32,
    name: String,
}

impl Display for ModAuthor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} <{}>", self.name.yellow(), self.member_id)
    }
}

impl Default for ModAuthor {
    fn default() -> Self {
        ModAuthor {
            member_group_id: 0,
            member_id: 0,
            name: "Alan Smithee".to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModEndorsement {
    pub(crate) endorse_status: EndorsementStatus,
    pub(crate) timestamp: Option<u64>,
    pub(crate) version: Option<String>,
}

impl Display for ModEndorsement {
    // just delegate to the status
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.endorse_status.fmt(f)
    }
}

impl Default for ModEndorsement {
    fn default() -> Self {
        Self {
            endorse_status: EndorsementStatus::Undecided,
            timestamp: None,
            version: None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ModStatus {
    Hidden,
    NotPublished,
    Published,
    Removed,
    UnderModeration,
    Wastebinned,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct ModInfoFull {
    // the next two fields fully identify a mod
    domain_name: String,
    mod_id: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    picture_url: Option<String>, // valid URL if present
    version: String, // no enforcement of semver

    author: String, // arbitrary text for credit
    uploaded_by: String,
    user: ModAuthor, // this points to a nexus user
    uploaded_users_profile_url: String,

    #[serde(default)]
    description: String, // long; bbcode-marked text

    created_time: String, // formatted time: 2021-02-18T17:05:56.000+00:00
    created_timestamp: u64,
    updated_time: String,
    updated_timestamp: u64,

    available: bool,
    status: ModStatus,
    allow_rating: bool,
    category_id: u16,
    contains_adult_content: bool,
    endorsement: Option<ModEndorsement>, // might be null
    endorsement_count: u32,
    game_id: u32,
    uid: u64, // unknown meaning
    etag: String,
}

impl ModInfoFull {
    pub fn key(key: (&str, u32)) -> String {
        format!("{}/{}", key.0, key.1)
    }

    pub fn by_prefix(prefix: &str, db: &kv::Store) -> Vec<Self> {
        let bucket = super::bucket::<Self, (&str, u32)>(db).unwrap();

        let mut result: Vec<Self> = Vec::new();
        for item in bucket.iter_prefix(prefix).flatten() {
            if let Ok(modinfo) = item.value() {
                result.push(modinfo);
            }
        }

        result
    }

    pub fn available(&self) -> bool {
        self.available
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn summary_cleaned(&self) -> String {
        self.summary.replace("<br />", "\n")
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn category_id(&self) -> u16 {
        self.category_id
    }

    pub fn mod_id(&self) -> u32 {
        self.mod_id
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn uploaded_by(&self) -> &str {
        &self.uploaded_by
    }

    pub fn url(&self) -> String {
        format!(
            "https://www.nexusmods.com/{}/mods/{}",
            self.domain_name, self.mod_id
        )
    }

    pub fn status(&self) -> ModStatus {
        self.status.clone()
    }

    pub fn display_name(&self) -> String {
        let n = if self.name.is_empty() {
            format!("id #{}", self.mod_id)
        } else {
            self.name()
        };

        match self.status {
            ModStatus::Hidden => {
                format!("{} {}", n.blue(), "HIDDEN".red())
            }
            ModStatus::NotPublished => {
                format!("{} {}", n.green(), "UNPUBLISHED".red())
            }
            ModStatus::Published => {
                format!("{}", n.green())
            }
            ModStatus::Removed => {
                format!("{} {}", n.blue(), "REMOVED".red())
            }
            ModStatus::UnderModeration => {
                format!("{} {}", n.blue(), "MODERATED".red())
            }
            ModStatus::Wastebinned => {
                format!("{} {}", n.blue(), "WASTEBINNED".red())
            }
        }
    }

    pub fn compact_info(&self) -> String {
        if let Some(endorse) = &self.endorsement {
            format!(
                "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\ <{}> {}",
                self.url(),
                self.display_name(),
                self.uploaded_by.cyan(),
                endorse.endorse_status.display_for_tracked()
            )
        } else {
            format!(
                "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\ <{}>",
                self.url(),
                self.display_name(),
                self.uploaded_by.cyan()
            )
        }
    }
}

impl Default for ModInfoFull {
    fn default() -> Self {
        ModInfoFull {
            domain_name: "unknown".to_string(),
            mod_id: 0,
            name: "".to_string(),
            summary: "".to_string(),
            picture_url: None,
            version: "".to_string(),
            author: "".to_string(),
            uploaded_by: "".to_string(),
            user: ModAuthor::default(),
            uploaded_users_profile_url: "".to_string(),
            description: "".to_string(),
            created_time: Utc::now().to_string(),
            created_timestamp: 0,
            updated_time: Utc::now().to_string(),
            updated_timestamp: 0,
            available: false,
            status: ModStatus::NotPublished,
            allow_rating: false,
            category_id: 0,
            contains_adult_content: false,
            endorsement: None,
            endorsement_count: 0,
            game_id: 0,
            uid: 0,
            etag: "".to_string(),
        }
    }
}

impl Display for ModInfoFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let width: usize = if let Some((Width(w), Height(_h))) = terminal_size() {
            w as usize - 2
        } else {
            72
        };
        let summary = textwrap::fill(&self.summary_cleaned(), width);
        let dt = match self.updated_time.parse::<DateTime<Utc>>() {
            Ok(v) => v.format("%c").to_string(),
            Err(_) => self.updated_time.clone(),
        };
        write!(
            f,
            "{}\nversion {} updated {}\n{}\n",
            self.compact_info(),
            self.version.red(),
            dt.blue(),
            &summary
        )
    }
}

impl Cacheable<(&str, u32)> for ModInfoFull {
    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
    }

    fn bucket_name() -> &'static str {
        "mods"
    }

    fn get(
        key: (&str, u32),
        refresh: bool,
        db: &kv::Store,
        nexus: &mut NexusClient,
    ) -> Option<Box<Self>> {
        super::get::<Self, (&str, u32)>(key, refresh, db, nexus)
    }

    fn local(key: (&str, u32), db: &kv::Store) -> Option<Box<Self>> {
        let compound = ModInfoFull::key(key);
        let bucket = super::bucket::<Self, (&str, u32)>(db).unwrap();
        let found = bucket.get(&*compound).ok()?;
        found.map(Box::new)
    }

    fn fetch(key: (&str, u32), nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>> {
        nexus.mod_by_id(key.0, key.1, etag).map(Box::new)
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, (&str, u32)>(db).unwrap();
        let compound = ModInfoFull::key((&self.domain_name, self.mod_id));
        if bucket.set(&*compound, self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }
}

// TODO write a macro for this
impl kv::Value for ModInfoFull {
    fn to_raw_value(&self) -> Result<kv::Raw, kv::Error> {
        let x = serde_json::to_vec(&self)?;
        Ok(x.into())
    }

    fn from_raw_value(r: kv::Raw) -> Result<Self, kv::Error> {
        let x: Self = serde_json::from_slice(&r)?;
        Ok(x)
    }
}
