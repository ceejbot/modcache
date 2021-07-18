# modcache

`modcache` is a Rust cli program that builds and then queries a local cache of the [Nexus Mods](https://www.nexusmods.com) [Skyrim SE](https://www.nexusmods.com/skyrimspecialedition) registry. I find Nexus's own categorization and search tools to be inadequate and was interested in discovering if a local restructuring of the data would be useful. The data they make available from their API is missing user-provided tags, sadly, but perhaps some full-text search will help?

Another use case is for me to scan my list of tracked mods to see which ones I haven't downloaded or kept up to date.

It remains to be seen how practical it will be to cache mod metadata locally given API rate limits. It might be more practical to scrape their fully-rendered website.

Install Rust for your platform with [rustup](https://rustup.rs). Copy `.env-example` into `.env` and add your api key, which you can find [on the Nexus settings page](https://www.nexusmods.com/users/myaccount?tab=api). Run `cargo run -- --help` for usage.

```sh
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
    endorsements    Fetch the list of mods you've endorsed
    game            Get Nexus metadata about a game by slug
    help            Prints this message or the help of the given subcommand(s)
    latest          Show 10 mods most recently added for a game
    mod             Display detailed info for a single mod
    populate        Populate the local cache with mods tracked for a specific game
    track           Track a specific mod
    tracked         Fetch your list of tracked mods
    trending        Show the 10 top all-time trending mods for a game
    untrack         Stop tracking a mod
    updated         Show the 10 mods most recently updated for a game
    validate        Test your Nexus API key; whoami
```

## References

[The Nexus API](https://app.swaggerhub.com/apis-docs/NexusMods/nexus-mods_public_api_params_in_form_data/1.0#/)

## License

[Blue Oak Model License](https://blueoakcouncil.org/license/1.0.0); text in [LICENSE.md](./LICENSE.md).
