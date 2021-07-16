# modcache

`modcache` is a Rust cli program that builds and then queries a local cache of the [Nexus Mods](https://www.nexusmods.com) [Skyrim SE](https://www.nexusmods.com/skyrimspecialedition) registry. I find Nexus's own categorization and search tools to be inadequate and was interested in discovering if a local restructuring of the data would be useful. The data they make available from their API is missing user-provided tags, sadly, but perhaps some full-text search will help?

Another use case is for me to scan my list of tracked mods to see which ones I haven't downloaded or kept up to date.

It remains to be seen how practical it will be to cache mod metadata locally given API rate limits. It might be more practical to scrape their fully-rendered website.

Install Rust for your platform with [rustup](https://rustup.rs). Copy `.env-example` into `.env` and add your api key, which you can find [on the Nexus settings page](https://www.nexusmods.com/users/myaccount?tab=api). Run `cargo run -- help` for usage.

## References

[The Nexus API](https://app.swaggerhub.com/apis-docs/NexusMods/nexus-mods_public_api_params_in_form_data/1.0#/)

## License

[Blue Oak Model License](https://blueoakcouncil.org/license/1.0.0); text in [LICENSE.md](./LICENSE.md).
