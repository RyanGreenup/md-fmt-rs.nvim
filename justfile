check:
    cd rust && cargo build
    cd rust && cargo fmt --check
    cd rust && cargo clippy --all-targets -- -D warnings
    @just test
    stylua --check .

test:
    cd rust && cargo test
    cd rust && cargo build --release
    nvim --headless --clean -c 'set rtp+=.' -l tests/cursor_spec.lua
    nvim --headless --clean -c 'set rtp+=.' -l tests/table_spec.lua
    nvim --headless --clean -c 'set rtp+=.' -l tests/format_spec.lua
