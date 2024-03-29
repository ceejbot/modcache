use owo_colors::OwoColorize;

use crate::data::Cacheable;
use crate::formatting::pluralize_mod;
use crate::nexus::NexusClient;
use crate::{Flags, GameMetadata, Tracked};

pub fn handle(flags: &Flags, game: &String, nexus: &mut NexusClient) -> anyhow::Result<()> {
    if let Some(metadata) = GameMetadata::get(game, flags.refresh, nexus) {
        if flags.json {
            let pretty = serde_json::to_string_pretty(&metadata)?;
            println!("{}", pretty);
        } else {
            metadata.emit_fancy();

            let mods = metadata.mods();
            println!(
                "There are {} in cache for this game.",
                pluralize_mod(mods.len())
            );

            let tracked = Tracked::get(&Tracked::listkey(), flags.refresh, nexus);
            if let Some(tracked) = tracked {
                let filtered = tracked.by_game(game);
                println!(
                    "You are tracking {} for {}.",
                    pluralize_mod(filtered.len()),
                    game.yellow()
                );
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
