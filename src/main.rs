use std::collections::HashMap;

use dotenv::dotenv;
use itertools::Itertools;
use log::{debug, error, info, warn};
use owo_colors::OwoColorize;
use prettytable::{cell, row, Table};
use serde::Serialize;
use structopt::clap::AppSettings::*;
use structopt::StructOpt;
use term_grid::{Cell, Direction, Filling, Grid, GridOptions};
use terminal_size::*;

pub mod data;
pub mod nexus;

use data::*;

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
    #[structopt(
        short,
        long,
        help = "Emit full output as json; not applicable everywhere"
    )]
    json: bool,
    #[structopt(
        short,
        long,
        help = "Refresh data from the Nexus; not applicable everywhere"
    )]
    refresh: bool,
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
    /// Track a specific mod
    Track {
        /// Which game the mod belongs to; Nexus short name
        game: String,
        /// The id of the mod to track
        mod_id: u32,
    },
    /// Stop tracking a mod
    Untrack {
        /// Which game the mod belongs to; Nexus short name
        game: String,
        /// The id of the mod to track
        mod_id: u32,
    },
    /// Get changelogs for a specific mod.
    Changelogs {
        /// Which game the mod belongs to; Nexus short name
        game: String,
        /// The id of the mod to fetch changelogs for
        mod_id: u32,
    },
    /// Get the list of files for a specific mod. Not very useful yet.
    Files {
        /// Which game the mod belongs to; Nexus short name
        game: String,
        /// The id of the mod to fetch files for
        mod_id: u32,
    },
    /// Fetch the list of mods you've endorsed
    Endorsements {
        /// Optionally filter endorsements by this game name.
        game: Option<String>,
    },
    /// Get Nexus metadata about a game by slug
    Game {
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Get all mods locally cached for this game by slug
    Mods {
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Find mods with names matching the given string, for the named game.
    ByName {
        /// Look for mods with names similar to this
        name: String,
        /// The slug for the game to filter by.
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Find mods that mention this string in their names or text summaries.
    Search {
        /// Look for mods that mention this string
        text: String,
        /// The slug for the game to filter by.
        #[structopt(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Find mods for this game that are hidden, probably so you can untrack them.
    Hidden {
        /// The slug for the game to consider.
        /// The slug for the game to filter by.
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
    /// Display detailed info for a single mod
    Mod {
        /// Which game the mod is for; Nexus short name
        game: String,
        /// The id of the mod to show
        mod_id: u32,
    },
}

fn print_in_grid(items: Vec<impl ToString>) {
    let width = if let Some((Width(w), Height(_h))) = terminal_size() {
        w - 2
    } else {
        72
    };

    let mut grid = Grid::new(GridOptions {
        filling: Filling::Spaces(2),
        direction: Direction::LeftToRight,
    });
    for item in items {
        grid.add(Cell::from(item.to_string()));
    }

    if let Some(g) = grid.fit_into_width(width.into()) {
        // https://github.com/ogham/rust-term-grid/issues/11
        println!("{}", g);
    } else {
        println!("{}", grid.fit_into_columns(10));
    }
}

fn pluralize_mod(count: usize) -> String {
    if count == 1 {
        format!("{} mod", "one".blue())
    } else {
        format!("{} mods", count.blue())
    }
}

fn emit_modlist_with_caption(modlist: Vec<ModInfoFull>, caption: &str) {
    if !modlist.is_empty() {
        println!(
            "{} {}:",
            pluralize_mod(modlist.len()).bold(),
            caption.bold()
        );
        print_in_grid(modlist.iter().map(|xs| xs.mod_id()).collect());
    }
}

fn show_endorsements(
    game: &str,
    modlist: &[UserEndorsement],
    store: &kv::Store,
    client: &mut nexus::NexusClient,
) {
    let game_meta = GameMetadata::get(game, false, store, client).unwrap();
    println!(
        "\n{} endorsed for {}:",
        pluralize_mod(modlist.len()),
        game_meta.name().yellow().bold()
    );
    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_CLEAN);
    modlist.iter().for_each(|opinion| {
        if let Some(mod_info) = ModInfoFull::get((game, opinion.mod_id()), false, store, client) {
            table.add_row(row![
                format!("{}", opinion.status()),
                format!(
                    "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
                    opinion.url(),
                    mod_info.display_name()
                ),
            ]);
        } else {
            table.add_row(row![
                format!("{}", opinion.status()),
                format!(
                    "\x1b]8;;{}\x1b\\uncached mod id #{}\x1b]8;;\x1b\\",
                    opinion.url(),
                    opinion.mod_id()
                ),
            ]);
        }
    });
    println!("{}", table);
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
            if let Some(metadata) = GameMetadata::get(&game, flags.refresh, &store, &mut nexus) {
                let pretty = serde_json::to_string_pretty(&metadata)?;
                println!("{}", pretty);
            } else {
                println!(
                    "No game identified as {} found on the Nexus. Recheck the slug!",
                    game.yellow().bold()
                );
            }
        }
        Command::Mods { game } => {
            if let Some(metadata) = GameMetadata::get(&game, flags.refresh, &store, &mut nexus) {
                for m in metadata.mods(&store).into_iter() {
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
        }
        Command::ByName { name, game } => {
            if let Some(metadata) = GameMetadata::get(&game, flags.refresh, &store, &mut nexus) {
                for m in metadata.mods_name_match(&name, &store).into_iter() {
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
        }
        Command::Search { text, game } => {
            if let Some(metadata) = GameMetadata::get(&game, flags.refresh, &store, &mut nexus) {
                let mods = metadata.mods_match_text(&text, &store);
                if flags.json {
                    let pretty = serde_json::to_string_pretty(&mods)?;
                    println!("{}", pretty);
                } else {
                    if mods.is_empty() {
                        println!("\nNo mods found that match `{}`", text);
                    } else if mods.len() == 1 {
                        println!(
                            "\nOne match found for `{}` in {}:\n",
                            text,
                            metadata.name().yellow().bold()
                        );
                    } else {
                        println!(
                            "\n{} matches found for `{}` in {}:\n",
                            mods.len(),
                            text,
                            metadata.name().yellow().bold()
                        );
                    }

                    for m in mods.into_iter() {
                        println!("{}", m);
                    }
                }
            } else {
                println!(
                    "No game identified as {} found on the Nexus. Recheck the slug!",
                    game.yellow().bold()
                );
            }
        }
        Command::Hidden { game } => {
            if let Some(metadata) = GameMetadata::get(&game, flags.refresh, &store, &mut nexus) {
                let mods = metadata.mods_hidden(&store);
                if flags.json {
                    let pretty = serde_json::to_string_pretty(&mods)?;
                    println!("{}", pretty);
                } else {
                    if mods.is_empty() {
                        println!(
                            "\nNo hidden mods in cache for {}",
                            metadata.name().yellow().bold()
                        );
                    } else if mods.len() == 1 {
                        println!(
                            "\nOne hidden mod in cache for {}:\n",
                            metadata.name().yellow().bold()
                        );
                    } else {
                        println!(
                            "\n{} hidden mods in cache for {}:\n",
                            mods.len(),
                            metadata.name().yellow().bold()
                        );
                    }

                    for m in mods.into_iter() {
                        println!("{}", m);
                    }
                }
            } else {
                println!(
                    "No game identified as {} found on the Nexus. Recheck the slug!",
                    game.yellow().bold()
                );
            }
        }
        Command::Mod { game, mod_id } => {
            if let Some(modinfo) =
                ModInfoFull::get((&game, mod_id), flags.refresh, &store, &mut nexus)
            {
                if flags.json {
                    let pretty = serde_json::to_string_pretty(&modinfo)?;
                    println!("{}", pretty);
                } else {
                    println!("{}", modinfo);
                }
            }
        }
        Command::Populate { game, limit } => {
            let gamemeta = GameMetadata::get(&game, flags.refresh, &store, &mut nexus);
            if gamemeta.is_none() {
                warn!("{} can't be found on the Nexus! Bailing.", game);
                return Ok(());
            }

            let tracked = Tracked::get((), flags.refresh, &store, &mut nexus);
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
                "Now iterating tracked mods, caching the first {} uncached found",
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
                } else if let Some(m) = ModInfoFull::fetch(key, &mut nexus, None) {
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
                    println!("   {} -> cache", fullmod.compact_info());
                }

                if fetches < limit {
                    item = mod_iter.next();
                } else {
                    item = None;
                }
            }
        }
        Command::Validate => {
            if let Some(user) = AuthenticatedUser::fetch("ignored", &mut nexus, None) {
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
                warn!("Something went wrong validating your API key.")
            }
        }
        Command::Tracked { game } => {
            let maybe = Tracked::get((), flags.refresh, &store, &mut nexus);

            if let Some(tracked) = maybe {
                if flags.json {
                    let pretty = serde_json::to_string_pretty(&tracked)?;
                    println!("{}", pretty);
                } else if game == "all" {
                    println!("{}", tracked);
                } else {
                    let filtered = tracked.by_game(&game);
                    if filtered.is_empty() {
                        println!("You aren't tracking any mods for {}", game.yellow().bold());
                    } else {
                        let mut game_meta =
                            GameMetadata::get(&game, flags.refresh, &store, &mut nexus).unwrap();
                        // bucket mods by category, treating removed and wastebinned mods separately.
                        let mut uncached = 0;
                        // I note that this list of special-cases is looking very pattern-like.
                        let mut wasted: Vec<ModInfoFull> = Vec::new();
                        let mut removed: Vec<ModInfoFull> = Vec::new();
                        let mut moderated: Vec<ModInfoFull> = Vec::new();
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
                    }
                }
            } else {
                error!("Something went wrong fetching tracked mods. Rerun with -v to get more details.");
            }
        }
        Command::Track { game, mod_id } => match nexus.track(&game, mod_id) {
            Ok(message) => {
                let pretty = serde_json::to_string_pretty(&message)?;
                println!("{}", pretty);
            }
            Err(_) => {
                println!("Whoops. Run with -v to get more info.");
            }
        },
        Command::Untrack { game, mod_id } => match nexus.untrack(&game, mod_id) {
            Ok(message) => {
                let pretty = serde_json::to_string_pretty(&message)?;
                println!("{}", pretty);
            }
            Err(_) => {
                println!("Whoops. Run with -v to get more info.");
            }
        },
        Command::Changelogs { game, mod_id } => {
            let maybe = Changelogs::get((&game, mod_id), flags.refresh, &store, &mut nexus);
            if let Some(changelogs) = maybe {
                if flags.json {
                    let pretty = serde_json::to_string_pretty(&changelogs)?;
                    println!("{}", pretty);
                    return Ok(());
                }
                if let Some(mod_info) = ModInfoFull::get((&game, mod_id), false, &store, &mut nexus)
                {
                    println!(
                        "\nchangelogs for \x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
                        mod_info.url(),
                        mod_info.display_name()
                    );
                } else {
                    println!("changelogs for {} #{}:", game, mod_id);
                }
                for (version, logs) in changelogs.versions() {
                    println!("\n{}:", version.red());
                    for log in logs {
                        println!("    {}", log);
                    }
                }
            }
        }
        Command::Files { game, mod_id } => {
            let maybe = Files::get((&game, mod_id), flags.refresh, &store, &mut nexus);
            if let Some(files) = maybe {
                let pretty = serde_json::to_string_pretty(&files)?;
                println!("{}", pretty);
                return Ok(());
            } else {
                println!("Nothing found.");
            }
        }
        Command::Endorsements { game } => {
            let maybe = EndorsementList::get((), flags.refresh, &store, &mut nexus);

            if let Some(opinions) = maybe {
                if flags.json {
                    let pretty = serde_json::to_string_pretty(&opinions)?;
                    println!("{}", pretty);
                    return Ok(());
                }

                let mapping = opinions.get_game_map();
                if let Some(g) = game {
                    if let Some(modlist) = mapping.get(&g) {
                        show_endorsements(&g, modlist, &store, &mut nexus);
                    } else {
                        println!("No opinions expressed on mods for {}.", g);
                    }
                } else {
                    println!(
                        "\n{} mods opinionated upon for {} games\n",
                        opinions.mods.len().red(),
                        mapping.len().blue()
                    );
                    for (game, modlist) in mapping.iter() {
                        show_endorsements(game, modlist, &store, &mut nexus);
                    }
                }
            } else {
                error!("Something went wrong fetching endorsements. Rerun with -v to get more details.");
            }
        }
        Command::Trending { game } => {
            let res = nexus.trending(&game)?;
            if flags.json {
                let pretty = serde_json::to_string_pretty(&res)?;
                println!("{}", pretty);
                return Ok(());
            }

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
            if flags.json {
                let pretty = serde_json::to_string_pretty(&res)?;
                println!("{}", pretty);
                return Ok(());
            }
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
            if flags.json {
                let pretty = serde_json::to_string_pretty(&res)?;
                println!("{}", pretty);
                return Ok(());
            }
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
