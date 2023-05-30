set dotenv-load := false

# build release version then run the skyrim updater
all: build skyrim

@tidy:
	cargo clippy --fix
	cargo +nightly fmt

# build release version and stick it in the bindir
@build: _bin clean
	cargo build --release
	cp target/release/modcache bin/
	cp target/release/modcache ~/bin

# refresh tracked mods
update:
    ./bin/modcache --refresh tracked

# populate the cache with missing items
@skyrim: update
	bin/modcache populate

_bin:
	@mkdir -p bin

# clean up the bindir
clean:
	@rm -f bin/modcache

spotless:
	cargo clean
