use owo_colors::OwoColorize;

use crate::data::Cacheable;
use crate::nexus::NexusClient;
use crate::Flags;
use crate::GameMetadata;
use crate::Tracked;

pub fn handle(flags: &Flags, game: &String, nexus: &mut NexusClient) -> anyhow::Result<()> {
    let store = crate::store();

    if let Some(metadata) = GameMetadata::get(game, flags.refresh, store, nexus) {
        if flags.json {
            let pretty = serde_json::to_string_pretty(&metadata)?;
            println!("{}", pretty);
        } else {
            metadata.emit_fancy(store);

            let mods = metadata.mods(store);
            println!(
                "There are {} mods in cache for this game.",
                mods.len().blue()
            );

            let tracked = Tracked::get(&Tracked::listkey(), flags.refresh, store, nexus);
            if let Some(tracked) = tracked {
                let filtered = tracked.by_game(game);
                println!(
                    "You are tracking {} mods for this game.",
                    filtered.len().blue()
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
