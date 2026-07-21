---@class mdfmt.Config.mod: mdfmt.Config
local M = {}

---@class mdfmt.Config
local defaults = {
  -- Filetypes `:MdFmt` will touch. MDX is detected from the `.mdx` extension
  -- first, since Neovim ships no filetype rule for it.
  filetypes = { "markdown", "markdown.mdx", "mdx" },
  -- Column to wrap prose at. 0 leaves line breaks where the author put them.
  width = 80,
  -- Frontmatter fence, or `false` to treat a leading `---` as a thematic break.
  frontmatter = "---",
  -- Format on `:w`. Runs the binary synchronously, because an async callback
  -- cannot delay the write.
  format_on_save = false,
  -- Path to a prebuilt `md-fmt`. When nil, the one under `rust/target/release`
  -- in this plugin's own directory is used.
  bin = nil, ---@type string?
  -- Build the binary when it is missing or older than the crate source.
  auto_build = true,
  -- Cargo executable used for that build.
  cargo = "cargo",
  -- Milliseconds to wait for a synchronous format before giving up.
  timeout = 5000,
  debug = false,
}

---@type mdfmt.Config
local options

---@param opts? mdfmt.Config
function M.setup(opts)
  ---@type mdfmt.Config
  options = vim.tbl_deep_extend("force", {}, options or defaults, opts or {})

  vim.api.nvim_create_user_command("MdFmt", function(...)
    require("mdfmt.cmd").execute(...)
  end, {
    nargs = "*",
    complete = function(...)
      return require("mdfmt.cmd").complete(...)
    end,
    desc = "mdfmt.nvim",
  })

  local group = vim.api.nvim_create_augroup("mdfmt", { clear = true })
  if options.format_on_save then
    vim.api.nvim_create_autocmd("BufWritePre", {
      group = group,
      callback = function(event)
        require("mdfmt.format").format({ buf = event.buf, sync = true })
      end,
      desc = "mdfmt.nvim: format before writing",
    })
  end

  return options
end

return setmetatable(M, {
  __index = function(_, key)
    options = options or M.setup()
    return options[key]
  end,
})
