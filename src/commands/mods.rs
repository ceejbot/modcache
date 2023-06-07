use anyhow::anyhow;
use owo_colors::OwoColorize;

use crate::data::modinfo::ModInfoFull;
use crate::data::{Cacheable, CompoundKey};
use crate::nexus::NexusClient;
use crate::{Flags, GameMetadata};

pub fn show_game_mods(flags: &Flags, game: &String, nexus: &mut NexusClient) -> anyhow::Result<()> {
    if let Some(metadata) = GameMetadata::get(game, flags.refresh, nexus) {
        for m in metadata.mods().into_iter() {
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

pub fn show_single_mod(
    flags: &Flags,
    game: &String,
    mod_id: u32,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    let key = CompoundKey::new(game.to_string(), mod_id);
    match ModInfoFull::get(&key, flags.refresh, nexus) {
        Some(modinfo) => {
            if flags.json {
                let pretty = serde_json::to_string_pretty(&modinfo)?;
                println!("{}", pretty);
            } else {
                modinfo.print_full_info();
            }
            Ok(())
        }
        None => {
            println!("{}/{} not found on the Nexus!", game, mod_id);
            Err(anyhow!("{}/{} not found on the Nexus!", game, mod_id))
        }
    }
}
