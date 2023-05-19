//! Nexus mod data structs and trait implementations, plus caching layer.
//! More complex structures are broken out into separate files.

use std::fmt::{Debug, Display};

use kv::{Codec, Json};
use serde::{Deserialize, Serialize};

pub mod changelogs;
pub mod endorsement;
pub mod files;
pub mod game;
pub mod modinfo;
pub mod tracked;
pub mod user;

pub use changelogs::*;
pub use endorsement::*;
pub use files::*;
pub use game::*;
pub use modinfo::*;
pub use tracked::*;
pub use user::*;

use crate::nexus::NexusClient;

/// Get the item, looking in local cache first then calling to the Nexus if not found.
/// Set refresh to true if you want to check the Nexus even if you have a cache hit.
pub fn get<T>(
    key: &<T as Cacheable>::K,
    refresh: bool,
    db: &kv::Store,
    nexus: &mut NexusClient,
) -> Option<Box<T>>
where
    T: Cacheable + Debug,
{
    if let Some(found) = local::<T>(key, db) {
        if refresh {
            if let Some(fetched) = T::fetch(key, nexus, Some(found.etag().to_string())) {
                log::info!("    ↪ refreshed nexus data");
                let merged = found.update(&fetched);
                match merged.store(db) {
                    Ok(_) => {
                        log::info!("    ✓ cached nexus data");
                    }
                    Err(e) => {
                        log::warn!("Failed to store refreshed object! {e:?}");
                    }
                }
                Some(Box::new(merged))
            } else {
                log::info!("    ↩ no update; responding with cached");
                Some(found)
            }
        } else {
            Some(found)
        }
    } else if let Some(fetched) = T::fetch(key, nexus, None) {
        log::info!("    ﹢ first fetch of nexus data");
        if fetched.store(db).is_ok() {
            log::info!("    ✓ cached new nexus data");
        }
        Some(fetched)
    } else {
        log::info!("    ␀nexus gave us nothing");
        None
    }
}

/// Given a bucket name and appropriate types, return a kv bucket for the data.
pub fn bucket<T>(db: &kv::Store) -> Option<kv::Bucket<'static, &'static str, Json<T>>>
where
    T: Cacheable,
{
    match db.bucket::<&str, Json<T>>(Some(T::bucket_name())) {
        Err(e) => {
            log::info!("Can't open bucket {}! {:?}", T::bucket_name(), e);
            None
        }
        Ok(v) => Some(v),
    }
}

/// Look for an item locally, in the kv store, by type and key.
pub fn local<T>(key: &<T as Cacheable>::K, db: &kv::Store) -> Option<Box<T>>
where
    T: Cacheable,
{
    let bucket = bucket::<T>(db).unwrap();
    let found: Option<Json<T>> = bucket.get(&&*key.to_string()).ok()?;
    found.map(|x| Box::new(x.into_inner()))
}

/// The main trait for objects we store.
pub trait Cacheable
where
    Self: for<'de> Deserialize<'de> + Serialize + Debug,
{
    type K: Debug + Clone + Display + Into<String>;

    /// Get the name of the bucket where these items are stored.
    fn bucket_name() -> &'static str;
    /// Get an item of this type, looking in local storage first then fetching from the Nexus if it
    /// isn't found locally. Set `refresh` to true to do a conditional GET to the Nexus for updated
    /// data even if we have a local hit.
    fn get(
        key: &Self::K,
        refresh: bool,
        db: &kv::Store,
        nexus: &mut NexusClient,
    ) -> Option<Box<Self>>;
    /// Fetch an item from the Nexus by key.
    fn fetch(key: &Self::K, nexus: &mut NexusClient, etag: Option<String>) -> Option<Box<Self>>;
    /// Get this item's key
    fn key(&self) -> Self::K;
    /// Get an etag for this data.
    fn etag(&self) -> &str;
    /// Set the etag for this data.
    fn set_etag(&mut self, etag: &str);
    /// Store this item in local cache.
    fn store(&self, db: &kv::Store) -> anyhow::Result<usize>;
    /// Merge properties, if wanted, before storing an updated version of this object.
    fn update(&self, other: &Self) -> Self;
}

/// A commonly-used key type that composes the game's name and a mod id.
#[derive(Debug, Clone)]
pub struct CompoundKey {
    domain_name: String,
    mod_id: u32,
}

impl CompoundKey {
    pub fn new(domain_name: String, mod_id: u32) -> Self {
        Self {
            domain_name,
            mod_id,
        }
    }
}

impl Display for CompoundKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.domain_name, self.mod_id)
    }
}

impl From<&CompoundKey> for String {
    fn from(val: &CompoundKey) -> Self {
        val.clone().to_string()
    }
}

impl From<CompoundKey> for String {
    fn from(val: CompoundKey) -> Self {
        val.to_string()
    }
}

// no home for this structure yet; it's used by several nexus fetches

/// A list of full mod data objects. Returned by the game-wide trending and updated lists.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct ModInfoList {
    pub mods: Vec<ModInfoFull>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EndorseResponse {
    message: String,
    pub status: EndorsementStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrackingResponse {
    pub message: String,
}
