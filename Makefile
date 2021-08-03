all: release

bin:
	@mkdir -p bin

release: bin clean
	@cargo build --release
	@cp target/release/modcache bin/

clean:
	@rm bin/modcache

spotless:
	cargo clean

.phony: release clean spotless
