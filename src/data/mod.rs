use std::fmt::Debug;

use log::info;
use serde::{Deserialize, Serialize};

pub mod endorsement;
pub mod game;
pub mod modinfo;
pub mod tracked;
pub mod user;

pub use endorsement::*;
pub use game::*;
pub use modinfo::*;
pub use tracked::*;
pub use user::*;

use crate::nexus::NexusClient;

// Nexus mod data structs and trait implementations, plus caching layer.
// More complex structures are broken out into separate files.

/// Get the item, looking in local cache first then calling to the Nexus if not found.
// This isn't great yet, I guess, what with the attack of the clones.
pub fn find<T, K: Debug + Clone>(key: K, db: &kv::Store, nexus: &mut NexusClient) -> Option<Box<T>>
where
    T: Cacheable<K>,
{
    if let Some(found) = T::local(key.clone(), db) {
        info!("cache hit for {:?}", key);
        return Some(found);
    }
    if let Some(fetched) = T::fetch(key.clone(), nexus, None) {
        info!("fetched from the Nexus for {:?}", key);
        if fetched.store(db).is_ok() {
            info!("cached {:?}", key);
        }
        return Some(fetched);
    }
    None
}

pub fn refresh<T, K: Debug + Clone>(
    key: K,
    db: &kv::Store,
    nexus: &mut NexusClient,
) -> Option<Box<T>>
where
    T: Cacheable<K> + HasEtag,
{
    if let Some(found) = T::local(key.clone(), db) {
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

pub trait HasEtag {
    fn etag(&self) -> &str;
    fn set_etag(&mut self, etag: &str);
}

pub trait Cacheable<T>
where
    Self: kv::Value,
{
    /// Get the name of the bucket where these items are stored.
    fn bucket_name() -> &'static str;
    /// Look for the item locally.
    fn local(key: T, db: &kv::Store) -> Option<Box<Self>>;
    /// Fetch this item from the Nexus.
    fn fetch(key: T, nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>>;
    /// Store this item in local cache.
    fn store(&self, db: &kv::Store) -> anyhow::Result<usize>;
}

// no home for this structure yet; it's used by several nexus fetches

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModInfoList {
    pub mods: Vec<ModInfoFull>,
    pub etag: String,
}
