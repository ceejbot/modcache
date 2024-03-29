#![forbid(unsafe_code)]
#![deny(future_incompatible)]
#![warn(
    missing_debug_implementations,
    rust_2018_idioms,
    trivial_casts,
    unused_qualifications
)]

use std::str::FromStr;
use std::sync::Mutex;

use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_complete::{generate, Shell};
use dotenvy::dotenv;
use once_cell::sync::OnceCell;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

pub mod commands;
pub mod data;
pub mod formatting;
pub mod nexus;

use commands::mods::{show_game_mods, show_single_mod};
use commands::*;
use data::*;
use unicase::UniCase;

static REQ_LIMIT: u16 = 50;

// Set up the cli and commands
#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Flags {
    #[clap(subcommand)]
    cmd: Command,
    #[arg(
        short,
        long,
        action = clap::ArgAction::Count,
        help = "Pass -v or -vv to increase verbosity",
        global = true
    )]
    verbose: u8,
    #[arg(
        short,
        long,
        help = "Emit full output as json; not applicable everywhere",
        global = true
    )]
    json: bool,
    #[arg(
        short,
        long,
        help = "Refresh data from the Nexus; not applicable everywhere",
        global = true
    )]
    refresh: bool,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Test your Nexus API key; whoami
    #[clap(alias = "whoami")]
    Validate,
    /// Fetch your list of tracked mods and show a by-game summary.
    Tracked {
        /// Optionally, display a detailed list of tracked mods for a specific game.
        game: Option<String>,
    },
    /// Populate the local cache with mods tracked for a specific game.
    Populate {
        /// The number of API calls allowed before stopping.
        #[clap(short, long, default_value_t = REQ_LIMIT)]
        limit: u16,
        /// The game to populate.
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Refresh your tracked mods and pull new ones to cache.
    ///
    /// Executes `tracked` then `populate` for the given game.
    Update {
        /// The game to update.
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Find mods that mention this string in their names or text summaries.
    ///
    /// Pass --refresh to update cached data from the Nexus for each result.
    Search {
        /// Optional sort for the matches: name, author, date
        #[clap(short, long, default_value = "id")]
        sort: SortKey,
        /// Look for mods that mention this string
        text: String,
        /// The slug for the game to filter by.
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Find mods with names matching the given string, for the named game.
    ///
    /// Pass --refresh to update cached data from the Nexus for each result.
    ByName {
        /// Optional sort for the matches: name, author, date
        #[clap(short, long, default_value = "id")]
        sort: SortKey,
        /// Look for mods with names similar to this
        name: String,
        /// The slug for the game to filter by.
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Find mods by the given author, for the named game.
    ByAuthor {
        /// Optional sort for the matches: name, author, date, id
        #[clap(short, long, default_value = "id")]
        sort: SortKey,
        /// Look for mods with authors similar to this
        author: String,
        /// The slug for the game to filter by.
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Track a specific mod
    Track {
        /// The id of the mod to track
        mod_id: u32,
        /// Which game the mods belong to; Nexus short name
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Stop tracking a mod or list of mods, by id.
    Untrack {
        /// Which game the mods belong to; Nexus short name
        #[clap(short, long, default_value = "skyrimspecialedition")]
        game: String,
        /// The ids of the mods to stop tracking
        ids: Vec<u32>,
    },
    /// Stop tracking all removed mods for a specific game
    UntrackRemoved {
        /// Which game to clean up your tracking list for; Nexus short name
        game: String,
    },
    /// Get changelogs for a specific mod.
    Changelogs {
        /// The id of the mod to fetch changelogs for
        mod_id: u32,
        /// Which game the mods belong to; Nexus short name
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Get the list of files for a specific mod.
    Files {
        /// The id of the mod to fetch files for
        mod_id: u32,
        /// Which game the mods belong to; Nexus short name
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Get information about the mod's primary file, usefully formatted.
    PrimaryFile {
        /// The id of the mod to fetch files for
        mod_id: u32,
        /// Which game the mods belong to; Nexus short name
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Get information about a specific mod file.
    FileInfo {
        /// The id of the mod to fetch files for
        mod_id: u32,
        /// Which game the mods belong to; Nexus short name
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
        /// the id of the file to get detailed info on
        file_id: usize,
    },
    /// Fetch the list of mods you have endorsed
    Endorsements {
        /// Optionally filter displayed endorsements by this game name.
        game: Option<String>,
    },
    /// Endorse a mod or list of mods
    Endorse {
        /// Which game the mods belong to; Nexus short name
        game: String,
        /// The ids of the mods to endorse
        ids: Vec<u32>,
    },
    /// Abstain from endorsing a mod.
    Abstain {
        /// Which game the mods belong to; Nexus short name
        game: String,
        /// The id of the mod to refuse to endorse
        mod_id: u32,
    },
    /// Get Nexus metadata about a game by slug
    Game {
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Get all mods locally cached for this game by slug
    Mods {
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Find mods for this game that are hidden, probably so you can untrack them.
    Hidden {
        /// The slug for the game to consider.
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Find mods for this game that are removed, probably so you can untrack them.
    Removed {
        /// The slug for the game to consider.
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Find mods for this game that were wastebinned by their authors.
    Wastebinned {
        /// The slug for the game to consider.
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Show the 10 top all-time trending mods for a game
    Trending {
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Show 10 mods most recently added for a game
    Latest {
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Show the 10 mods most recently updated for a game
    Updated {
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    /// Display detailed info for a single mod
    Mod {
        /// The id of the mod to show
        mod_id: u32,
        /// Which game the mods belong to; Nexus short name
        #[clap(default_value = "skyrimspecialedition")]
        game: String,
    },
    Completions {
        #[clap(value_enum)]
        shell: Shell,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SortKey {
    Id,
    Name,
    Date,
    Author,
}

impl FromStr for SortKey {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "id" => Ok(SortKey::Id),
            "name" => Ok(SortKey::Name),
            "date" => Ok(SortKey::Date),
            "author" => Ok(SortKey::Author),
            _ => Ok(SortKey::Id),
        }
    }
}

pub trait SortByKey {
    fn sort(&mut self, key: &SortKey);
}

impl SortByKey for Vec<ModInfoFull> {
    fn sort(&mut self, key: &SortKey) {
        match key {
            SortKey::Id => self.sort_by_key(|xs| xs.mod_id()),
            SortKey::Name => self.sort_by_key(|xs| UniCase::new(xs.name())),
            SortKey::Date => self.sort_by_key(|xs| xs.updated_timestamp()),
            SortKey::Author => self.sort_by_key(|xs| UniCase::new(xs.uploaded_by().to_string())),
        }
    }
}

/// A shared reference to our kv store on disk.
static STORE: OnceCell<kv::Store> = OnceCell::new();

/// Fetch our kv store instance
pub fn kvstore() -> &'static kv::Store {
    STORE.get_or_init(|| {
        let dbpath = std::env::var("NEXUS_CACHE_PATH")
            .unwrap_or_else(|_| "./db/nexus_cache.sled".to_string());
        log::debug!("Storing data in {}", dbpath.bold());
        let cfg = kv::Config::new(dbpath);
        kv::Store::new(cfg).expect("unable to create k/v store!")
    })
}

/// A shared reference to our nexus client. This is persistent so we can manage
/// rate limiting and API call limits
static NEXUS: OnceCell<Mutex<nexus::NexusClient>> = OnceCell::new();

/// Fetch our nexus client instance
pub fn nexus_client() -> &'static Mutex<nexus::NexusClient> {
    NEXUS.get_or_init(|| {
        let nexuskey = std::env::var("NEXUS_API_KEY")
            .expect("You must provide your personal Nexus API key in the env var NEXUS_API_KEY.");

        let nexus = nexus::NexusClient::new(nexuskey);
        Mutex::new(nexus)
    })
}

fn main() -> Result<()> {
    dotenv().ok();
    let flags = Flags::parse();

    loggerv::Logger::new()
        .verbosity(flags.verbose as u64)
        .line_numbers(false)
        .module_path(false)
        .colors(true)
        .init()
        .unwrap();

    let mut nexus = nexus_client().lock().unwrap();

    match flags.cmd {
        Command::Validate => {
            handle_validate(&flags, &mut nexus)?;
        }
        Command::Tracked { ref game } => {
            handle_tracked(&flags, game, &mut nexus)?;
        }
        Command::Populate { ref game, limit } => {
            handle_populate(&flags, game, limit, &mut nexus)?;
        }
        Command::Update { ref game } => {
            let mut force_refresh = flags.clone();
            force_refresh.refresh = true;
            handle_tracked(&force_refresh, &None, &mut nexus)?;
            handle_populate(&force_refresh, game, REQ_LIMIT, &mut nexus)?;
        }
        Command::Search {
            ref text,
            ref game,
            ref sort,
        } => {
            search::full_text(&flags, game, text, sort, &mut nexus)?;
        }
        Command::ByAuthor {
            ref author,
            ref game,
            ref sort,
        } => {
            search::by_author(&flags, game, author, sort, &mut nexus)?;
        }
        Command::ByName {
            ref name,
            ref game,
            ref sort,
        } => {
            search::by_name(&flags, game, name, sort, &mut nexus)?;
        }
        Command::Game { ref game } => {
            handle_game(&flags, game, &mut nexus)?;
        }
        Command::Mods { ref game } => {
            show_game_mods(&flags, game, &mut nexus)?;
        }
        Command::Hidden { ref game } => {
            cleanup::hidden(&flags, game, &mut nexus)?;
        }
        Command::Removed { ref game } => {
            cleanup::removed(&flags, game, &mut nexus)?;
        }
        Command::Wastebinned { ref game } => {
            cleanup::wastebinned(&flags, game, &mut nexus)?;
        }
        Command::Mod { ref game, mod_id } => {
            show_single_mod(&flags, game, mod_id, &mut nexus)?;
        }
        Command::Endorsements { ref game } => {
            handle_endorsements(&flags, game, &mut nexus)?;
        }
        Command::Endorse { ref game, ref ids } => {
            mod_actions::endorse(&flags, game, ids, &mut nexus)?;
        }
        Command::Abstain { ref game, mod_id } => {
            mod_actions::abstain(&flags, game, mod_id, &mut nexus)?;
        }
        Command::Track { ref game, mod_id } => {
            mod_actions::track(&flags, game, mod_id, &mut nexus)?;
        }
        Command::Untrack { ref game, ref ids } => {
            mod_actions::untrack(&flags, game, ids, &mut nexus)?;
        }
        Command::UntrackRemoved { ref game } => {
            cleanup::untrack_removed(&flags, game, &mut nexus)?;
        }
        Command::Trending { game } => {
            let res = nexus.trending(&game)?;
            store_and_print(&res.mods, flags.json)?;
        }
        Command::Latest { game } => {
            let res = nexus.latest_added(&game)?;
            store_and_print(&res.mods, flags.json)?;
        }
        Command::Updated { game } => {
            let res = nexus.latest_updated(&game)?;
            store_and_print(&res.mods, flags.json)?;
        }
        // TODO: move these into the browser ui once it exists.
        Command::Changelogs { game, mod_id } => {
            let key = CompoundKey::new(game.clone(), mod_id);
            let maybe = Changelogs::get(&key, flags.refresh, &mut nexus);
            if let Some(changelogs) = maybe {
                if flags.json {
                    let pretty = serde_json::to_string_pretty(&changelogs)?;
                    println!("{}", pretty);
                    return Ok(());
                }
                if let Some(mod_info) = ModInfoFull::get(&key, false, &mut nexus) {
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
        Command::Files { ref game, mod_id } => {
            return commands::files::mod_files(game.as_str(), mod_id, &flags, &mut nexus)
        }
        Command::PrimaryFile { ref game, mod_id } => {
            return commands::files::primary_file(game.as_str(), mod_id, &flags, &mut nexus);
        }
        Command::FileInfo {
            ref game,
            mod_id,
            file_id,
        } => {
            return commands::files::file_by_id(game.as_str(), mod_id, file_id, &flags, &mut nexus);
        }
        Command::Completions { shell } => {
            use clap::CommandFactory;
            let mut app = Flags::command();
            generate(shell, &mut app, "modcache", &mut std::io::stdout())
        }
    }

    Ok(())
}

fn store_and_print(mods: &[ModInfoFull], json: bool) -> Result<()> {
    for item in mods.iter() {
        if item.store().is_err() {
            log::error!("storing mod failed...");
        };
        if json {
            let pretty = serde_json::to_string_pretty(&item)?;
            println!("{}", pretty);
        } else {
            println!("{}", item);
        }
    }
    Ok(())
}
