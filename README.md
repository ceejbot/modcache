# modcache

`modcache` is a Rust cli program that builds and then queries a local cache of the [Nexus Mods](https://www.nexusmods.com) [Skyrim SE](https://www.nexusmods.com/skyrimspecialedition) registry. I find Nexus's own categorization and search tools to be inadequate and was interested in discovering if a local restructuring of the data would be useful. The data they make available from their API is missing user-provided tags, sadly, but perhaps some full-text search will help?

Another use case is for me to scan my list of tracked mods to see which ones I haven't downloaded or kept up to date.

It remains to be seen how practical it will be to cache mod metadata locally given API rate limits. It might be more practical to scrape their fully-rendered website.

## Hacking

Install Rust for your platform with [rustup](https://rustup.rs). The tool uses [sqlite3](https://sqlite.org/index.html) for local storage. There are some conveniences provided as Makefile targets to wrap up tasks like creating the db and running migrations. To use the migration targets, you'll need the [refinery migration tool](https://lib.rs/crates/refinery_cli) installed. On OX X, with homebrew installed:

```sh
brew install sqlite
cargo install refinery_cli
make create
make build
bin/modcache --help
```

## References

[The Nexus API](https://app.swaggerhub.com/apis-docs/NexusMods/nexus-mods_public_api_params_in_form_data/1.0#/)

## License

[Blue Oak Model License](https://blueoakcouncil.org/license/1.0.0); text in [LICENSE.md](./LICENSE.md).
