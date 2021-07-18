use std::collections::HashMap;

use dotenv::dotenv;
use log::{debug, error, info, warn};
use owo_colors::OwoColorize;
use prettytable::{cell, row, Table};
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
                tracked.mods.len().blue(),
                filtered.len().blue()
            );

            println!(
                "Now iterating tracked mods, caching the first uncached {} found",
                limit
            );

            let mut mod_iter = filtered.iter();
            let mut item = mod_iter.next();
            let mut fetches: u16 = 0;

            while item.is_some() {
                let modinfo = item.unwrap();
                let key = (modinfo.domain_name.as_ref(), modinfo.mod_id);

                // Find the next uncached mod.
                let maybe_mod = if ModInfoFull::local(key, &store).is_some() {
                    None
                } else if let Some(m) = ModInfoFull::fetch(key, &mut nexus) {
                    m.store(&store)?;
                    fetches += 1;
                    Some(m)
                } else {
                    info!(
                        "   ! unable to find {}/{} for caching",
                        modinfo.domain_name,
                        modinfo.mod_id.red()
                    );
                    None
                };

                if let Some(fullmod) = maybe_mod {
                    println!("   {} -> cache", fullmod.name().green());
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
                        let mut game_meta =
                            find::<GameMetadata, &str>(&game, &store, &mut nexus).unwrap();
                        // bucket mods by category, treating removed and wastebinned mods separately.
                        let mut uncached = 0;
                        let mut wasted: Vec<ModInfoFull> = Vec::new();
                        let mut removed: Vec<ModInfoFull> = Vec::new();
                        let mut cat_map: HashMap<u16, Vec<ModInfoFull>> = HashMap::new();
                        filtered.iter().for_each(|m| {
                            if let Some(mod_info) = ModInfoFull::local((&game, m.mod_id), &store) {
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
                                    _ => {
                                        bucket.push(*mod_info);
                                    }
                                }
                            } else {
                                uncached += 1
                            }
                        });

                        // TODO order these less than arbitrarily
                        for (catid, mods) in cat_map.iter() {
                            if let Some(category) = game_meta.category_from_id(*catid) {
                                println!("----- {}:", category.name().purple());
                            } else {
                                println!("----- category id #{}:", catid.blue());
                            }

                            mods.iter().for_each(|mod_info| {
                                mod_info.print_compact();
                            });
                        }

                        println!(
                            "\nYou are tracking {} mods for {}.",
                            filtered.len().blue(),
                            game_meta.name().yellow().bold()
                        );
                        print!("\n{} mods are marked as removed: ", removed.len().blue());
                        removed.iter().for_each(|xs| print!("{} ", xs.mod_id()));
                        print!(
                            "\n{} mods were wasted by their authors: ",
                            wasted.len().blue()
                        );
                        wasted.iter().for_each(|xs| print!("{} ", xs.mod_id()));
                        println!(
                            "\n\n{} tracked mods are in cache.",
                            (filtered.len() - uncached).blue()
                        );
                        println!("Another {} mods are not yet cached.", uncached.blue());
                    }
                }
            } else {
                error!("Something went wrong fetching tracked mods. Rerun with -v to get more details.");
            }
        }
        Command::Endorsements => {
            if let Some(opinions) = EndorsementList::all(&store, &mut nexus) {
                let mapping = opinions.get_game_map();
                println!(
                    "\n{} mods opinionated upon for {} games\n",
                    opinions.mods.len().red(),
                    mapping.len().blue()
                );

                for (game, modlist) in mapping.iter() {
                    let game_meta =
                        find::<GameMetadata, &str>(game, &store, &mut nexus).unwrap();
                    println!("Endorsements for {}:", game_meta.name().yellow().bold());
                    let mut table = Table::new();
                    table.set_format(*prettytable::format::consts::FORMAT_CLEAN);
                    modlist.iter().for_each(|opinion| {
                        if let Some(mod_info) = ModInfoFull::local((game, opinion.mod_id()), &store) {
                            table.add_row(row![
                                format!("{}", opinion.status()),
                                format!("\x1b]8;;{}\x1b\\{}\x1b]8m;;\x1b\\", opinion.get_url(), mod_info.name().green()),
                            ]);
                        } else {
                            table.add_row(row![
                                format!("{}", opinion.status()),
                                format!("\x1b]8;;{}\x1b\\mod id #{}\x1b]8m;;\x1b\\", opinion.get_url(), opinion.mod_id()),
                            ]);
                        }
                        // \x1b[
                        // printf '\e]8;;http://example.com\e\\This is a link\e]8;;\e\\n'
                    });
                    println!("{}", table);
                }
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
