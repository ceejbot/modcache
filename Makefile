BOLD=\033[0;1;32m
NORMAL=\033[m

all: build

create:
	@mkdir -p db
	@touch db/nexus_cache.db
	@refinery migrate files -p ./migrations

migrate:
	@refinery migrate files -p ./migrations

build: migrate
	@echo "Building $(BOLD)$*$(NORMAL)..."
	@cargo build --release
	@mkdir -p bin/
	@cp target/release/modcache bin/

populate: build
	@bin/modcache populate --help

clean:
	rm -f *.tar *.gz releases/*
	rmdir releases

spotless: clean
	cargo clean

.PHONY: build fetch clean spotless
