use owo_colors::OwoColorize;

use crate::data::Cacheable;
use crate::nexus::NexusClient;
use crate::{Flags, GameMetadata};

pub fn handle(flags: &Flags, game: &String, nexus: &mut NexusClient) -> anyhow::Result<()> {
    let store = crate::store();

    if let Some(metadata) = GameMetadata::get(game, flags.refresh, store, nexus) {
        for m in metadata.mods(store).into_iter() {
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