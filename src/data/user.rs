use kv::Json;
use log::{info, warn};
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::Cacheable;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct AuthenticatedUser {
    email: String,
    is_premium: bool,
    is_supporter: bool,
    name: String,
    profile_url: String,
    user_id: u32,
    etag: String,
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
            etag: "".to_string(),
        }
    }
}

impl Cacheable<&str> for AuthenticatedUser {
    fn bucket_name() -> &'static str {
        "authed_users"
    }

    fn get(
        _key: &&str,
        _refresh: bool,
        db: &kv::Store,
        nexus: &mut NexusClient,
    ) -> Option<Box<Self>> {
        // We do not ever rely on cache for this.
        if let Ok(user) = nexus.validate() {
            match user.store(db) {
                Ok(_) => info!("stored authed user"),
                Err(e) => warn!("failed to store authed user! {:?}", e),
            }
            Some(Box::new(user))
        } else {
            None
        }
    }

    fn fetch(_key: &&str, nexus: &mut NexusClient, _etag: Option<String>) -> Option<Box<Self>> {
        if let Ok(user) = nexus.validate() {
            Some(Box::new(user))
        } else {
            None
        }
    }

    fn key(&self) -> &'static str {
        "authed_user"
    }

    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, &str>(db).unwrap();
        if bucket.set(self.key(), Json(self.clone())).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn update(&self, other: &Self) -> Self {
        other.clone()
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
