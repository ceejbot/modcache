use dotenv::dotenv;
use log::{debug, warn};
use owo_colors::OwoColorize;
// use prettytable::Table;
use serde::Serialize;
use structopt::clap::AppSettings::*;
use structopt::StructOpt;

pub mod data;
pub mod nexus;

use data::*;

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
        std::env::var("NEXUS_CACHE_PATH").unwrap_or_else(|_| "./db/nexus_cache.sled".to_string());
    debug!("Storing data in {}", dbpath.bold());
    let cfg = kv::Config::new(dbpath);
    let store = kv::Store::new(cfg)?;

    match flags.cmd {
        Command::Game { game } => {
            if let Some(metadata) = find::<GameMetadata>(Key::Name(game), &store, &mut nexus) {
                let pretty = serde_json::to_string_pretty(&metadata)?;
                println!("{}", pretty);
            }
        }
        Command::Mod { game, mod_id } => {
            if let Some(modinfo) = find::<ModInfoFull>(
                Key::NameIdPair {
                    name: game,
                    id: mod_id,
                },
                &store,
                &mut nexus,
            ) {
                println!("{}", modinfo);
            }
        }
        Command::Populate { game } => {
            warn!("TODO: populate a local cache for {}", game.yellow());
            let tracked = Tracked::all(&store, &mut nexus);
            if tracked.is_none() {
                anyhow::bail!("Unable to fetch any tracked mods.");
            }
            let tracked = tracked.unwrap();
            println!("You are tracking {} mods.", tracked.mods.len());
            let mapped = tracked.get_game_map();

            // First get metadata for all our games.
            for (key, val) in mapped.iter() {
                println!(
                    "Populating {} tracked mods for {}",
                    val.len().bold(),
                    key.yellow().bold()
                );
                if find::<GameMetadata>(Key::Name(key.to_string()), &store, &mut nexus).is_some() {
                    debug!("    {} metadata now in cache", key.yellow().bold());
                }
                /*
                // Now walk all mod_ids in the vec and store those. Except skyrim for now.
                if key == "skyrimspecialedition" {
                    continue;
                }
                let mut count = 0;
                val.iter().for_each(|id| {
                    if let Some(modinfo) = ModInfoFull::find(
                        Key::NameIdPair {
                            name: key.clone(),
                            id: *id,
                        },
                        &store,
                        &mut nexus,
                    ) {
                        count += 1;
                        println!("{}", modinfo);
                    }
                });
                println!("   cached data for {} mods", count);
                */
            }
        }
        Command::Validate => {
            if let Some(user) =
                AuthenticatedUser::fetch(Key::Name("authed_user".to_string()), &mut nexus)
            {
                warn!("You are logged in as:\n{}", user);
            } else {
                warn!("Something went wrong validating your API key.")
            }
        }
        Command::Tracked => {
            if let Some(tracked) = Tracked::all(&store, &mut nexus) {
                println!("{}", tracked);
            } else {
                println!("Something went wrong fetching tracked mods. Rerun with -v to get more details.");
            }
        }
        Command::Endorsements => {
            if let Some(opinions) = EndorsementList::all(&store, &mut nexus) {
                println!{"{}", opinions};
            } else {
                println!("Something went wrong fetching endorsements. Rerun with -v to get more details.");
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
