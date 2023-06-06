# modcache

`modcache` is a Rust cli program that builds and then queries a local cache of the sections [Nexus Mods](https://www.nexusmods.com) registry. I play a lot of Skyrim, and have a long active modlist and and even longer list of [Skyrim SE](https://www.nexusmods.com/skyrimspecialedition) mods I'm interested in shuffling around. I find Nexus's own categorization and search tools to be inadequate and was interested in discovering if a local restructuring of the data would be useful. The data they make available from their API is missing user-provided tags, sadly, but perhaps some full-text search will help?

Another intended use case was for me to scan my list of tracked mods to see which ones I haven't downloaded or kept up to date. This use case is also impossible because the Nexus does not make your download history available through its API. Oh well.

However, the tool is still useful as a *very* rapid local search of all the locally-cached data. Results are sortable; run `modcache search --help` for options.

The output has clickable http links to the Nexus if your terminal supports it. If you have `mdcat` or `glow` installed, the detailed info display for a single mod-- invoked as `modcache mod <id> [game]`-- will render the mod's description in your terminal.

Install Rust for your platform with [rustup](https://rustup.rs). Copy `.env-example` into `.env` and add your api key, which you can find [on the Nexus settings page](https://www.nexusmods.com/users/myaccount?tab=api). Run `cargo run -- help` for usage. `cargo run -- <command> --help` shows detailed help for that command.

```text
Tools for making a local searchable database of the Nexus mod list for a moddable game.

Usage: modcache [OPTIONS] <COMMAND>

Commands:
  validate         Test your Nexus API key; whoami
  tracked          Fetch your list of tracked mods and show a by-game summary
  populate         Populate the local cache with mods tracked for a specific game
  search           Find mods that mention this string in their names or text summaries
  by-name          Find mods with names matching the given string, for the named game
  by-author        Find mods by the given author, for the named game
  track            Track a specific mod
  untrack          Stop tracking a mod or list of mods, by id
  untrack-removed  Stop tracking all removed mods for a specific game
  changelogs       Get changelogs for a specific mod
  files            Get the list of files for a specific mod. Not very useful yet
  endorsements     Fetch the list of mods you have endorsed
  endorse          Endorse a mod or list of mods
  abstain          Abstain from endorsing a mod
  game             Get Nexus metadata about a game by slug
  mods             Get all mods locally cached for this game by slug
  hidden           Find mods for this game that are hidden, probably so you can untrack them
  removed          Find mods for this game that are removed, probably so you can untrack them
  wastebinned      Find mods for this game that were wastebinned by their authors
  trending         Show the 10 top all-time trending mods for a game
  latest           Show 10 mods most recently added for a game
  updated          Show the 10 mods most recently updated for a game
  mod              Display detailed info for a single mod
  help             Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Pass -v or -vv to increase verbosity
  -j, --json        Emit full output as json; not applicable everywhere
  -r, --refresh     Refresh data from the Nexus; not applicable everywhere
  -h, --help        Print help
  -V, --version     Print version
```

My workflow was to run `modcache tracked` to get my full tracked modlist into cache, then run `modcache populate skyrimspecialedition --limit 90` every hour until I had the 3K+ mods I track stored locally.

`--refresh` uses the weak etag the Nexus returns to see if their data has changed. This dings you an API request even if you get a 304 back :(.

If you have [just](https://github.com/casey/just) installed, the justfile provides some conveniences for building and running the tool.

## References

[The Nexus API](https://app.swaggerhub.com/apis-docs/NexusMods/nexus-mods_public_api_params_in_form_data/1.0#/)

## License

[Blue Oak Model License](https://blueoakcouncil.org/license/1.0.0); text in [LICENSE.md](./LICENSE.md).
