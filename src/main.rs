use dotenv::dotenv;
use log::{debug, info, warn};
use owo_colors::OwoColorize;
// use prettytable::Table;
use serde::Serialize;
use structopt::clap::AppSettings::*;
use structopt::StructOpt;

pub mod data;
pub mod nexus;

use data::*;

use crate::data::AuthenticatedUser;

// static MOST_RECENT_ID: u32 = 52368;

// Set up the cli and commands
#[derive(Clone, Serialize, StructOpt)]
#[structopt(name = "modcache", about = "ask questions about nexus mod data")]
#[structopt(global_setting(ColoredHelp), global_setting(ColorAuto))]
pub struct Flags {
    #[structopt(
        short,
        long,
        parse(from_occurrences),
        help = "Pass -v or -vv to increase verbosity"
    )]
    verbose: u64,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Clone, Serialize, StructOpt)]
enum Command {
    /// Populate the local cache, starting either from 0 or from the passed-in mod id
    Populate {
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
        #[structopt(default_value = "0")]
        start: String,
    },
    /// Test your Nexus API key; whoami
    Validate,
    /// Fetch your list of tracked mods
    Tracked,
    /// Fetch the list of mods you've endorsed
    Endorsements,
    /// Get Nexus metadata about a game by slug
    Game {
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Show the 10 mods top all-time trending mods for a game
    Trending {
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Show 10 mods most recently added for a game
    Latest {
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Show the 10 mods most recently updated for a game
    Updated {
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Fetch info about a mod by id and game
    Mod {
        #[structopt(long, short, default_value = "skyrimspecialedition")]
        game: String,
        mod_id: u32,
    },
}

fn main() -> anyhow::Result<(), anyhow::Error> {
    dotenv().ok();
    let nexuskey = std::env::var("NEXUS_API_KEY")
        .expect("You must provide your personal Nexus API key in the env var NEXUS_API_KEY.");
    let flags = Flags::from_args();

    loggerv::Logger::new()
        .verbosity(flags.verbose)
        .line_numbers(false)
        .module_path(false)
        .colors(true)
        .init()
        .unwrap();

    let mut nexus = nexus::NexusClient::new(nexuskey);
    let dbpath =
        std::env::var("NEXUS_CACHE_PATH").unwrap_or_else(|_| "./db/nexus_cache.db".to_string());
    debug!("Storing data in {}", dbpath.bold());
    let storage = rusqlite::Connection::open(&dbpath)?;

    match flags.cmd {
        Command::Game { game } => {
            if let Some(metadata) = GameMetadata::fetch(game, &storage, &mut nexus) {
                let pretty = serde_json::to_string_pretty(&metadata)?;
                println!("{}", pretty);
            }
        }
        Command::Mod { game, mod_id } => {
            let res = nexus.mod_by_id(&game, mod_id)?;
            // let pretty = serde_json::to_string_pretty(&res)?;
            println!("{}", res);
        }
        Command::Populate { game, start } => {
            warn!(
                "TODO: populate a local cache for {} starting at {}",
                game, start
            );
            let tracked = Tracked::fetch("".to_string(), &storage, &mut nexus);
            if tracked.is_none() {
                anyhow::bail!("Unable to fetch any tracked mods.");
            }
            let tracked = tracked.unwrap();
            println!("You are tracking {} mods.", tracked.mods.len());
            /*
             Steps:
                Fetch tracked mods. Iterate list.
                If present in the db:
                    fetch etag; use etag for conditional nexus request
                    refresh data if updated
                If not present:
                    fetch mod data & populate db
            */
        }
        Command::Validate => {
            if let Some(user) = AuthenticatedUser::fetch("".to_string(), &storage, &mut nexus) {
                warn!("You are logged in as:\n{}", user);
            } else {
                warn!("Something went wrong validating your API key.")
            }
        }
        Command::Tracked => {
            if let Some(tracked) = Tracked::fetch("".to_string(), &storage, &mut nexus) {
                println!("{}", tracked);
            }
        }
        Command::Endorsements => {
            let opinions = nexus.endorsements()?;
            // A very imperative way of doing this, but I hated iterating more than once.
            let mut endorsed: u16 = 0;
            let mut abstained: u16 = 0;
            opinions.mods.iter().for_each(|xs| match xs.status() {
                EndorsementStatus::Endorsed => endorsed += 1,
                EndorsementStatus::Abstained => abstained += 1,
                _ => {}
            });
            println!(
                "You have endorsed {} mods and abstained for {}.",
                endorsed, abstained
            );

            // This display is pretty useless, but leaving it for now.
            for item in opinions.mods.into_iter() {
                println!("{}", item);
            }
        }
        Command::Trending { game } => {
            let res = nexus.trending(&game)?;
            for item in res.mods.into_iter() {
                println!("{}", item);
            }
        }
        Command::Latest { game } => {
            let res = nexus.latest_added(&game)?;
            for item in res.mods.into_iter() {
                if item.available() {
                    println!("{}", item);
                }
            }
        }
        Command::Updated { game } => {
            let res = nexus.latest_updated(&game)?;
            for item in res.mods.into_iter() {
                println!("{}", item);
            }
        }
    }

    Ok(())
}
