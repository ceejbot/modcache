use owo_colors::OwoColorize;

use crate::data::modinfo::ModInfoFull;
use crate::data::tracked::Tracked;
use crate::data::{local, Cacheable, CompoundKey};
use crate::formatting::pluralize_mod;
use crate::nexus::NexusClient;
use crate::{Flags, GameMetadata};

pub fn handle(
    flags: &Flags,
    game: &String,
    limit: u16,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    let store = crate::store();

    let gamemeta = GameMetadata::get(game, flags.refresh, store, nexus);
    if gamemeta.is_none() {
        log::warn!("{} can't be found on the Nexus! Bailing.", game);
        return Ok(());
    }

    let tracked = Tracked::get(&Tracked::listkey(), flags.refresh, store, nexus);
    if tracked.is_none() {
        anyhow::bail!("Unable to fetch any tracked mods.");
    }
    let tracked = tracked.unwrap();
    let filtered = tracked.by_game(game);
    println!(
        "You are tracking {} total and {} for this game.",
        pluralize_mod(tracked.mods.len()),
        filtered.len().blue()
    );

    println!(
        "Now iterating tracked mods, caching the first {} uncached found",
        limit
    );

    let mut mod_iter = filtered.iter();
    let mut item = mod_iter.next();
    let mut fetches: u16 = 0;

    while item.is_some() {
        let modinfo = item.unwrap();
        let key = CompoundKey::new(modinfo.domain_name.clone(), modinfo.mod_id);
        // Find the next uncached mod.
        let maybe_mod = if local::<ModInfoFull, CompoundKey>(&key, store).is_some() {
            None
        } else if let Some(m) = ModInfoFull::fetch(&key, nexus, None) {
            m.store(store)?;
            fetches += 1;
            Some(m)
        } else {
            log::info!(
                "   ! unable to find {}/{} for caching",
                modinfo.domain_name,
                modinfo.mod_id.red()
            );
            None
        };

        if let Some(fullmod) = maybe_mod {
            println!("   {} -> cache", fullmod.compact_info());
        }

        if fetches < limit {
            item = mod_iter.next();
        } else {
            item = None;
        }
    }

    Ok(())
}
