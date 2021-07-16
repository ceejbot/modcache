// All structs and trait impls supporting the full mod info response from the Nexus.

use chrono::Utc;
use log::{error, info};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::{Cacheable, Key};

#[derive(Deserialize, Serialize, Debug, Clone)]
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
    #[serde()]
    NotPublished,
    Published,
    Hidden,
    Removed,
}

impl From<String> for ModStatus {
    fn from(s: String) -> Self {
        match s.as_ref() {
            "hidden" => ModStatus::Hidden,
            "not_published" => ModStatus::NotPublished,
            "published" => ModStatus::Published,
            "removed" => ModStatus::Removed,
            _ => ModStatus::Removed,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModInfoFull {
    // the next two fields fully identify a mod
    domain_name: String,
    mod_id: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    picture_url: String, // valid URL if present
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
}

impl ModInfoFull {
    pub fn available(&self) -> bool {
        self.available
    }
}

impl Default for ModInfoFull {
    fn default() -> Self {
        ModInfoFull {
            domain_name: "unknown".to_string(),
            mod_id: 0,
            name: "".to_string(),
            summary: "".to_string(),
            picture_url: "".to_string(),
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
        }
    }
}

impl Display for ModInfoFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n{} @ {}\nuploaded by {}\n\n{}\n",
            self.name.yellow(),
            self.version,
            self.updated_time,
            self.uploaded_by,
            self.summary
        )
    }
}

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

impl Cacheable for ModInfoFull {
    fn bucket(db: &kv::Store) -> Option<kv::Bucket<'static, &'static str, Self>> {
        match db.bucket::<&str, Self>(Some("mods")) {
            Err(e) => {
                error!("Can't open bucket for mod info! {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }

    fn local(key: Key, db: &kv::Store) -> Option<Box<Self>> {
        let (game, mod_id) = match key {
            Key::NameIdPair { name, id } => (name, id),
            _ => {
                return None;
            }
        };
        let compound = format!("{}/{}", game, mod_id);
        let bucket = ModInfoFull::bucket(db).unwrap();
        let found = bucket.get(&*compound).ok()?;
        if let Some(modinfo) = found {
            info!("cache hit for {}", compound);
            Some(Box::new(modinfo))
        } else {
            None
        }
    }

    fn fetch(key: Key, nexus: &mut NexusClient) -> Option<Box<Self>> {
        let (game, mod_id) = match key {
            Key::NameIdPair { name, id } => (name, id),
            _ => {
                return None;
            }
        };

        if let Ok(modinfo) = nexus.mod_by_id(&game, mod_id) {
            Some(Box::new(modinfo))
        } else {
            None
        }
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = ModInfoFull::bucket(db).unwrap();
        let compound = format!("{}/{}", self.domain_name, self.mod_id);
        if bucket.set(&*compound, self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }
}
