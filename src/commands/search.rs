use owo_colors::OwoColorize;

use crate::data::modinfo::ModInfoFull;
use crate::data::Cacheable;
use crate::nexus::NexusClient;
use crate::{Flags, GameMetadata};

fn emit_search_results(
    flags: &Flags,
    filter: &str,
    metadata: GameMetadata,
    mods: Vec<ModInfoFull>,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    if flags.json {
        let pretty = serde_json::to_string_pretty(&mods)?;
        println!("{}", pretty);
        Ok(())
    } else {
        if mods.is_empty() {
            println!("\nNo mods found matching `{}`", filter);
        } else if mods.len() == 1 {
            println!(
                "\nOne match found for `{}` in {}:\n",
                filter,
                metadata.name().yellow().bold()
            );
        } else {
            println!(
                "\n{} matches found for `{}` in {}:\n",
                mods.len(),
                filter,
                metadata.name().yellow().bold()
            );
        }

        for m in mods.into_iter() {
            let refreshed = if flags.refresh {
                ModInfoFull::get(&m.key(), true, nexus)
            } else {
                None
            };
            let info = match refreshed {
                Some(v) => *v,
                None => m,
            };
            println!("{info}");
        }

        Ok(())
    }
}

pub fn by_name(
    flags: &Flags,
    game: &String,
    filter: &str,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    let Some(metadata) = GameMetadata::get(game, flags.refresh, nexus) else {
        println!(
            "No game identified as {} found on the Nexus. Recheck the slug!",
            game.yellow().bold()
        );
        return Ok(());
    };

    let mods = metadata.mods_name_match(filter);
    emit_search_results(flags, filter, *metadata, mods, nexus)?;

    Ok(())
}

pub fn full_text(
    flags: &Flags,
    game: &String,
    filter: &str,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    let Some(metadata) = GameMetadata::get(game, flags.refresh, nexus) else {
        println!(
            "No game identified as {} found on the Nexus. Recheck the slug!",
            game.yellow().bold()
        );
        return Ok(());
    };

    let mods = metadata.mods_match_text(filter);
    emit_search_results(flags, filter, *metadata, mods, nexus)?;

    Ok(())
}
