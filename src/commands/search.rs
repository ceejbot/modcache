use owo_colors::OwoColorize;

use crate::data::Cacheable;
use crate::nexus::NexusClient;
use crate::Flags;
use crate::GameMetadata;

pub fn by_name(
    flags: &Flags,
    game: &String,
    filter: &str,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    let store = crate::store();

    if let Some(metadata) = GameMetadata::get(game, flags.refresh, store, nexus) {
        for m in metadata.mods_name_match(filter, store).into_iter() {
            if flags.json {
                let pretty = serde_json::to_string_pretty(&m)?;
                println!("{}", pretty);
            } else {
                println!("{}", m);
            }
        }
    } else {
        println!(
            "No game identified as {} found on the Nexus. Recheck the slug!",
            game.yellow().bold()
        );
    }

    Ok(())
}

pub fn full_text(
    flags: &Flags,
    game: &String,
    filter: &str,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    let store = crate::store();

    if let Some(metadata) = GameMetadata::get(game, flags.refresh, store, nexus) {
        let mods = metadata.mods_match_text(filter, store);
        if flags.json {
            let pretty = serde_json::to_string_pretty(&mods)?;
            println!("{}", pretty);
        } else {
            if mods.is_empty() {
                println!("\nNo mods found that match `{}`", filter);
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
                println!("{}", m);
            }
        }
    } else {
        println!(
            "No game identified as {} found on the Nexus. Recheck the slug!",
            game.yellow().bold()
        );
    }

    Ok(())
}
