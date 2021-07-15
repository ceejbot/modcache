use log::{error, info};
use owo_colors::OwoColorize;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
// use prettytable::Table;

use std::collections::HashMap;
use std::fmt::Display;

// Nexus mod data structs and trait implementations.

pub trait Cacheable {
    fn cache(&self, db: &Connection) -> anyhow::Result<bool>;
    fn from_row(row: &Row) -> Result<Box<Self>, rusqlite::Error>;
    // these can be abstracted
    fn lookup_by_int_id(id: u32, db: &Connection) -> Option<Box<Self>>;
    fn lookup_by_string_id(id: &str, db: &Connection) -> Option<Box<Self>>;
}


#[derive(Deserialize, Serialize, Debug)]
pub struct AuthenticatedUser {
    email: String,
    is_premium: bool,
    is_supporter: bool,
    name: String,
    profile_url: String,
    user_id: u32,
    #[serde(flatten)]
    ignored: Option<HashMap<String, serde_json::Value>>,
}

impl Default for AuthenticatedUser {
    fn default() -> Self {
        AuthenticatedUser {
            name: "example".to_string(),
            user_id: 1,
            email: "foo@example.com".to_string(),
            is_premium: false,
            is_supporter: false,
            profile_url: "".to_string(),
            ignored: None,
        }
    }
}

impl Cacheable for AuthenticatedUser {
    fn cache(&self, db: &Connection) -> anyhow::Result<bool> {
        let count = db.execute(
            r#"
            INSERT INTO authn_user
                (user_id, email, is_premium, is_supporter, name, profile_url)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT(user_id) DO NOTHING
            "#,
            params![
                self.user_id,
                self.email,
                self.is_premium,
                self.is_supporter,
                self.name,
                self.profile_url
            ],
        )?;
        Ok(count == 1)
    }

    fn from_row(row: &Row) -> Result<Box<AuthenticatedUser>, rusqlite::Error> {
        let user = AuthenticatedUser {
            user_id: row.get(0)?,
            email: row.get(1)?,
            is_premium: row.get(2)?,
            is_supporter: row.get(3)?,
            name: row.get(4)?,
            profile_url: row.get(5)?,
            ..Default::default()
        };
        Ok(Box::new(user))
    }

    fn lookup_by_string_id(id: &str, db: &Connection) -> Option<Box<AuthenticatedUser>> {
        // TODO handle specific errors
        match db.query_row(
            "SELECT * FROM authn_user WHERE email=$1",
            params![id],
            |row| AuthenticatedUser::from_row(row),
        ) {
            Err(e) => {
                error!("db query error! {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }

    fn lookup_by_int_id(id: u32, db: &Connection) -> Option<Box<AuthenticatedUser>> {
        // TODO handle specific errors
        match db.query_row(
            "SELECT * FROM authn_user WHERE user_id=$1",
            params![id],
            |row| AuthenticatedUser::from_row(row),
        ) {
            Err(e) => {
                error!("db query error! {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }
}

impl Display for AuthenticatedUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "    {} <{}>\n    https://www.nexusmods.com/users/{}\n    {}>",
            self.name.bold(),
            self.email.yellow(),
            self.user_id,
            self.profile_url
        )
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ModCategory {
    category_id: u16,
    name: String,
    // TODO custom deserialization
    // this is either `false` for the top-level game category or an unsigned int that
    // points to that top-level category_id  for the game
    parent_category: serde_json::Value,
}

impl Display for ModCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}: {})", self.name.yellow(), self.category_id)
    }
}

#[derive(Deserialize, Serialize, Debug)]
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
}

impl Cacheable for GameMetadata {
    fn cache(&self, db: &Connection) -> anyhow::Result<bool> {
        // TODO also store categories
        let count = db.execute(r#"INSERT INTO games
                (id, domain_name, name, approved_date, authors, downloads, file_count, file_endorsements, file_views, forum_url, genre, mods, nexusmods_url)
                VALUES (?1, ?2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
            params![self.id, self.domain_name, self.name, self.approved_date, self.authors,
                self.downloads, self.file_count, self.file_endorsements, self.file_views,
                self.forum_url, self.genre, self.mods, self.nexusmods_url],
        )?;

        Ok(count > 0)
    }

    fn from_row(row: &Row) -> Result<Box<GameMetadata>, rusqlite::Error> {
        let game = GameMetadata {
            id: row.get(0)?,
            domain_name: row.get(1)?,
            name: row.get(2)?,
            approved_date: row.get(3)?,
            authors: row.get(4)?,
            downloads: row.get(5)?,
            file_count: row.get(6)?,
            file_endorsements: row.get(7)?,
            file_views: row.get(8)?,
            forum_url: row.get(9)?,
            genre: row.get(10)?,
            mods: row.get(11)?,
            nexusmods_url: row.get(12)?,
            categories: Vec::new(),
        };
        Ok(Box::new(game))
    }

    fn lookup_by_string_id(id: &str, db: &Connection) -> Option<Box<GameMetadata>> {
        // Note opportunity for abstraction. Also could chain `.optional()`
        // TODO fill in categories
        match db.query_row(
            "SELECT * FROM games WHERE domain_name=$1",
            params![id],
            |row| GameMetadata::from_row(row),
        ) {
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                info!("cache miss for {}", id);
                None
            }
            Err(e) => {
                error!("db query error! {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }

    fn lookup_by_int_id(id: u32, db: &Connection) -> Option<Box<GameMetadata>> {
        // TODO fill in categories
        match db.query_row("SELECT * FROM games WHERE id=$1", params![id], |row| {
            GameMetadata::from_row(row)
        }) {
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                info!("cache miss for {}", id);
                None
            }
            Err(e) => {
                error!("db query error! {:?}", e);
                None
            }
            Ok(v) => Some(v),
        }
    }
}

// write!(f, "    {} <{}\n    https://www.nexusmods.com/{}/mods/categories/{}\n    {}>",

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

#[derive(Serialize, Deserialize, Debug)]
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
        write!(f, "{} {}/{}", self.status, self.domain_name, self.mod_id)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct EndorsementList {
    pub mods: Vec<UserEndorsement>,
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

#[derive(Deserialize, Serialize, Debug)]
pub enum ModStatus {
    NotPublished,
    Published,
    Hidden,
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

    pub(crate) available: bool,
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct ModInfoList {
    pub mods: Vec<ModInfoFull>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModReference {
    pub domain_name: String,
    pub mod_id: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct ModReferenceList {
    pub mods: Vec<ModReference>,
}
