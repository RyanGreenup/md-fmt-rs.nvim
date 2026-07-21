--- conform.nvim adapter.
---
--- Runs the same binary, over the same stdin, with the same flags as
--- `mdfmt.format`, so a host config only has to point conform at this module
--- rather than re-deriving the args in its own conform.lua.
local Bin = require("mdfmt.bin")
local Config = require("mdfmt.config")
local Format = require("mdfmt.format")

return {
  command = function()
    return Bin.path()
  end,
  args = function(_, ctx)
    local out = { "--width", tostring(Config.width) }
    if Format.is_mdx(ctx.buf) then
      table.insert(out, "--mdx")
    end
    if Config.frontmatter then
      vim.list_extend(out, { "--frontmatter", Config.frontmatter })
    else
      table.insert(out, "--no-frontmatter")
    end
    return out
  end,
  stdin = true,
  condition = function()
    return Bin.exists()
  end,
}
