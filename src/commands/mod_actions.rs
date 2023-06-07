use crate::nexus::NexusClient;
use crate::Flags;

pub fn track(
    _flags: &Flags,
    game: &str,
    mod_id: u32,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    let message = nexus.track(game, mod_id)?;
    let pretty = serde_json::to_string_pretty(&message)?;
    println!("{}", pretty);
    Ok(())
}

pub fn untrack(
    flags: &Flags,
    game: &str,
    ids: &[u32],
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    for mod_id in ids.iter() {
        match nexus.untrack(game, *mod_id) {
            Ok(message) => {
                if flags.json {
                    let pretty = serde_json::to_string_pretty(&message)?;
                    println!("{}", pretty);
                } else {
                    println!("{}", message.message);
                }
            }
            Err(e) => {
                println!("Error untracking {}:\n{:?}", mod_id, e);
            }
        }
    }
    Ok(())
}

pub fn endorse(
    flags: &Flags,
    game: &str,
    ids: &[u32],
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    for mod_id in ids.iter() {
        match nexus.abstain(game, *mod_id) {
            Ok(response) => {
                if flags.json {
                    let pretty = serde_json::to_string_pretty(&response)?;
                    println!("{}", pretty);
                } else {
                    println!(
                        "Endorsement status for mod {} is now {}",
                        mod_id, response.status
                    );
                }
            }
            Err(e) => {
                println!("Error endorsing {}:\n{:?}", mod_id, e);
            }
        }
    }
    Ok(())
}

pub fn abstain(
    flags: &Flags,
    game: &str,
    mod_id: u32,
    nexus: &mut NexusClient,
) -> anyhow::Result<()> {
    let response = nexus.abstain(game, mod_id)?;
    if flags.json {
        let pretty = serde_json::to_string_pretty(&response)?;
        println!("{}", pretty);
    } else {
        println!(
            "Endorsement status for mod {} is now {}",
            mod_id, response.status
        );
    }
    Ok(())
}
