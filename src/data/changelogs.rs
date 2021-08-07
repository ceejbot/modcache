// All structs and trait impls supporting the full mod info response from the Nexus.

use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::Cacheable;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct Changelogs {
    domain_name: String,
    mod_id: u32,
    etag: String,
    #[serde(flatten)]
    versions: HashMap<String, Vec<String>>,
}

impl Changelogs {
    pub fn key(key: (&str, u32)) -> String {
        format!("{}/{}", key.0, key.1)
    }

    pub fn versions(&self) -> &HashMap<String, Vec<String>> {
        &self.versions
    }
}

impl Default for Changelogs {
    fn default() -> Self {
        Changelogs {
            domain_name: "".to_string(),
            mod_id: 0,
            versions: HashMap::new(),
            etag: "".to_string(),
        }
    }
}

impl Display for Changelogs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "changelogs!")
    }
}

impl Cacheable<(&str, u32)> for Changelogs {
    fn bucket_name() -> &'static str {
        "changelogs"
    }

    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
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
        let compound = Changelogs::key(key);
        let bucket = super::bucket::<Self, (&str, u32)>(db).unwrap();
        let found = bucket.get(&*compound).ok()?;
        found.map(Box::new)
    }

    fn fetch(key: (&str, u32), nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>> {
        // The game & modid are *not* included in the response data. This is okay, but I want it.
        nexus.changelogs(key.0, key.1, etag).map(|mut v| {
            v.domain_name = key.0.to_string();
            v.mod_id = key.1;
            Box::new(v)
        })
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, (&str, u32)>(db).unwrap();
        let compound = Changelogs::key((&self.domain_name, self.mod_id));
        if bucket.set(&*compound, self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }
}

// TODO write a macro for this
impl kv::Value for Changelogs {
    fn to_raw_value(&self) -> Result<kv::Raw, kv::Error> {
        let x = serde_json::to_vec(&self)?;
        Ok(x.into())
    }

    fn from_raw_value(r: kv::Raw) -> Result<Self, kv::Error> {
        let x: Self = serde_json::from_slice(&r)?;
        Ok(x)
    }
}
