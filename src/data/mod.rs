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
    if let Some(fetched) = T::fetch(key.clone(), nexus) {
        info!("fetched from the Nexus for {:?}", key);
        if fetched.store(db).is_ok() {
            info!("cached {:?}", key);
        }
        return Some(fetched);
    }
    None
}

pub trait Cacheable<T>
where
    Self: kv::Value,
{
    /// Get the kv/sled bucket where these items are stored.
    fn bucket(db: &kv::Store) -> Option<kv::Bucket<'static, &'static str, Self>>;
    /// Look for the item locally.
    fn local(key: T, db: &kv::Store) -> Option<Box<Self>>;
    /// Fetch this item from the Nexus.
    fn fetch(key: T, nexus: &mut NexusClient) -> Option<Box<Self>>;
    /// Store this item in local cache.
    fn store(&self, db: &kv::Store) -> anyhow::Result<usize>;
}

// no home for this structure yet; it's used by several nexus fetches

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct ModInfoList {
    pub mods: Vec<ModInfoFull>,
}
