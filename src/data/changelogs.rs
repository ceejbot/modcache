// All structs and trait impls supporting the full mod info response from the Nexus.

use kv::Json;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::Display;

use crate::nexus::NexusClient;
use crate::{Cacheable, CompoundKey};

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

impl Cacheable<CompoundKey> for Changelogs {
    fn bucket_name() -> &'static str {
        "changelogs"
    }

    fn key(&self) -> CompoundKey {
        CompoundKey {
            domain_name: self.domain_name.clone(),
            mod_id: self.mod_id,
        }
    }

    fn etag(&self) -> &str {
        &self.etag
    }

    fn set_etag(&mut self, etag: &str) {
        self.etag = etag.to_string()
    }

    fn get(
        key: &CompoundKey,
        refresh: bool,
        db: &kv::Store,
        nexus: &mut NexusClient,
    ) -> Option<Box<Self>> {
        super::get::<Self, CompoundKey>(key, refresh, db, nexus)
    }

    fn fetch(
        key: &CompoundKey,
        nexus: &mut NexusClient,
        etag: Option<String>,
    ) -> Option<Box<Self>> {
        // The game & modid are *not* included in the response data. This is okay, but I want it.
        nexus
            .changelogs(&key.domain_name, key.mod_id, etag)
            .map(|mut v| {
                v.domain_name = key.domain_name.clone();
                v.mod_id = key.mod_id;
                Box::new(v)
            })
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, CompoundKey>(db).unwrap();
        if bucket
            .set(&&*self.key().to_string(), &Json(self.clone()))
            .is_ok()
        {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    fn update(&self, other: &Self) -> Self {
        other.clone()
    }
}
