// All structs and trait impls supporting the full mod info response from the Nexus.

use chrono::Utc;
use log::{error, info};
use owo_colors::OwoColorize;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::{Cacheable, Key};

#[derive(Deserialize, Serialize, Debug)]
pub struct ModAuthor {
    pub(crate) member_group_id: u16,
    pub(crate) member_id: u32,
    pub(crate) name: String,
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

#[derive(Deserialize, Serialize, Debug)]
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

#[derive(Deserialize, Serialize, Debug)]
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
#[serde(rename_all = "lowercase")]
pub enum ModStatus {
    #[serde()]
    NotPublished,
    Published,
    Hidden,
}

impl Into<String> for ModStatus {
    fn into(self) -> String {
        match self {
            ModStatus::Hidden => "hidden".to_string(),
            ModStatus::NotPublished => "notpublished".to_string(),
            ModStatus::Published => "published".to_string(),
        }
    }
}

impl From<String> for ModStatus {
    fn from(s: String) -> Self {
        match s.as_ref() {
            "hidden" => ModStatus::Hidden,
            "notpublished" => ModStatus::NotPublished,
            "published" => ModStatus::Published,
            _ => ModStatus::NotPublished,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
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

impl Cacheable for ModInfoFull {
    fn fetch(key: Key, db: &Connection, nexus: &mut NexusClient) -> Option<Box<Self>> {
        let hit = Self::lookup(key.clone(), db);
        if hit.is_some() {
            info!("cache hit for {:?}", key);
            return hit;
        }

        let (game, mod_id) = match key {
            Key::NameIdPair { name, id } => (name, id),
            _ => {
                return None;
            }
        };
        match nexus.mod_by_id(&game, mod_id) {
            Err(_) => None,
            Ok(modinfo) => {
                println!("got something back from the nexus");
                if modinfo.cache(db).is_ok() {
                    info!("stored mod {} # {}", game.yellow(), mod_id.blue());
                }
                Some(Box::new(modinfo))
            }
        }
    }

    fn cache(&self, db: &Connection) -> anyhow::Result<usize> {
        // TODO all sub-data via joins or whatever.
        // We are not writing an ORM, people. We're doing it the old-fashioned way.
        // This parameter list makes me doubt my life choices in not using an ORM.
        let status = serde_json::to_string(&self.status)?;
        let count = db.execute(
            r#"INSERT INTO mods (
                    domain_name, mod_id, uid, game_id,
                    name, version, category_id, summary, description,
                    picture_url, available, status, allow_rating,
                    author, uploaded_by, uploaded_users_profile_url,
                    endorsement_count,
                    nexus_created, nexus_updated
                )
                VALUES (?1, ?2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
                ON CONFLICT(domain_name, mod_id) DO NOTHING"#,
            params![self.domain_name, self.mod_id, self.uid, self.game_id,
                self.name, self.version, self.category_id, self.summary, self.description,
                self.picture_url, self.available, status, self.allow_rating,
                self.author, self.uploaded_by, self.uploaded_users_profile_url,
                self.endorsement_count,
                self.created_time, self.updated_time
            ],
        )?;

        // TODO: user field (ModAuthor struct)
        // TODO: endorsement status (ModEndorsement)
        Ok(count)
    }

    fn lookup(key: Key, db: &Connection) -> Option<Box<Self>> {
        let (game, mod_id) = match key {
            Key::NameIdPair { name, id } => (name, id),
            _ => {
                return None;
            }
        };

        let modinfo = match db.query_row(
            r#"SELECT * FROM mods WHERE domain_name=$1 AND mod_id=$2"#,
            params![game, mod_id],
            |row| ModInfoFull::from_row(row),
        ) {
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                info!("cache miss");
                return None;
            }
            Err(e) => {
                error!("sqlite error: {:?}", e);
                return None;
            }
            Ok(v) => v,
        };

        // TODO there's work here

        Some(modinfo)
    }

    fn from_row(row: &Row) -> Result<Box<Self>, rusqlite::Error> {
        // A little hacky.
        let status_str: String = row.get("status")?;
        let status = match serde_json::from_str(&status_str) {
            Ok(v) => v,
            Err(_) => ModStatus::NotPublished
        };

        let modinfo = ModInfoFull {
            domain_name: row.get("domain_name")?,
            mod_id: row.get("mod_id")?,
            name: row.get("name")?,
            version: row.get("version")?,
            category_id: row.get("category_id")?,
            summary: row.get("summary")?,
            description: row.get("description")?,
            picture_url: row.get("picture_url")?,
            status,
            available: row.get("available")?,
            allow_rating: row.get("allow_rating")?,
            contains_adult_content: row.get("contains_adult_content")?,
            author: row.get("author")?,
            uploaded_by: row.get("uploaded_by")?,
            uploaded_users_profile_url: row.get("uploaded_users_profile_url")?,
            endorsement_count: row.get("endorsement_count")?,
            created_time: row.get("nexus_created")?,
            updated_time: row.get("nexus_updated")?,
            ..Default::default()
            // todo user_id INT
            // ModEndorsement field
        };
        Ok(Box::new(modinfo))
    }
}
