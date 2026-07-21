--- Running the binary over a buffer.
local Bin = require("mdfmt.bin")
local Config = require("mdfmt.config")
local Cursor = require("mdfmt.cursor")
local Util = require("mdfmt.util")

local M = {}

--- Whether this buffer is MDX.
---
--- The file extension is asked first: Neovim ships no filetype rule for
--- `.mdx`, so plenty of people are editing MDX in a buffer that calls itself
--- `markdown`.
---@param buf integer
---@return boolean
function M.is_mdx(buf)
  if vim.fn.fnamemodify(vim.api.nvim_buf_get_name(buf), ":e") == "mdx" then
    return true
  end
  local ft = vim.bo[buf].filetype
  return ft == "mdx" or ft == "markdown.mdx"
end

---@param buf integer
---@return boolean
function M.supported(buf)
  return vim.tbl_contains(Config.filetypes, vim.bo[buf].filetype) or M.is_mdx(buf)
end

--- The command that formats this buffer, up to the subcommand. Table
--- realignment appends its own flags, so that both ways of running the binary
--- read the buffer with the same settings.
---@param buf integer
---@return string[]
function M.args(buf)
  local out = { Bin.path(), "--width", tostring(Config.width) }
  if M.is_mdx(buf) then
    table.insert(out, "--mdx")
  end
  if Config.frontmatter then
    vim.list_extend(out, { "--frontmatter", Config.frontmatter })
  else
    table.insert(out, "--no-frontmatter")
  end
  return out
end

---@param stdout string
---@return string[]
local function to_lines(stdout)
  local lines = vim.split(stdout, "\n", { plain = true })
  -- The document ends with a newline, which `split` turns into an empty final
  -- element that is not a line of the buffer.
  if lines[#lines] == "" then
    table.remove(lines)
  end
  return lines
end

--- Take the binary's result and put it in the buffer, unless the user has
--- typed something since it was asked for.
---@param buf integer
---@param tick integer
---@param result vim.SystemCompleted
local function finish(buf, tick, result)
  if not vim.api.nvim_buf_is_valid(buf) then
    return
  end
  if vim.api.nvim_buf_get_changedtick(buf) ~= tick then
    return Util.debug("buffer changed while formatting; discarding the result")
  end
  if result.code ~= 0 then
    return Util.error(vim.trim(result.stderr or "md-fmt failed"))
  end

  if Cursor.apply(buf, to_lines(result.stdout or "")) then
    Util.debug("formatted")
  end
end

--- Format a buffer.
---@param opts? {buf?: integer, sync?: boolean}
function M.format(opts)
  opts = opts or {}
  local buf = opts.buf or vim.api.nvim_get_current_buf()

  if not vim.bo[buf].modifiable then
    return Util.warn("buffer is not modifiable")
  end
  if not M.supported(buf) then
    return Util.warn("not a Markdown or MDX buffer: " .. vim.bo[buf].filetype)
  end

  local tick = vim.api.nvim_buf_get_changedtick(buf)
  local text = table.concat(vim.api.nvim_buf_get_lines(buf, 0, -1, false), "\n") .. "\n"

  -- `:w` cannot wait on a callback, so format-on-save blocks instead.
  if opts.sync then
    if not Bin.exists() then
      return Util.error("md-fmt is not built yet; run :MdFmt build")
    end
    return finish(buf, tick, vim.system(M.args(buf), { stdin = text, text = true }):wait(Config.timeout))
  end

  Bin.ensure(function(ok)
    if not ok then
      return
    end
    vim.system(M.args(buf), { stdin = text, text = true }, function(result)
      vim.schedule(function()
        finish(buf, tick, result)
      end)
    end)
  end)
end

return M
