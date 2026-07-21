check:
    cd rust && cargo build
    cd rust && cargo build --release
    cd rust && cargo fmt --check
    cd rust && cargo clippy --all-targets -- -D warnings
    @just test
    stylua --check .

test:
    cd rust && cargo test
    # nvim --headless -u lua/mdfmt/init.lua -l ./tests/cursor_spec.lua
    # nvim --headless -u lua/mdfmt/init.lua -l ./tests/format_spec.lua
