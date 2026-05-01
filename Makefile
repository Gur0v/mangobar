PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin

.PHONY: build check clean fmt install run

build:
	cargo build --release

check:
	cargo fmt --check
	cargo check

clean:
	cargo clean

fmt:
	cargo fmt

install: build
	install -Dm755 target/release/mangobar $(DESTDIR)$(BINDIR)/mangobar

run:
	cargo run
