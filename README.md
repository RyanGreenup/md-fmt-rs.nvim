# hello.nvim

A minimal Neovim plugin that inserts text at the cursor. Its real purpose is
to be a **scaffolding template**, structured after
[lazydev.nvim](https://github.com/folke/lazydev.nvim): rename `hello` and
replace the feature modules with your own.

## Features

- Insert a configurable greeting
- Insert the current date with an optional strftime format
- Insert a multi-line signature block

## Structure

```
lua/hello/
  init.lua       -- thin public API: setup() + one function per feature
  config.lua     -- defaults, setup(), lazy self-setup via setmetatable
  cmd.lua        -- :Hello subcommand dispatch + completion
  util.lua       -- shared helpers (insert_at_cursor, notify)
  greeting.lua   -- feature module
  date.lua       -- feature module
  signature.lua  -- feature module
doc/hello.nvim.txt
```

Conventions inherited from lazydev.nvim:

- `config.lua` merges user opts over defaults with `vim.tbl_deep_extend` and
  a `setmetatable` `__index` so `require("hello.config").name` works even if
  `setup()` was never called.
- `cmd.lua` exposes one user command (`:Hello`) with subcommands and
  tab completion, instead of one command per feature.
- Feature modules are lazy-required so nothing loads until used.

## Installation

With [lazy.nvim](https://github.com/folke/lazy.nvim):

```lua
{
  "yourname/hello.nvim",
  opts = {},
}
```

## Configuration

```lua
require("hello").setup({
  name = "World",
  greeting = "Hello, %s!",
  date_format = "%Y-%m-%d",
  signature = {
    "--",
    "Sent from hello.nvim",
  },
  debug = false,
})
```

## Usage

| Command                | Action                                       |
| ---------------------- | -------------------------------------------- |
| `:Hello` / `:Hello greet` | Insert the greeting                       |
| `:Hello date [format]` | Insert the date, e.g. `:Hello date %H:%M`    |
| `:Hello sign`          | Insert the signature block                   |

Lua API:

```lua
require("hello").greet()
require("hello").date()
require("hello").sign()
```
