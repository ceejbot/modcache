use kv::Json;
use serde::{Deserialize, Serialize};

use crate::nexus::NexusClient;
use crate::{Cacheable, CompoundKey};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileInfo {
    category_id: u32,
    category_name: Option<String>,
    changelog_html: Option<String>,
    content_preview_link: String,
    description: String,
    external_virus_scan_url: String,
    file_id: usize,
    file_name: String,
    id: Vec<usize>,
    is_primary: bool,
    mod_version: String,
    name: String,
    size_in_bytes: u64,
    size_kb: usize,
    size: u64,
    uploaded_time: String,
    uploaded_timestamp: usize,
    uuid: Option<String>,
    version: String,
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

impl Cacheable for Files {
    type K = CompoundKey;

    fn bucket_name() -> &'static str {
        "files"
    }

    fn get(
        key: &CompoundKey,
        refresh: bool,
        db: &kv::Store,
        nexus: &mut NexusClient,
    ) -> Option<Box<Self>> {
        super::get::<Self>(key, refresh, db, nexus)
    }

    fn fetch(
        key: &CompoundKey,
        nexus: &mut NexusClient,
        etag: Option<String>,
    ) -> Option<Box<Self>> {
        // The game & modid are *not* included in the response data. This is okay, but I want it.
        nexus
            .files(&key.domain_name, key.mod_id, etag)
            .map(|mut v| {
                v.domain_name = key.domain_name.clone();
                v.mod_id = key.mod_id;
                Box::new(v)
            })
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

    fn store(&self, db: &kv::Store) -> anyhow::Result<usize> {
        let bucket = super::bucket::<Self>(db).unwrap();
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
