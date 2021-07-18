use chrono::{DateTime, Utc};
use log::{error, info, warn};
use owo_colors::OwoColorize;
use serde::Deserialize;

use std::time::Duration;
use std::u16;

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
            error!(
                "Past hourly api call limit of {}! Wait until {}",
                self.limits.hourly_limit, self.limits.hourly_reset
            );
            false
        } else if self.limits.daily_remaining < 1 {
            error!(
                "Past daily api call limit of {}! Wait until {}",
                self.limits.daily_limit, self.limits.daily_reset
            );
            false
        } else {
            true
        }
    }

    // This would be a perfect use case for middleware.
    fn handle_headers(&mut self, response: &ureq::Response) -> Result<(), anyhow::Error> {
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

        Ok(())
    }

    fn make_get_request<T: for<'de> Deserialize<'de>>(
        &mut self,
        uri: &str,
    ) -> Result<T, anyhow::Error> {
        if !self.requests_allowed() {
            anyhow::bail!("Rate-limited");
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

        // TODO snag the etag header too
        if let Err(e) = self.handle_headers(&response) {
            error!("problem parsing headers: {:?}", e)
        }

        let payload = response.into_json::<T>();
        match payload {
            Err(e) => {
                error!("problem deserializing: {:?}", e);
                Err(anyhow::Error::new(e))
            }
            Ok(v) => Ok(v),
        }
    }

    // Shut up. I'm repeating myself to find patterns, dammit.
    // Here I assume I'm never going to care about what the Nexus responds with,
    // other than the status code.
    pub fn make_post_request(
        &mut self,
        uri: &str,
        body: &[(&str, &str)],
    ) -> Result<serde_json::Value, anyhow::Error> {
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
                    warn!("The Nexus has rate-limited you!");
                } else {
                    error!("The Nexus responded with {}", code.red());
                    error!("{:?}", v);
                }
                v
            }
            Err(e) => {
                error!("Transport layer error: {:?}", e);
                anyhow::bail!(e);
            }
        };
        if let Err(e) = self.handle_headers(&response) {
            error!("problem parsing headers: {:?}", e)
        }
        info!("post got status={}", response.status());
        let payload = response.into_json::<serde_json::Value>();
        match payload {
            Err(e) => {
                error!("problem deserializing: {:?}", e);
                Err(anyhow::Error::new(e))
            }
            Ok(v) => Ok(v),
        }
    }

    // repeat previous comment
    pub fn make_del_request(
        &mut self,
        uri: &str,
        body: &[(&str, &str)],
    ) -> Result<serde_json::Value, anyhow::Error> {
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
                    warn!("The Nexus has rate-limited you!");
                } else {
                    error!("The Nexus responded with {}", code.red());
                    error!("{:?}", v);
                }
                v
            }
            Err(e) => {
                error!("Transport layer error: {:?}", e);
                anyhow::bail!(e);
            }
        };
        if let Err(e) = self.handle_headers(&response) {
            error!("problem parsing headers: {:?}", e)
        }
        info!("del got status={}", response.status());
        let payload = response.into_json::<serde_json::Value>();
        match payload {
            Err(e) => {
                error!("problem deserializing: {:?}", e);
                Err(anyhow::Error::new(e))
            }
            Ok(v) => Ok(v),
        }
    }

    // Boy, this sure looks like predictable code.

    pub fn gameinfo(&mut self, game: &str) -> anyhow::Result<GameMetadata> {
        let uri = format!("{}/v1/games/{}.json", NEXUS_BASE, game);
        self.make_get_request::<GameMetadata>(&uri)
    }

    pub fn mod_by_id(&mut self, game: &str, modid: u32) -> anyhow::Result<ModInfoFull> {
        let uri = format!("{}/v1/games/{}/mods/{}.json", NEXUS_BASE, game, modid);
        self.make_get_request::<ModInfoFull>(&uri)
    }

    pub fn validate(&mut self) -> anyhow::Result<AuthenticatedUser> {
        let uri = format!("{}/v1/users/validate.json", NEXUS_BASE);
        self.make_get_request::<AuthenticatedUser>(&uri)
    }

    pub fn tracked(&mut self) -> anyhow::Result<Tracked> {
        let uri = format!("{}/v1/user/tracked_mods.json", NEXUS_BASE);
        self.make_get_request::<Tracked>(&uri)
    }

    pub fn track(&mut self, game: &str, mod_id: u32) -> anyhow::Result<serde_json::Value> {
        let uri = format!(
            "{}/v1/user/tracked_mods.json?domain_name={}",
            NEXUS_BASE, game
        );
        self.make_post_request(&uri, &[("mod_id", &format!("{}", mod_id))])
    }

    pub fn untrack(&mut self, game: &str, mod_id: u32) -> anyhow::Result<serde_json::Value> {
        let uri = format!(
            "{}/v1/user/tracked_mods.json?domain_name={}",
            NEXUS_BASE, game
        );
        self.make_del_request(&uri, &[("mod_id", &format!("{}", mod_id))])
    }

    pub fn endorsements(&mut self) -> anyhow::Result<EndorsementList> {
        let uri = format!("{}/v1/user/endorsements.json", NEXUS_BASE);
        self.make_get_request::<EndorsementList>(&uri)
    }

    pub fn trending(&mut self, game: &str) -> anyhow::Result<ModInfoList> {
        let uri = format!("{}/v1/games/{}/mods/trending.json", NEXUS_BASE, game);
        self.make_get_request::<ModInfoList>(&uri)
    }

    pub fn latest_added(&mut self, game: &str) -> anyhow::Result<ModInfoList> {
        let uri = format!("{}/v1/games/{}/mods/latest_added.json", NEXUS_BASE, game);
        self.make_get_request::<ModInfoList>(&uri)
    }

    pub fn latest_updated(&mut self, game: &str) -> anyhow::Result<ModInfoList> {
        let uri = format!("{}/v1/games/{}/mods/latest_updated.json", NEXUS_BASE, game);
        self.make_get_request::<ModInfoList>(&uri)
    }

    // TODO
    // /v1/games/:game/mods/:mod/changelogs.json
    // /v1/games/:game/mods/:mod/files.json
    // /v1/games/:game/mods/:mod/files/:file.json
}
