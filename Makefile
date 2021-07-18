all: release

bin:
	@mkdir -p bin

release: bin
	@cargo build --release
	@cp target/release/modcache bin/

clean:
	cargo clean --release

spotless:
	cargog clean

.phony: release clean spotless
