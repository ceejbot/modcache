use std::fmt::Debug;

use log::info;
use serde::{Deserialize, Serialize};

pub mod changelogs;
pub mod endorsement;
pub mod game;
pub mod modinfo;
pub mod tracked;
pub mod user;

pub use changelogs::*;
pub use endorsement::*;
pub use game::*;
pub use modinfo::*;
pub use tracked::*;
pub use user::*;

use crate::nexus::NexusClient;

// Nexus mod data structs and trait implementations, plus caching layer.
// More complex structures are broken out into separate files.

/// Get the item, looking in local cache first then calling to the Nexus if not found.
/// Set refresh to true if you want to check the Nexus even if you have a cache hit.
pub fn get<T, K: Debug + Clone>(
    key: K,
    refresh: bool,
    db: &kv::Store,
    nexus: &mut NexusClient,
) -> Option<Box<T>>
where
    T: Cacheable<K>,
{
    if let Some(found) = T::local(key.clone(), db) {
        if refresh {
            if let Some(fetched) = T::fetch(key, nexus, Some(found.etag().to_string())) {
                println!("    refreshed nexus data");
                if fetched.store(db).is_ok() {
                    info!("cached nexus data");
                }
                Some(fetched)
            } else {
                info!("no update; responding with cached");
                Some(found)
            }
        } else {
            Some(found)
        }
    } else if let Some(fetched) = T::fetch(key, nexus, None) {
        println!("    first fetch of nexus data");
        if fetched.store(db).is_ok() {
            info!("cached refreshed nexus data");
        }
        Some(fetched)
    } else {
        info!("nexus gave us nothing?");
        None
    }
}

/// Given a bucket name and appropriate types, return a bucket for the data.
pub fn bucket<T, K: Debug + Clone>(db: &kv::Store) -> Option<kv::Bucket<'static, &'static str, T>>
where
    T: Cacheable<K>,
{
    match db.bucket::<&str, T>(Some(T::bucket_name())) {
        Err(e) => {
            info!("Can't open bucket {}! {:?}", T::bucket_name(), e);
            None
        }
        Ok(v) => Some(v),
    }
}

pub trait Cacheable<T>
where
    Self: kv::Value,
{
    /// Get an etag for this data.
    fn etag(&self) -> &str;
    /// Set the etag for this data.
    fn set_etag(&mut self, etag: &str);
    /// Get the name of the bucket where these items are stored.
    fn bucket_name() -> &'static str;
    /// Look for the item locally.
    fn local(key: T, db: &kv::Store) -> Option<Box<Self>>;
    /// Fetch this item from the Nexus.
    fn fetch(key: T, nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>>;
    /// Store this item in local cache.
    fn store(&self, db: &kv::Store) -> anyhow::Result<usize>;
    fn get(key: T, refresh: bool, db: &kv::Store, nexus: &mut NexusClient) -> Option<Box<Self>>;
}

// no home for this structure yet; it's used by several nexus fetches

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModInfoList {
    pub mods: Vec<ModInfoFull>,
    pub etag: String,
}
