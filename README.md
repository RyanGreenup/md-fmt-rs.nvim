# md-fmt-rs.nvim

A Markdown and MDX formatter for Neovim, backed by
[comrak](https://github.com/kivikakk/comrak).

## Table of contents

1. [What this project does](#what-this-project-does)
2. [Who this project is for](#who-this-project-is-for)
3. [Dependencies](#dependencies)
4. [Install md-fmt-rs.nvim](#install-mdfmtnvim)
5. [Configure md-fmt-rs.nvim](#configure-mdfmtnvim)
6. [Run md-fmt-rs.nvim](#run-mdfmtnvim)
7. [How MDX is handled](#how-mdx-is-handled)
8. [What happens to your cursor](#what-happens-to-your-cursor)
9. [Troubleshoot md-fmt-rs.nvim](#troubleshoot-mdfmtnvim)
10. [Contributing](#contributing)
11. [Additional documentation](#additional-documentation)
12. [How to get help](#how-to-get-help)
13. [Terms of use](#terms-of-use)

## What this project does

With md-fmt-rs.nvim you can reflow a Markdown or MDX buffer to a fixed width,
normalize its lists, tables, and code fences, and leave its JSX untouched. Run
`:MdFmt`, or set `format_on_save` and forget the command exists.

Two things separate it from wiring Prettier or `mdformat` into a general
formatter plugin:

* **MDX survives.** Prettier reformats the Markdown inside your components;
  mdformat refuses MDX outright. md-fmt-rs.nvim locates every MDX construct with
  [markdown-rs](https://github.com/wooorm/markdown-rs), masks it, formats
  around it, and pastes the original bytes back. Your `<Callout>` interiors come
  back byte for byte.
* **Your cursor stays put.** The result goes in as a minimal diff, and when
  rewrapping rewrites the line you are sitting on, the cursor follows the word
  it was on rather than the column number it was at. Marks, folds, and undo
  granularity survive.

The formatter itself is a small Rust binary in this repository's `rust/`
directory. The plugin compiles it for you and keeps it current; you supply
`cargo`.

## Who this project is for

This project is for Neovim users who write Markdown or MDX and want formatting
that behaves like a code formatter: fast, deterministic, and unwilling to
rewrite the parts it does not understand.

If you write no MDX and already run Prettier for everything else, you have
less to gain here.

## Dependencies

Before using md-fmt-rs.nvim, you need:

* **Neovim 0.11 or later.** The plugin calls `vim.system()` for the subprocess
  and `vim.text.diff()` for the minimal-diff apply, falling back to `vim.diff()`
  where that is all there is.
* **A Rust toolchain** on `$PATH`, from [rustup](https://rustup.rs/) or your
  package manager. The crate depends on two libraries and builds in a few
  seconds. If you would rather not have one, build `md-fmt` elsewhere and point
  [`bin`](#configure-mdfmtnvim) at it.
* **A plugin manager**, such as
  [lazy.nvim](https://github.com/folke/lazy.nvim). The instructions below use
  lazy.nvim, but nothing in the plugin depends on it.

## Install md-fmt-rs.nvim

1. Add the plugin to your lazy.nvim spec:

    ```lua
    {
      "ryangreenup/md-fmt-rs.nvim",
      build = "cargo build --release --manifest-path rust/Cargo.toml",
      opts = {},
    }
    ```

    Passing `opts` makes lazy.nvim call `require("mdfmt").setup()`, which
    registers the `:MdFmt` command. The `build` key is a courtesy: with
    `auto_build` on, the plugin compiles the binary the first time you format a
    buffer, and recompiles after an update that changes the Rust source.

2. Open a Markdown file and run `:MdFmt status`. It reports the binary's path,
   whether it exists, whether it looks stale, and its version.

3. Run `:MdFmt`. If the paragraphs rewrap at 80 columns, the install worked.

To try the plugin without installing it, clone the repository and start Neovim
with the clone on the runtimepath:

```sh
make build
nvim --clean -c 'set rtp+=/path/to/md-fmt-rs.nvim' -c 'lua require("mdfmt").setup()' README.md
```

## Configure md-fmt-rs.nvim

Pass a table to `setup()`. Anything you omit keeps its default:

```lua
require("mdfmt").setup({
  -- Filetypes :MdFmt will touch.
  filetypes = { "markdown", "markdown.mdx", "mdx" },
  -- Column to wrap prose at. 0 leaves line breaks where you put them.
  width = 80,
  -- Frontmatter fence, or false to treat a leading --- as a thematic break.
  frontmatter = "---",
  -- Format on :w. Blocks the write while the binary runs.
  format_on_save = false,
  -- Path to a prebuilt md-fmt, if you would rather manage it yourself.
  bin = nil,
  -- Build the binary when it is missing or older than the crate source.
  auto_build = true,
  cargo = "cargo",
  -- Milliseconds to wait for a synchronous format.
  timeout = 5000,
  debug = false,
})
```

Two settings deserve a note:

* **`width = 0`** turns wrapping off. Everything else still runs, so this is
  the setting for a repository that keeps one sentence per line.
* **`bin`** opts out of the build machinery entirely. That binary is yours: the
  plugin never rebuilds it and never calls it stale.

## Run md-fmt-rs.nvim

| Command | Lua | Result |
| --- | --- | --- |
| `:MdFmt` or `:MdFmt format` | `require("mdfmt").format()` | Formats the current buffer |
| `:MdFmt build` | `require("mdfmt").build()` | Compiles the binary, stale or not |
| `:MdFmt status` | `require("mdfmt").status()` | Reports the binary's path, state, and version |

Press `<Tab>` after `:MdFmt ` to complete the subcommand names.

`format()` accepts `opts.buf` to target another buffer and `opts.sync` to block
until the binary returns instead of applying the result from a callback. Format
on save uses the synchronous path, because a `BufWritePre` callback cannot delay
the write.

To bind formatting to a key:

```lua
vim.keymap.set("n", "<leader>mf", function()
  require("mdfmt").format()
end, { desc = "Format Markdown" })
```

The binary also runs on its own, over stdin, if you want it in a shell pipeline
or another editor:

```sh
md-fmt --width 100 --mdx < page.mdx
```

## How MDX is handled

comrak does not understand [MDX](https://mdxjs.com/), and teaching it would
mean forking a CommonMark parser. So markdown-rs does one job: it reports the
byte spans of the MDX constructs. Those spans become placeholders that comrak
carries through untouched, comrak formats the Markdown around them, and the
original bytes go back where they came from.

The consequence worth knowing is that MDX interiors are frozen. Markdown inside
`<Callout>...</Callout>` keeps whatever you wrote, because the interior of a JSX
element is the component's business, and reformatting it can change what the
component receives.

Invalid MDX is an error rather than a best effort. When `:MdFmt` reports a parse
failure, the buffer is exactly as you left it.

MDX is detected from the `.mdx` file extension before the filetype, because
Neovim ships no filetype rule for `.mdx` and plenty of people edit it in a
buffer that calls itself `markdown`.

## What happens to your cursor

The formatted text goes in as a minimal diff: only the lines that actually
changed are rewritten. Marks, folds, and extmarks elsewhere in the buffer
survive, and the undo history stays granular instead of collapsing into one
opaque step.

When the cursor's own line is rewritten, which rewrapping does often, the cursor
is carried by the word it was sitting on. That word is found again in the
replacement text and the cursor follows it, keeping its offset within the word.
When the word is gone, the cursor falls back to the equivalent position in the
replacement.

If you type while the binary is running, the result is discarded rather than
applied over your edit.

## Troubleshoot md-fmt-rs.nvim

| Issue | Solution |
| --- | --- |
| `:MdFmt` reports `E492: Not an editor command` | `setup()` never ran, and that call registers the command. Call `require("mdfmt").setup()`, or give lazy.nvim an `opts` table. |
| `cargo is not executable` | The plugin compiles its own formatter. Install [Rust](https://rustup.rs/), or point `bin` at a binary you built elsewhere. |
| `could not parse MDX` | The buffer is not valid MDX. Note that HTML comments are invalid in MDX outside a code block; use `{/* ... */}`. |
| Formatting does nothing | Run `:MdFmt status`. If `exists` is false the build failed; run `:MdFmt build` and read the cargo output. |
| `not a Markdown or MDX buffer` | The buffer's filetype is not in `filetypes` and its name does not end in `.mdx`. Check `:set filetype?`. |
| Lines are still longer than `width` | Tables, link destinations, and MDX blobs have no break points in them. comrak wraps at spaces only, so anything without one overflows. |

## Contributing

Open an issue with the output of `:MdFmt status`, your Neovim version from
`:version`, and the smallest input that reproduces the problem. A formatter bug
is usually a few lines of Markdown, so paste them.

The [`Makefile`](Makefile) covers the loop:

```sh
make build   # cargo build --release
make test    # Rust unit tests, then the Lua specs against the real binary
make fmt     # stylua and cargo fmt
make lint    # selene and cargo clippy
```

`make test-lua` builds the binary first, because the Lua specs in
[`tests/`](tests) drive the real thing rather than a stub. Formatting changes
belong in [`rust/tests/format.rs`](rust/tests/format.rs) alongside a fixture;
buffer and cursor changes belong in the Lua specs.

## Additional documentation

* [`doc/mdfmt.txt`](doc/mdfmt.txt) is the in-editor help file. Read it with
  `:help md-fmt-rs.nvim` once your plugin manager has generated the tags.
* [comrak](https://github.com/kivikakk/comrak) does the CommonMark formatting.
  Its options are what `width`, `frontmatter`, and the enabled extensions
  (tables, strikethrough, task lists, alerts, autolinks, footnotes) map onto.
* [markdown-rs](https://github.com/wooorm/markdown-rs) supplies the MDX spans.
  Its README documents which MDX constructs it recognizes.
* [The MDX specification](https://mdxjs.com/docs/what-is-mdx/) explains the
  constructs the formatter preserves and the rules that make an input invalid.
* [`md-fmt --help`](rust/src/cli.rs) lists the binary's flags, which is the
  fastest way to reason about what the plugin passes it.

## How to get help

* Open an issue on this repository for bugs and feature requests.
* Ask about Neovim plugins in the
  [Neovim Discourse](https://neovim.discourse.group/) or on
  [r/neovim](https://reddit.com/r/neovim).
* Report a wrong formatting decision that has nothing to do with MDX
  [upstream to comrak](https://github.com/kivikakk/comrak/issues) instead. This
  plugin sets comrak's options; it does not implement the rendering.

## Terms of use

This project ships no license file yet. Add one before publishing, and name it
here. [choosealicense.com](https://choosealicense.com/) walks through the common
options in a few minutes.
