# hello.nvim

A working Neovim plugin you can rename and gut to start your own.

## Table of contents

1. [What this project does](#what-this-project-does)
2. [Who this project is for](#who-this-project-is-for)
3. [Dependencies](#dependencies)
4. [Install hello.nvim](#install-hellonvim)
5. [Configure hello.nvim](#configure-hellonvim)
6. [Run hello.nvim](#run-hellonvim)
7. [How the code is organized](#how-the-code-is-organized)
8. [Turn this template into your own plugin](#turn-this-template-into-your-own-plugin)
9. [Troubleshoot hello.nvim](#troubleshoot-hellonvim)
10. [Contributing](#contributing)
11. [Additional documentation](#additional-documentation)
12. [How to get help](#how-to-get-help)
13. [Terms of use](#terms-of-use)

## What this project does

hello.nvim inserts a greeting, a timestamp, or a signature block at the cursor.
Those three features are deliberately trivial; the scaffolding around them is
the point: a config module that merges user options over defaults, a single user
command that dispatches to subcommands and completes them, a shared utility
module, and a help file.

Most plugin templates give you an empty `lua/` directory and a license. This one
gives you three features that already work end to end, so you can delete one,
copy its shape, and know your replacement will load. The layout follows
[lazydev.nvim](https://github.com/folke/lazydev.nvim), a small production plugin
by folke, so the habits you pick up here match plugins people actually read.

## Who this project is for

This project is for Neovim users who write Lua and want to publish their first
plugin without deciding, from scratch, where the config lives or how to register
commands.

## Dependencies

Before using hello.nvim, you need:

* **Neovim 0.7 or later.** The newest API the plugin calls is
  `vim.api.nvim_create_user_command`, which arrived in 0.7. Everything else
  predates it. Tested on 0.11.
* **A plugin manager**, such as [lazy.nvim](https://github.com/folke/lazy.nvim).
  The instructions below use lazy.nvim, but nothing in the plugin depends on it.

To develop the template further, you also want
[StyLua](https://github.com/JohnnyMorganz/StyLua) for formatting and
[Selene](https://github.com/Kampfkarren/selene) for linting. The repository
ships a `stylua.toml`, a `selene.toml`, and a `vim.yml` standard library
definition, all copied from lazydev.nvim.

## Install hello.nvim

1. Add the plugin to your lazy.nvim spec:

    ```lua
    {
      "yourname/hello.nvim",
      opts = {},
    }
    ```

    Passing `opts` makes lazy.nvim call `require("hello").setup()`. That call
    registers the `:Hello` command, so the plugin does nothing until it runs.

2. Restart Neovim and run `:Hello`. If a greeting appears at the cursor, the
   install worked.

To try the plugin without installing it, clone the repository and start Neovim
with the clone on the runtimepath:

```sh
nvim --clean -c 'set rtp+=/path/to/hello.nvim' -c 'lua require("hello").setup()'
```

## Configure hello.nvim

Pass a table to `setup()`. Anything you omit keeps its default:

```lua
require("hello").setup({
  -- Name substituted into the greeting
  name = "World",
  -- Greeting template. %s is replaced with name.
  greeting = "Hello, %s!",
  -- Format string passed to os.date
  date_format = "%Y-%m-%d",
  -- Lines inserted by the signature feature
  signature = {
    "--",
    "Sent from hello.nvim",
  },
  debug = false,
})
```

`date_format` accepts any [`os.date`](https://www.lua.org/manual/5.1/manual.html#pdf-os.date)
format string, which follows C `strftime`. Use `%H:%M` for a clock time or
`%A, %d %B %Y` for a long date.

You can read configuration without calling `setup()`. `lua/hello/config.lua`
uses a `setmetatable` `__index` hook that runs `setup()` on first access, so
`require("hello.config").name` returns `"World"` in a fresh session. This
pattern comes from lazydev.nvim, and it keeps initialization checks out of every
other module.

## Run hello.nvim

Each feature has a subcommand and a Lua function.

| Command | Lua | Result |
| --- | --- | --- |
| `:Hello` or `:Hello greet` | `require("hello").greet()` | Inserts `Hello, World!` |
| `:Hello date` | `require("hello").date()` | Inserts today's date, such as `2026-07-21` |
| `:Hello date %H:%M` | `require("hello").date("%H:%M")` | Inserts the current time, such as `14:32` |
| `:Hello sign` | `require("hello").sign()` | Inserts the signature lines |

Every subcommand inserts text at the cursor in the current buffer. Press `<Tab>`
after `:Hello ` to complete the subcommand names.

To bind a feature to a key:

```lua
vim.keymap.set("n", "<leader>hg", function()
  require("hello").greet()
end, { desc = "Insert greeting" })
```

## How the code is organized

```
lua/hello/
  init.lua       Public API. setup() plus one function per feature.
  config.lua     Defaults, option merging, and the :Hello command.
  cmd.lua        Subcommand dispatch and tab completion.
  util.lua       Shared helpers: insert_at_cursor, notify, warn, error, debug.
  greeting.lua   Feature module.
  date.lua       Feature module.
  signature.lua  Feature module.
doc/hello.nvim.txt   Help file. Plugin managers build doc/tags from it on install,
                     which is why doc/tags is gitignored rather than committed.
.luarc.json          Tells lua-language-server about the vim global.
```

Three conventions carry over from lazydev.nvim, and they are the reason to start
here rather than from a blank directory:

* **`init.lua` stays thin.** It exposes the public API and nothing else. Every
  function body is a single `require` call, so loading the plugin loads one small
  file and touches no feature code.
* **One user command, many subcommands.** `cmd.lua` maps subcommand names to
  functions in a table, then completes and dispatches against that table. Adding
  a feature means adding one table entry, not another `nvim_create_user_command`
  call cluttering the command namespace.
* **Feature modules stay independent.** `greeting.lua`, `date.lua`, and
  `signature.lua` each expose a pure function that returns text and a thin
  `insert()` wrapper. You test the pure function; the wrapper touches the
  buffer.

The [`.luarc.json`](https://luals.github.io/wiki/configuration/) file points
[lua-language-server](https://github.com/LuaLS/lua-language-server) at the
Neovim runtime, which silences `undefined global vim` warnings in your editor.
Open the project from its root, or the server never reads the file.

## Turn this template into your own plugin

1. **Rename the module directory.** Move `lua/hello` to `lua/yourplugin`, then
   replace `hello` in every `require` path.

2. **Rename the command.** Change `"Hello"` in `lua/yourplugin/config.lua` to
   your command name, and change the matching `parts[1]:find("Hello")` guard in
   `cmd.lua`. That guard strips the command name when Neovim passes the whole
   line to the completion function.

3. **Replace the defaults.** Rewrite the `defaults` table in `config.lua`. Keep
   the `setmetatable` block at the bottom of the file unchanged.

4. **Swap the feature modules.** Delete `greeting.lua`, `date.lua`, and
   `signature.lua`, and write your own modules in their shape. Register each one
   in the `M.commands` table in `cmd.lua` and, if it belongs in the public API,
   in `init.lua`.

5. **Rewrite the help file.** Rename `doc/hello.nvim.txt`, update its tags, and
   run `:helptags doc` to regenerate the index.

6. **Add a license.** See [Terms of use](#terms-of-use).

Verify the result without leaving your shell:

```sh
nvim --headless --clean -c 'set rtp+=.' \
  -c 'lua require("yourplugin").setup()' \
  -c 'YourCommand' \
  -c 'lua print(vim.api.nvim_get_current_line())' -c 'qa!'
```

## Troubleshoot hello.nvim

| Issue | Solution |
| --- | --- |
| `:Hello` reports `E492: Not an editor command` | `setup()` never ran, and that call registers the command. Call `require("hello").setup()`, or give lazy.nvim an `opts` table. |
| `:Hello` reports `Invalid command: <name>` | The subcommand is not in `M.commands` in `lua/hello/cmd.lua`. Press `<Tab>` after `:Hello ` to list valid names. |
| Your editor flags `Undefined global vim` | lua-language-server cannot see the Neovim runtime. Open the project from its root so the server picks up `.luarc.json`. |
| `:help hello.nvim` reports `E149: Sorry, no help for hello.nvim` | The help tags are missing. Run `:helptags doc` from the project root. |
| Text lands in the wrong place | `util.insert_at_cursor` writes at the cursor of the current window. Insert-mode mappings should use `<Cmd>Hello greet<CR>` rather than `:Hello greet<CR>`, which leaves insert mode first. |

## Contributing

This is a template rather than a shared library, so the most useful contribution
is a fork. If you find a bug in the scaffolding itself, open an issue with the
Neovim version from `:version` and the smallest `--clean` reproduction you can
manage.

Before sending a patch, format and lint:

```sh
stylua lua/
selene lua/
```

## Additional documentation

* [`doc/hello.nvim.txt`](doc/hello.nvim.txt) is the in-editor help file; read it
  with `:help hello.nvim`. Any plugin you publish needs one.
* [lazydev.nvim](https://github.com/folke/lazydev.nvim) is the plugin this
  layout draws from. Read its `lua/lazydev/` directory to see the same patterns
  carrying real features.
* [`:help write-plugin`](https://neovim.io/doc/user/usr_41.html#write-plugin) and
  [`:help lua-guide`](https://neovim.io/doc/user/lua-guide.html) are the official
  references for plugin structure and the Lua API.
* [nvim-lua/plugin-template](https://github.com/nvim-lua/plugin-template) is an
  alternative starting point: it adds CI and a test harness but ships no working
  features.

## How to get help

* Open an issue on this repository for problems with the template.
* Ask about Neovim plugin development in the
  [Neovim Discourse](https://neovim.discourse.group/) or the
  [r/neovim](https://reddit.com/r/neovim) community.
* Read `:help lua` and `:help api` in your editor before searching the web. The
  bundled documentation is current for the version you are running.

## Terms of use

This template ships without a license file, because the license should be yours.
Before publishing, add a `LICENSE` file and name it in this section. If you have
no preference, [choosealicense.com](https://choosealicense.com/) walks you
through the common options in a few minutes.
