all: release

bin:
	@mkdir -p bin

release: bin clean
	@cargo build --release
	@cp target/release/modcache bin/

skyrim: release
	@bin/modcache --refresh tracked skyrimspecialedition
	@bin/modcache populate

clean:
	@rm -f bin/modcache

spotless:
	cargo clean

.phony: release clean spotless
