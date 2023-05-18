use owo_colors::OwoColorize;

use crate::data::Cacheable;
use crate::nexus::NexusClient;
use crate::{AuthenticatedUser, Flags};

pub fn validate(flags: &Flags, nexus: &mut NexusClient) -> anyhow::Result<()> {
    if let Some(user) = AuthenticatedUser::fetch(&"ignored", nexus, None) {
        if flags.json {
            let pretty = serde_json::to_string_pretty(&user)?;
            println!("{}", pretty);
        } else {
            println!("You are logged in as:\n{}", user);
            println!(
                "\nYou have {} requests remaining this hour and {} for today.",
                nexus.remaining_hour().bold(),
                nexus.remaining_day().bold()
            );
        }
    } else {
        log::warn!("Something went wrong validating your API key.")
    }

    Ok(())
}
