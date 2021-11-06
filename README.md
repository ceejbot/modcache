# modcache

`modcache` is a Rust cli program that builds and then queries a local cache of the sections [Nexus Mods](https://www.nexusmods.com) registry. I play a lot of Skyrim, and have a long active modlist and and even longer list of [Skyrim SE](https://www.nexusmods.com/skyrimspecialedition) mods I'm interested in shuffling around. I find Nexus's own categorization and search tools to be inadequate and was interested in discovering if a local restructuring of the data would be useful. The data they make available from their API is missing user-provided tags, sadly, but perhaps some full-text search will help?

Another use case is for me to scan my list of tracked mods to see which ones I haven't downloaded or kept up to date.

It remains to be seen how practical it will be to cache mod metadata locally given API rate limits. It might be more practical to scrape their fully-rendered website (respecting any `robots.txt`, of course).

Install Rust for your platform with [rustup](https://rustup.rs). Copy `.env-example` into `.env` and add your api key, which you can find [on the Nexus settings page](https://www.nexusmods.com/users/myaccount?tab=api). Run `cargo run -- help` for usage. `cargo run -- <command> --help` shows detailed help for that command.

```text
modcache 0.1.0
ask questions about nexus mod data

USAGE:
    modcache [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -j, --json       Emit full output as json; not applicable everywhere
    -r, --refresh    Refresh data from the Nexus; not applicable everywhere
    -V, --version    Prints version information
    -v, --verbose    Pass -v or -vv to increase verbosity

SUBCOMMANDS:
    abstain            Abstain from endorsing a mod
    by-name            Find mods with names matching the given string, for the named game
    changelogs         Get changelogs for a specific mod
    endorse            Endorse a mod or list of mods
    endorsements       Fetch the list of mods you have endorsed
    files              Get the list of files for a specific mod. Not very useful yet
    game               Get Nexus metadata about a game by slug
    help               Prints this message or the help of the given subcommand(s)
    hidden             Find mods for this game that are hidden, probably so you can untrack them
    latest             Show 10 mods most recently added for a game
    mod                Display detailed info for a single mod
    mods               Get all mods locally cached for this game by slug
    populate           Populate the local cache with mods tracked for a specific game
    removed            Find mods for this game that are removed, probably so you can untrack them
    search             Find mods that mention this string in their names or text summaries
    track              Track a specific mod
    tracked            Fetch your list of tracked mods and show a by-game summary
    trending           Show the 10 top all-time trending mods for a game
    untrack            Stop tracking a mod or list of mods, by id
    untrack-removed    Stop tracking all removed mods for a specific game
    updated            Show the 10 mods most recently updated for a game
    validate           Test your Nexus API key; whoami
    wastebinned        Find mods for this game that were wastebinned by their authors
```

My workflow was to run `modcache tracked` to get my full tracked modlist into cache, then run `modcache populate 90 skyrimspecialedition` every hour until I had the 2K+ mods I track stored locally.

`--refresh` uses the weak etag the Nexus returns to see if their data has changed. This dings you an API request even if you get a 304 back :(.

If you have [just](https://github.com/casey/just) installed, the justfile provides some conveniences for running the tool

## References

[The Nexus API](https://app.swaggerhub.com/apis-docs/NexusMods/nexus-mods_public_api_params_in_form_data/1.0#/)

## License

[Blue Oak Model License](https://blueoakcouncil.org/license/1.0.0); text in [LICENSE.md](./LICENSE.md).
