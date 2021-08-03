use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::Cacheable;

#[derive(Deserialize, Serialize, Debug, Clone)]
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

impl Cacheable<&str> for AuthenticatedUser {
    fn bucket_name() -> &'static str {
        "authed_users"
    }

    fn local(key: &str, db: &kv::Store) -> Option<Box<Self>> {
        let bucket = super::bucket::<Self, &str>(db).unwrap();
        let found = bucket.get(key).ok()?;
        found.map(Box::new)
    }

    fn fetch(_key: &str, nexus: &mut NexusClient, _etag: Option<String>) -> Option<Box<Self>> {
        if let Ok(user) = nexus.validate() {
            Some(Box::new(user))
        } else {
            None
        }
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, &str>(db).unwrap();
        if bucket.set("authed_user", self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
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

// it feels like if I figured out the kv crate traits I wouldn't have to do this.
impl kv::Value for AuthenticatedUser {
    fn to_raw_value(&self) -> Result<kv::Raw, kv::Error> {
        let x = serde_json::to_vec(&self)?;
        Ok(x.into())
    }

    fn from_raw_value(r: kv::Raw) -> Result<Self, kv::Error> {
        let x: Self = serde_json::from_slice(&r)?;
        Ok(x)
    }
}
