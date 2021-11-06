set dotenv-load := false

all: build skyrim

build: bin clean
	@cargo build --release
	@cp target/release/modcache bin/

update:
    ./bin/modcache --refresh tracked

skyrim: update
	@bin/modcache populate

bin:
	@mkdir -p bin

clean:
	@rm -f bin/modcache

spotless:
	cargo clean
