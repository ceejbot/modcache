use serde::{Deserialize, Serialize};

use crate::nexus::NexusClient;
use crate::Cacheable;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileInfo {
    id: Vec<usize>,
    uuid: Option<String>,
    file_id: usize,
    name: String,
    version: String,
    category_id: u32,
    category_name: Option<String>,
    is_primary: bool,
    size: u64,
    file_name: String,
    uploaded_timestamp: usize,
    uploaded_time: String,
    mod_version: String,
    external_virus_scan_url: String,
    description: String,
    size_kb: usize,
    size_in_bytes: u64,
    changelog_html: String,
    content_preview_link: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileUpdates {
    old_file_id: usize,
    new_file_id: usize,
    old_file_name: String,
    new_file_name: String,
    uploaded_timestamp: usize,
    uploaded_time: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct Files {
    domain_name: String,
    mod_id: u32,
    etag: String,
    files: Vec<FileInfo>,
    file_updates: Vec<FileUpdates>,
}

impl Files {
    pub fn key(key: (&str, u32)) -> String {
        format!("{}/{}", key.0, key.1)
    }
}

impl Default for Files {
    fn default() -> Self {
        Files {
            domain_name: "".to_string(),
            mod_id: 0,
            etag: "".to_string(),
            files: Vec::new(),
            file_updates: Vec::new(),
        }
    }
}

impl Cacheable<(&str, u32)> for Files {
    fn bucket_name() -> &'static str {
        "files"
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
        let compound = Files::key(key);
        let bucket = super::bucket::<Self, (&str, u32)>(db).unwrap();
        let found = bucket.get(&*compound).ok()?;
        found.map(Box::new)
    }

    fn fetch(key: (&str, u32), nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>> {
        // The game & modid are *not* included in the response data. This is okay, but I want it.
        nexus.files(key.0, key.1, etag).map(|mut v| {
            v.domain_name = key.0.to_string();
            v.mod_id = key.1;
            Box::new(v)
        })
    }

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self, (&str, u32)>(db).unwrap();
        let compound = Files::key((&self.domain_name, self.mod_id));
        if bucket.set(&*compound, self.clone()).is_ok() {
            Ok(1)
        } else {
            Ok(0)
        }
    }
}

// TODO write a macro for this
impl kv::Value for Files {
    fn to_raw_value(&self) -> Result<kv::Raw, kv::Error> {
        let x = serde_json::to_vec(&self)?;
        Ok(x.into())
    }

    fn from_raw_value(r: kv::Raw) -> Result<Self, kv::Error> {
        let x: Self = serde_json::from_slice(&r)?;
        Ok(x)
    }
}
