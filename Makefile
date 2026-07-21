.PHONY: all build test test-rust test-lua fmt lint clean

all: build test

build:
	cargo build --release --manifest-path rust/Cargo.toml

test: test-rust test-lua

test-rust:
	cargo test --manifest-path rust/Cargo.toml

# The Lua tests drive the real binary, so it has to exist first.
test-lua: build
	nvim --headless --clean -c 'set rtp+=.' -l tests/cursor_spec.lua
	nvim --headless --clean -c 'set rtp+=.' -l tests/format_spec.lua

fmt:
	stylua lua/ tests/
	cargo fmt --manifest-path rust/Cargo.toml

lint:
	selene lua/
	cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings

clean:
	cargo clean --manifest-path rust/Cargo.toml
