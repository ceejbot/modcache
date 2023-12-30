use std::time::Duration;
use std::u16;

use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;
use serde::Deserialize;

use crate::data::*;

static NEXUS_BASE: &str = "https://api.nexusmods.com";

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
            hourly_reset: Utc::now(), // TODO add 1 hour
            daily_limit: 2500,
            daily_remaining: 2500,
            daily_reset: Utc::now(), // TODO add 24 hours
        }
    }
}

// Now a wrapper for the client so nobody else has to think about rate limiting.
#[derive(Debug)]
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

    pub fn remaining_hour(&self) -> u16 {
        self.limits.hourly_remaining
    }

    pub fn remaining_day(&self) -> u16 {
        self.limits.daily_remaining
    }

    fn requests_allowed(&self) -> bool {
        if self.limits.hourly_remaining < 1 {
            log::error!(
                "Past hourly api call limit of {}! Wait until {}",
                self.limits.hourly_limit,
                self.limits.hourly_reset
            );
            false
        } else if self.limits.daily_remaining < 1 {
            log::error!(
                "Past daily api call limit of {}! Wait until {}",
                self.limits.daily_limit,
                self.limits.daily_reset
            );
            false
        } else {
            true
        }
    }

    /// Handle nexus rate-limiting headers, responding with the value of an etag
    /// header if one is found.
    // This would be a perfect use case for middleware.
    fn handle_headers(&mut self, response: &ureq::Response) -> Result<String, anyhow::Error> {
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

        let etag = match response.header("etag") {
            None => "".to_string(),
            Some(v) => v.to_string(),
        };

        Ok(etag)
    }

    /// Handle rate-limiting
    fn request_inner(&mut self, request: ureq::Request) -> anyhow::Result<ureq::Response> {
        if !self.requests_allowed() {
            anyhow::bail!("Rate-limited");
        }

        let response = match request.call() {
            Ok(v) => v,
            Err(ureq::Error::Status(code, response)) => {
                // max request rate is 30/sec, which tbh we can exceed instantly.
                if code == 429 {
                    log::warn!("The Nexus has rate-limited you!");
                } else {
                    log::error!("The Nexus responded with {}", code.red());
                    log::error!("{:?}", response.into_string());
                }
                anyhow::bail!("well this is another fine mess TODO better reporting");
            }
            Err(e) => {
                log::error!("Transport layer error: {:?}", e);
                anyhow::bail!(e);
            }
        };

        Ok(response)
    }

    /// Make a get request to the nexus, with optional etag.
    fn do_get(&mut self, uri: &str, etag: Option<String>) -> anyhow::Result<ureq::Response> {
        let mut request = self
            .agent
            .get(uri)
            .set("apikey", &self.apikey)
            .set("user-agent", "modcache: github.com/ceejbot/modcache");

        if let Some(t) = etag {
            request = request.set("if-none-match", &t);
        }

        self.request_inner(request)
    }

    fn conditional_get<T: for<'de> Deserialize<'de>>(
        &mut self,
        uri: &str,
        etag: Option<String>,
    ) -> Result<(Option<T>, String), anyhow::Error> {
        let response = self.do_get(uri, etag)?;
        let etag = self.handle_headers(&response)?;

        let status = response.status();
        if status == 304 {
            return Ok((None, etag));
        }

        match response.into_json::<T>() {
            Ok(v) => Ok((Some(v), etag)),
            Err(e) => {
                log::error!("couldn't deserialize nexus data: {:?}", e);
                log::error!("{}", uri);
                Err(anyhow::Error::new(e))
            }
        }
    }

    /// Unconditional nexus get; does not consider etags.
    // closer to reasonable...
    fn get<T: for<'de> Deserialize<'de>>(&mut self, uri: &str) -> Result<T, anyhow::Error> {
        let response = self.do_get(uri, None)?;
        let _etag = self.handle_headers(&response)?;

        let payload = response.into_json::<T>();
        match payload {
            Err(e) => {
                log::error!("couldn't deserialize nexus data: {:?}", e);
                log::error!("{}", uri);
                Err(anyhow::Error::new(e))
            }
            Ok(v) => Ok(v),
        }
    }

    // Shut up. I'm repeating myself to find patterns, dammit.
    /// Post data to the nexus, deserializing the response into the requested type.
    /// Handles rate-limiting headers.
    pub fn post<T: for<'de> Deserialize<'de>>(
        &mut self,
        uri: &str,
        body: &[(&str, &str)],
    ) -> Result<T, anyhow::Error> {
        let response = match self
            .agent
            .post(uri)
            .set("apikey", &self.apikey)
            .set("user-agent", "modcache: github.com/ceejbot/modcache")
            .send_form(body)
        {
            Ok(v) => v,
            Err(ureq::Error::Status(code, v)) => {
                if code == 429 {
                    log::warn!("The Nexus has rate-limited you!");
                } else {
                    log::error!("The Nexus responded with {}", code.red());
                    log::error!("{:?}", v);
                }
                v
            }
            Err(e) => {
                log::error!("Transport layer error: {:?}", e);
                anyhow::bail!(e);
            }
        };
        if let Err(e) = self.handle_headers(&response) {
            log::error!("problem parsing headers: {:?}", e)
        }
        log::debug!("post got status={}", response.status());
        let payload = response.into_json::<T>();
        match payload {
            Err(e) => {
                log::error!("problem deserializing: {:?}", e);
                Err(anyhow::Error::new(e))
            }
            Ok(v) => Ok(v),
        }
    }

    // repeat previous comment
    pub fn delete<T: for<'de> Deserialize<'de>>(
        &mut self,
        uri: &str,
        body: &[(&str, &str)],
    ) -> Result<T, anyhow::Error> {
        let response = match self
            .agent
            .delete(uri)
            .set("apikey", &self.apikey)
            .set("user-agent", "modcache: github.com/ceejbot/modcache")
            .send_form(body)
        {
            Ok(v) => v,
            Err(ureq::Error::Status(code, v)) => {
                if code == 429 {
                    log::warn!("The Nexus has rate-limited you!");
                } else {
                    log::error!("The Nexus responded with {}", code.red());
                    log::error!("{:?}", v);
                }
                v
            }
            Err(e) => {
                log::error!("Transport layer error: {:?}", e);
                anyhow::bail!(e);
            }
        };
        // We're calling this for the side effects, I'm afraid. This needs refactoring.
        if let Err(e) = self.handle_headers(&response) {
            log::error!("problem parsing headers: {:?}", e)
        }
        log::info!("del got status={}", response.status());
        let payload = response.into_json::<T>();
        match payload {
            Err(e) => {
                log::error!("problem deserializing: {:?}", e);
                Err(anyhow::Error::new(e))
            }
            Ok(v) => Ok(v),
        }
    }

    /// Validate your Nexus API token.
    pub fn validate(&mut self) -> anyhow::Result<AuthenticatedUser> {
        let uri = format!("{}/v1/users/validate.json", NEXUS_BASE);
        self.get::<AuthenticatedUser>(&uri)
    }

    /// Fetch info about a specific game by its Nexus `domain name`.
    pub fn gameinfo(&mut self, game: &str, etag: Option<String>) -> Option<GameMetadata> {
        let uri = format!("{}/v1/games/{}.json", NEXUS_BASE, game);
        if let Ok((Some(mut metadata), etag)) = self.conditional_get::<GameMetadata>(&uri, etag) {
            metadata.set_etag(&etag);
            return Some(metadata);
        }
        None
    }

    /// Fetch full information about a mod by game domain name & mod id number.
    /// These are the same two pieces of information in the url for a mod on the Nexus's site.
    pub fn mod_by_id(
        &mut self,
        game: &str,
        modid: u32,
        etag: Option<String>,
    ) -> Option<ModInfoFull> {
        let uri = format!("{}/v1/games/{}/mods/{}.json", NEXUS_BASE, game, modid);
        if let Ok((Some(mut modinfo), etag)) = self.conditional_get::<ModInfoFull>(&uri, etag) {
            modinfo.set_etag(&etag);
            return Some(modinfo);
        }
        None
    }

    pub fn changelogs(
        &mut self,
        game: &str,
        modid: u32,
        etag: Option<String>,
    ) -> Option<Changelogs> {
        let uri = format!(
            "{}/v1/games/{}/mods/{}/changelogs.json",
            NEXUS_BASE, game, modid
        );
        if let Ok((Some(mut changelogs), etag)) = self.conditional_get::<Changelogs>(&uri, etag) {
            changelogs.set_etag(&etag);
            return Some(changelogs);
        }
        None
    }

    pub fn files(&mut self, game: &str, modid: u32, etag: Option<String>) -> Option<Files> {
        let uri = format!("{}/v1/games/{}/mods/{}/files.json", NEXUS_BASE, game, modid);
        if let Ok((Some(mut files), etag)) = self.conditional_get::<Files>(&uri, etag) {
            files.set_etag(&etag);
            return Some(files);
        }
        None
    }

    /// Get detailed info about a file.
    pub fn mod_file_info(
        &mut self,
        game: &str,
        mod_id: u32,
        file: &str,
    ) -> anyhow::Result<FileInfo> {
        let uri = format!(
            "{}/v1/games/{}/{}/files/{}.json",
            NEXUS_BASE, game, mod_id, file
        );
        self.get::<FileInfo>(&uri)
    }

    /// Fetch the list of mods tracked for all games.
    pub fn tracked(&mut self, etag: Option<String>) -> Option<Tracked> {
        let uri = format!("{}/v1/user/tracked_mods.json", NEXUS_BASE);
        if let Ok((Some(mods), etag)) = self.conditional_get::<Vec<ModReference>>(&uri, etag) {
            return Some(Tracked { mods, etag });
        }
        None
    }

    /// Begin tracking a specific mod, identified by game domain name and id.
    pub fn track(&mut self, game: &str, mod_id: u32) -> anyhow::Result<TrackingResponse> {
        let uri = format!(
            "{}/v1/user/tracked_mods.json?domain_name={}",
            NEXUS_BASE, game
        );
        self.post(&uri, &[("mod_id", &format!("{}", mod_id))])
    }

    /// Stop tracking a specific mod, identified by game domain name and id.
    pub fn untrack(&mut self, game: &str, mod_id: u32) -> anyhow::Result<TrackingResponse> {
        let uri = format!(
            "{}/v1/user/tracked_mods.json?domain_name={}",
            NEXUS_BASE, game
        );
        self.delete(&uri, &[("mod_id", &format!("{}", mod_id))])
    }

    /// Get the list of all endorsement decisions made by the authed user.
    pub fn endorsements(&mut self, etag: Option<String>) -> Option<EndorsementList> {
        let uri = format!("{}/v1/user/endorsements.json", NEXUS_BASE);
        if let Ok((Some(mods), etag)) = self.conditional_get::<Vec<UserEndorsement>>(&uri, etag) {
            return Some(EndorsementList { mods, etag });
        }
        None
    }

    /// Endorse a mod.
    pub fn endorse(&mut self, game: &str, mod_id: u32) -> anyhow::Result<EndorseResponse> {
        let uri = format!(
            "{}/v1/games/{}/mods/{}/endorse.json",
            NEXUS_BASE, game, mod_id
        );
        self.post::<EndorseResponse>(&uri, &[("version", "*")])
    }

    /// Abstain from endorsing a mod.
    pub fn abstain(&mut self, game: &str, mod_id: u32) -> anyhow::Result<EndorseResponse> {
        let uri = format!(
            "{}/v1/games/{}/mods/{}/abstain.json",
            NEXUS_BASE, game, mod_id
        );
        self.post::<EndorseResponse>(&uri, &[("version", "*")])
    }

    /// Get a list of trending mods for a specific game. This list is capped at 10.
    pub fn trending(&mut self, game: &str) -> anyhow::Result<ModInfoList> {
        let uri = format!("{}/v1/games/{}/mods/trending.json", NEXUS_BASE, game);
        self.get::<ModInfoList>(&uri)
    }

    /// Get a list of the ten most-recently-added mods for a game. This list can include
    /// mods that have not yet been published, or contain incomplete information.
    pub fn latest_added(&mut self, game: &str) -> anyhow::Result<ModInfoList> {
        let uri = format!("{}/v1/games/{}/mods/latest_added.json", NEXUS_BASE, game);
        self.get::<ModInfoList>(&uri)
    }

    /// Get a list of the ten most-recently updated mods for a game.
    pub fn latest_updated(&mut self, game: &str) -> anyhow::Result<ModInfoList> {
        let uri = format!("{}/v1/games/{}/mods/latest_updated.json", NEXUS_BASE, game);
        self.get::<ModInfoList>(&uri)
    }
}
