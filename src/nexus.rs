use chrono::{DateTime, Utc};
use log::{error, info, warn};
use owo_colors::OwoColorize;
// use prettytable::Table;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;
use std::time::Duration;
use std::u16;

static NEXUS_BASE: &str = "https://api.nexusmods.com";

pub trait Cacheable {
    fn cache(&self, db: &Connection) -> anyhow::Result<()>;
    fn from_row(row: &Row) -> Result<Box<Self>, rusqlite::Error>;
    // these can be abstracted
    fn lookup_by_int_id(id: u32, db: &Connection) -> Option<Box<Self>>;
    fn lookup_by_string_id(id: &str, db: &Connection) -> Option<Box<Self>>;
}

// Nexus client wrapper and associated response structs;

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
    fn cache(&self, db: &Connection) -> anyhow::Result<()> {
        let row_count = db.execute(
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
        println!("{:?}", row_count);
        Ok(())
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
    fn cache(&self, db: &Connection) -> anyhow::Result<()> {
        // TODO upsert?
        db.execute(r#"INSERT INTO games
                (id, domain_name, name, approved_date, authors, downloads, file_count, file_endorsements, file_views, forum_url, genre, mods, nexusmods_url)
                VALUES (?1, ?2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
            params![self.id, self.domain_name, self.name, self.approved_date, self.authors, self.downloads, self.file_count, self.file_endorsements, self.file_views, self.forum_url, self.genre, self.mods, self.nexusmods_url],
        )?;

        Ok(())
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
        // Note opportunity for abstraction.
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
        // Note opportunity for abstraction.
        match db.query_row("SELECT * FROM games WHERE id=$1", params![id], |row| {
            GameMetadata::from_row(row)
        }) {
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

// rate limit data, mod private

#[derive(Debug)]
struct RateLimits {
    hourly_limit: u16,
    hourly_remaining: u16,
    hourly_reset: DateTime<Utc>,
    daily_limit: u16,
    daily_remaining: u16,
    daily_reset: DateTime<Utc>,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            hourly_limit: 100,
            hourly_remaining: 100,
            hourly_reset: Utc::now(), // add 1 hour
            daily_limit: 2500,
            daily_remaining: 2500,
            daily_reset: Utc::now(), // add 24 hours
        }
    }
}

// Now a wrapper for the client so nobody else has to think about rate limiting.
pub struct NexusClient {
    pub agent: ureq::Agent,
    apikey: String,
    limits: RateLimits,
}

impl NexusClient {
    pub fn new(apikey: String) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(50))
            .timeout_write(Duration::from_secs(5))
            .build();

        NexusClient {
            agent,
            apikey,
            limits: RateLimits::default(),
        }
    }

    fn make_request<T: for<'de> Deserialize<'de>>(
        &mut self,
        uri: &str,
    ) -> Result<T, anyhow::Error> {
        if self.limits.hourly_remaining < 1 {
            anyhow::bail!(
                "Past hourly api call limit of {}! Wait until {}",
                self.limits.hourly_limit,
                self.limits.hourly_reset
            );
        }
        if self.limits.daily_remaining < 1 {
            anyhow::bail!(
                "Past daily api call limit of {}! Wait until {}",
                self.limits.daily_limit,
                self.limits.daily_reset
            );
        }

        let response = match self
            .agent
            .get(uri)
            .set("apikey", &self.apikey)
            .set("user-agent", "modcache: github.com/ceejbot/modcache")
            .call()
        {
            Ok(v) => v,
            Err(ureq::Error::Status(code, response)) => {
                if code == 429 {
                    warn!("The Nexus has rate-limited you!");
                } else {
                    error!("The Nexus responded with {}", code.red());
                    error!("{:?}", response.into_string());
                }
                anyhow::bail!("well this is another fine mess TODO better reporting");
            }
            Err(e) => {
                error!("Transport layer error: {:?}", e);
                anyhow::bail!(e);
            }
        };

        // handle nexus rate limiting; 429 response code indicates you've hit it
        // max request rate is 30/sec
        // We bail on parse failures because well, if this happens we've misunderstood the
        // contract the Nexus is upholding with us.
        if let Some(v) = response.header("x-rl-hourly-limit") {
            self.limits.hourly_limit = v.trim().parse()?;
        }
        if let Some(v) = response.header("x-rl-hourly-remaining") {
            self.limits.hourly_remaining = v.trim().parse()?;
        }
        if let Some(v) = response.header("x-rl-hourly-reset") {
            self.limits.hourly_reset = v.parse::<DateTime<Utc>>()?;
        }
        if let Some(v) = response.header("x-rl-daily-limit") {
            self.limits.daily_limit = v.trim().parse()?;
        }
        if let Some(v) = response.header("x-rl-daily-remaining") {
            self.limits.daily_remaining = v.trim().parse()?;
        }
        if let Some(v) = response.header("x-rl-daily-reset") {
            self.limits.daily_reset = v.parse::<DateTime<Utc>>()?;
        }

        // TODO snag the etag header too

        let payload = response.into_json::<T>();
        match payload {
            Err(e) => Err(anyhow::Error::new(e)),
            Ok(v) => Ok(v),
        }
    }

    pub fn gameinfo(&mut self, game: &str) -> anyhow::Result<GameMetadata> {
        let uri = format!("{}//v1/games/{}.json", NEXUS_BASE, game);
        self.make_request::<GameMetadata>(&uri)
    }

    pub fn mod_by_id(&mut self, game: &str, modid: u32) -> anyhow::Result<ModInfoFull> {
        let uri = format!("{}//v1/games/{}/mods/{}.json", NEXUS_BASE, game, modid);
        self.make_request::<ModInfoFull>(&uri)
    }

    pub fn validate(&mut self) -> anyhow::Result<AuthenticatedUser> {
        let uri = format!("{}//v1/users/validate.json", NEXUS_BASE);
        self.make_request::<AuthenticatedUser>(&uri)
    }

    pub fn tracked(&mut self) -> anyhow::Result<ModReferenceList> {
        let uri = format!("{}//v1/user/tracked_mods.json", NEXUS_BASE);
        self.make_request::<ModReferenceList>(&uri)
    }

    pub fn endorsements(&mut self) -> anyhow::Result<EndorsementList> {
        let uri = format!("{}//v1/user/endorsements.json", NEXUS_BASE);
        self.make_request::<EndorsementList>(&uri)
    }

    pub fn trending(&mut self, game: &str) -> anyhow::Result<ModInfoList> {
        let uri = format!("{}//v1/games/{}/mods/trending.json", NEXUS_BASE, game);
        self.make_request::<ModInfoList>(&uri)
    }

    pub fn latest_added(&mut self, game: &str) -> anyhow::Result<ModInfoList> {
        let uri = format!("{}//v1/games/{}/mods/latest_added.json", NEXUS_BASE, game);
        self.make_request::<ModInfoList>(&uri)
    }

    pub fn latest_updated(&mut self, game: &str) -> anyhow::Result<ModInfoList> {
        let uri = format!("{}//v1/games/{}/mods/latest_updated.json", NEXUS_BASE, game);
        self.make_request::<ModInfoList>(&uri)
    }

    // TODO
    // /v1/games/:game/mods/:mod/changelogs.json
    // /v1/games/:game/mods/:mod/files.json
    // /v1/games/:game/mods/:mod/files/:file.json
}
