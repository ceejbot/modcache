use anyhow::Result;

use crate::data::{Cacheable, CompoundKey, Files};
use crate::nexus::NexusClient;
use crate::Flags;

pub fn mod_files(game: &str, mod_id: u32, flags: &Flags, nexus: &mut NexusClient) -> Result<()> {
    let key = CompoundKey::new(game.to_string(), mod_id);
    if let Some(all_files) = Files::get(&key, flags.refresh, nexus) {
        let all_files = *all_files;

        for f in all_files.files() {
            f.print_compact_info();
            println!();
        }
        // let pretty = serde_json::to_string_pretty(&files)?;
        // println!("{}", pretty);
        return Ok(());
    }
    println!("Nothing found.");
    Ok(())
}

pub fn primary_file(game: &str, mod_id: u32, flags: &Flags, nexus: &mut NexusClient) -> Result<()> {
    let key = CompoundKey::new(game.to_string(), mod_id);
    if let Some(modfiles) = Files::get(&key, flags.refresh, nexus) {
        if let Some(found) = modfiles.primary_file() {
            println!("\n");
            found.print_full_info();
            return Ok(());
        }
    }
    println!("Nothing found.");
    Ok(())
}

pub fn file_by_id(
    game: &str,
    mod_id: u32,
    file_id: usize,
    flags: &Flags,
    nexus: &mut NexusClient,
) -> Result<()> {
    let key = CompoundKey::new(game.to_string(), mod_id);
    if let Some(mod_files) = Files::get(&key, flags.refresh, nexus) {
        if let Some(found) = mod_files.file_by_id(file_id) {
            println!("\n");
            found.print_full_info();
            return Ok(());
        }
    }
    println!("Nothing found.");
    Ok(())
}
