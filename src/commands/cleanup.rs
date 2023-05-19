use std::collections::HashSet;

use owo_colors::OwoColorize;

use crate::data::modinfo::ModInfoFull;
use crate::data::tracked::Tracked;
use crate::data::Cacheable;
use crate::nexus::NexusClient;
use crate::{Flags, GameMetadata};

pub fn hidden(flags: &Flags, game: &String, nexus: &mut NexusClient) -> anyhow::Result<()> {
    let store = crate::store();

    let Some(metadata) = GameMetadata::get(game, flags.refresh, store, nexus) else {
        println!(
            "No game identified as {} found on the Nexus. Recheck the slug!",
            game.yellow().bold()
        );
        return Ok(())
    };

    let mut mods = metadata.mods_hidden(store);
    if let Some(all_tracked) = Tracked::get(&Tracked::listkey(), flags.refresh, store, nexus) {
        // filter tracked mods from hidden
        let tracked: HashSet<u32> = all_tracked
            .by_game(game)
            .iter()
            .map(|xs| xs.mod_id)
            .collect();
        mods.retain(|xs| tracked.contains(&xs.mod_id()));
    }

    if flags.json {
        let pretty: String = serde_json::to_string_pretty(&mods)?;
        println!("{}", pretty);
    } else {
        if mods.is_empty() {
            println!(
                "\nNo hidden but tracked mods in cache for {}",
                metadata.name().yellow().bold()
            );
        } else if mods.len() == 1 {
            println!(
                "\nOne hidden but tracked mod in cache for {}:\n",
                metadata.name().yellow().bold()
            );
        } else {
            println!(
                "\n{} hidden but tracked mods in cache for {}:\n",
                mods.len(),
                metadata.name().yellow().bold()
            );
        }

        // Hidden mods might become unhidden, so we check if we've asked to
        // refresh.
        for m in mods.into_iter() {
            let refreshed = if flags.refresh {
                ModInfoFull::get(&m.key(), true, store, nexus)
            } else {
                None
            };
            let info = match refreshed {
                Some(v) => *v,
                None => m,
            };
            println!("{}", info.compact_info());
        }
    }

    Ok(())
}

pub fn removed(flags: &Flags, game: &String, nexus: &mut NexusClient) -> anyhow::Result<()> {
    let store = crate::store();
    let Some(metadata) = GameMetadata::get(game, flags.refresh, store, nexus) else {
        println!(
            "No game identified as {} found on the Nexus. Recheck the slug!",
            game.yellow().bold()
        );
        return Ok(())
    };

    let mods = metadata.mods_removed(store);
    if flags.json {
        let pretty = serde_json::to_string_pretty(&mods)?;
        println!("{}", pretty);
    } else {
        if mods.is_empty() {
            println!(
                "\nNo removed mods in cache for {}",
                metadata.name().yellow().bold()
            );
        } else {
            println!("\nRemoved mods for {}:\n", metadata.name().yellow().bold());
        }

        for m in mods.into_iter() {
            println!("{}", m.compact_info());
        }
    }

    Ok(())
}

pub fn wastebinned(flags: &Flags, game: &String, nexus: &mut NexusClient) -> anyhow::Result<()> {
    let store = crate::store();

    let Some(metadata) = GameMetadata::get(game, flags.refresh, store, nexus) else {
        println!(
            "No game identified as {} found on the Nexus. Recheck the slug!",
            game.yellow().bold()
        );
        return Ok(())
    };

    let mods = metadata.mods_wastebinned(store);
    if flags.json {
        let pretty = serde_json::to_string_pretty(&mods)?;
        println!("{}", pretty);
    } else {
        if mods.is_empty() {
            println!(
                "\nNo wastebinned mods in cache for {}",
                metadata.name().yellow().bold()
            );
        } else {
            println!(
                "\nWastebinned mods for {}:\n",
                metadata.name().yellow().bold()
            );
        }

        for m in mods.into_iter() {
            println!("{}\n{}\n", m.compact_info(), m.url());
        }
    }

    Ok(())
}

pub fn untrack_removed(
    flags: &Flags,
    game: &String,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    let store = crate::store();

    let Some(metadata) = GameMetadata::get(game, flags.refresh, store, nexus) else {
        println!(
            "No game identified as {} found on the Nexus. Recheck the slug!",
            game.yellow().bold()
        );
        return Ok(())
    };

    let maybe = Tracked::get(&Tracked::listkey(), flags.refresh, store, nexus);
    if let Some(all_tracked) = maybe {
        let tracked: HashSet<u32> = all_tracked
            .by_game(game)
            .iter()
            .map(|xs| xs.mod_id)
            .collect();
        let mods = metadata.mods_removed(store);
        for m in mods.into_iter() {
            // we minimize api calls to the nexus
            if tracked.contains(&m.mod_id()) {
                match nexus.untrack(game, m.mod_id()) {
                    Ok(_) => {
                        println!("untracked {}", m.mod_id().red());
                    }
                    Err(e) => {
                        println!("Error untracking {}:\n{:?}", m.mod_id(), e);
                    }
                }
            }
        }
    }

    Ok(())
}
