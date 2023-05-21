use std::collections::HashMap;

use itertools::Itertools;
use owo_colors::OwoColorize;

use crate::data::modinfo::{ModInfoFull, ModStatus};
use crate::data::tracked::Tracked;
use crate::data::{local, Cacheable, CompoundKey};
use crate::formatting::{emit_modlist_with_caption, pluralize_mod};
use crate::nexus::NexusClient;
use crate::{Flags, GameMetadata};

pub fn handle(flags: &Flags, game: &Option<String>, nexus: &mut NexusClient) -> anyhow::Result<()> {
    let maybe = Tracked::get(&Tracked::listkey(), flags.refresh, nexus);
    let Some(tracked) = maybe else {
        log::error!("Something went wrong fetching tracked mods. Rerun with -v to get more details.");
        return Ok(());
    };

    if game.is_none() {
        if flags.json {
            let pretty = serde_json::to_string_pretty(&tracked)?;
            println!("{}", pretty);
        } else {
            println!("{}", tracked);
        }
        return Ok(());
    }

    let game = game.clone().unwrap();
    let filtered = tracked.by_game(&game);
    if filtered.is_empty() {
        println!("You aren't tracking any mods for {}", game.yellow().bold());
        return Ok(());
    }

    let mut game_meta = GameMetadata::get(&game, flags.refresh, nexus).unwrap();
    // bucket mods by category, treating removed and wastebinned mods separately.
    let mut uncached = 0;
    // I note that this list of special-cases is looking very pattern-like.
    let mut wasted: Vec<ModInfoFull> = Vec::new();
    let mut removed: Vec<ModInfoFull> = Vec::new();
    let mut moderated: Vec<ModInfoFull> = Vec::new();
    let mut cat_map: HashMap<u16, Vec<ModInfoFull>> = HashMap::new();
    filtered.iter().for_each(|m| {
        let key = CompoundKey::new(game.clone(), m.mod_id);
        if let Some(mod_info) = local::<ModInfoFull>(&key) {
            let bucket = cat_map
                .entry(mod_info.category_id())
                .or_insert_with(Vec::new);
            match mod_info.status() {
                ModStatus::Wastebinned => {
                    wasted.push(*mod_info);
                }
                ModStatus::Removed => {
                    removed.push(*mod_info);
                }
                ModStatus::UnderModeration => {
                    moderated.push(*mod_info);
                }
                _ => {
                    bucket.push(*mod_info);
                }
            }
        } else {
            uncached += 1
        }
    });

    for (catid, mods) in cat_map.into_iter().sorted_by_key(|xs| xs.0) {
        if let Some(category) = game_meta.category_from_id(catid) {
            println!("----- {}:", category.name().purple());
        } else {
            println!("----- category id #{}:", catid.blue());
        }

        mods.iter()
            .sorted_by_key(|xs| xs.mod_id())
            .for_each(|mod_info| {
                println!("    {}", mod_info.compact_info());
            });
    }

    println!(
        "\nYou are tracking {} for {}.",
        pluralize_mod(filtered.len()),
        game_meta.name().yellow().bold()
    );
    if uncached == 0 {
        println!("All {} are in cache.", pluralize_mod(filtered.len()));
    } else {
        println!("{} are in cache.", pluralize_mod(filtered.len() - uncached));
        println!("Another {} not yet cached.", pluralize_mod(uncached));
    }
    println!();

    emit_modlist_with_caption(removed, "removed");
    emit_modlist_with_caption(wasted, "wastebinned by their authors");
    emit_modlist_with_caption(moderated, "under moderation");

    Ok(())
}
