use dotenv::dotenv;
use log::{debug, error, warn};
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
    /// Populate the local cache with mods tracked for a specific game.
    Populate {
        /// The game to populate.
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
        /// The number of API calls allowed before stopping.
        #[structopt(default_value = "50")]
        limit: u16,
    },
    /// Test your Nexus API key; whoami
    Validate,
    /// Fetch your list of tracked mods
    Tracked {
        #[structopt(default_value = "all")]
        game: String,
    },
    /// Fetch the list of mods you've endorsed
    Endorsements,
    /// Get Nexus metadata about a game by slug
    Game {
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Show the 10 top all-time trending mods for a game
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
            if let Some(metadata) = find::<GameMetadata, &str>(&game, &store, &mut nexus) {
                let pretty = serde_json::to_string_pretty(&metadata)?;
                println!("{}", pretty);
            }
        }
        Command::Mod { game, mod_id } => {
            if let Some(modinfo) =
                find::<ModInfoFull, (&str, u32)>((&game, mod_id), &store, &mut nexus)
            {
                println!("{}", modinfo);
            }
        }
        Command::Populate { game, limit } => {
            warn!(
                "Populating a local cache for {} with your tracked mods, 50 at a time.",
                game.yellow()
            );

            let gamemeta = find::<GameMetadata, &str>(&game, &store, &mut nexus);

            if gamemeta.is_none() {
                warn!("{} can't be found on the Nexus! Bailing.", game);
                return Ok(());
            }
            // let gamemeta = gamemeta.unwrap();

            let tracked = Tracked::all(&store, &mut nexus);
            if tracked.is_none() {
                anyhow::bail!("Unable to fetch any tracked mods.");
            }
            let tracked = tracked.unwrap();
            let filtered = tracked.by_game(&game);
            println!(
                "You are tracking {} mods total and {} for this game.",
                tracked.mods.len(),
                filtered.len()
            );

            println!("Now iterating tracked mods, caching the first 50 found that weren't already cached.");

            let mut mod_iter = filtered.iter();
            let mut item = mod_iter.next();
            let mut fetches: u16 = 0;

            while item.is_some() {
                let modinfo = item.unwrap();
                let key = (modinfo.domain_name.as_ref(), modinfo.mod_id);

                // Find the next uncached mod.
                let maybe_mod = if let Some(_) = ModInfoFull::local(key, &store) {
                    None
                } else if let Some(m) = ModInfoFull::fetch(key, &mut nexus) {
                    fetches += 1;
                    Some(m)
                } else {
                    None
                };

                if let Some(fullmod) = maybe_mod {
                    println!("   {} -> cache", fullmod.name().green());
                } else {
                    println!(
                        "   ! unable to find {}/{} for caching",
                        modinfo.domain_name,
                        modinfo.mod_id.red()
                    );
                }

                if fetches < limit {
                    item = mod_iter.next();
                } else {
                    item = None;
                }
            }
        }
        Command::Validate => {
            if let Some(user) = AuthenticatedUser::fetch("ignored", &mut nexus) {
                println!("You are logged in as:\n{}", user);
                println!(
                    "\nYou have {} requests remaining this hour and {} for today.",
                    nexus.remaining_hour().bold(),
                    nexus.remaining_day().bold()
                );
            } else {
                warn!("Something went wrong validating your API key.")
            }
        }
        Command::Tracked { game } => {
            if let Some(tracked) = Tracked::all(&store, &mut nexus) {
                if game == "all" {
                    println!("{}", tracked);
                } else {
                    let filtered = tracked.by_game(&game);
                    if filtered.is_empty() {
                        println!("You aren't tracking any mods for {}", game.yellow().bold());
                    } else {
                        let game_meta =
                            find::<GameMetadata, &str>(&game, &store, &mut nexus).unwrap();
                        println!(
                            "You are tracking {} mods for {}.",
                            filtered.len(),
                            game_meta.name().yellow().bold()
                        );
                        filtered.iter().for_each(|m| {
                            if let Some(mod_info) = ModInfoFull::local((&game, m.mod_id), &store) {
                                println!("    {}", mod_info.name());
                            } else {
                                println!("    {} (full info not available)", m.mod_id);
                            }
                        });
                    }
                }
            } else {
                error!("Something went wrong fetching tracked mods. Rerun with -v to get more details.");
            }
        }
        Command::Endorsements => {
            if let Some(opinions) = EndorsementList::all(&store, &mut nexus) {
                println! {"{}", opinions};
            } else {
                error!("Something went wrong fetching endorsements. Rerun with -v to get more details.");
            }
        }
        Command::Trending { game } => {
            let res = nexus.trending(&game)?;
            for item in res.mods.into_iter() {
                println!("{}", item);
                // never waste an opportunity to cache!
                if item.store(&store).is_err() {
                    error!("storing mod failed...");
                };
            }
        }
        Command::Latest { game } => {
            let res = nexus.latest_added(&game)?;
            for item in res.mods.into_iter() {
                if item.available() {
                    println!("{}", item);
                    if item.store(&store).is_err() {
                        error!("storing mod failed...");
                    };
                }
            }
        }
        Command::Updated { game } => {
            let res = nexus.latest_updated(&game)?;
            for item in res.mods.into_iter() {
                println!("{}", item);
                if item.store(&store).is_err() {
                    error!("storing mod failed...");
                };
            }
        }
    }

    Ok(())
}
