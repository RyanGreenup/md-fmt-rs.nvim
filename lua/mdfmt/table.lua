--- Keeping the Markdown table under the cursor aligned while it is typed.
---
--- No Markdown is read here. The binary is handed the whole buffer and the
--- cursor position and answers with a line range, its replacement, and where
--- the cursor ended up; comrak, on the other side of that call, is what
--- decides where a table starts, where its cells are, and how its columns are
--- aligned. Neovim's remaining job is the part only it can do: knowing where
--- the cursor is, and writing the answer back without disturbing marks or
--- collapsing the undo history.
---
--- The call is synchronous. It runs on a typing pause rather than on every
--- keystroke, the binary reads one buffer and writes a few lines, and doing it
--- inline means the buffer cannot move underneath the result.

local Bin = require("mdfmt.bin")
local Config = require("mdfmt.config")
local Cursor = require("mdfmt.cursor")
local Format = require("mdfmt.format")
local Util = require("mdfmt.util")

local M = {}

local enabled = true
local pending = {}
local applying = {}
local generation = 0

--- Milliseconds of quiet typing before the table is realigned.
local DEBOUNCE = 200

---@class mdfmt.TableEdit
---@field first integer 1-based first buffer line to replace
---@field last integer 1-based last buffer line to replace
---@field lines string[] replacement for `first..last`
---@field row integer 1-based cursor line
---@field col integer 0-based byte cursor column

--- Read the binary's answer: a header line of four numbers, then one
--- replacement line per buffer line it covers.
---@param stdout string
---@return mdfmt.TableEdit?
local function parse(stdout)
  local out = vim.split(stdout or "", "\n", { plain = true })
  if out[#out] == "" then
    table.remove(out)
  end
  local header = table.remove(out, 1)
  if not header then
    return nil
  end

  local first, last, row, col = header:match("^(%d+) (%d+) (%d+) (%d+)$")
  if not first then
    return Util.debug("md-fmt --table wrote something unexpected: " .. header)
  end

  first, last, row, col = tonumber(first), tonumber(last), tonumber(row), tonumber(col)
  if #out ~= last - first + 1 then
    return Util.debug("md-fmt --table returned the wrong number of lines")
  end
  return { first = first, last = last, lines = out, row = row, col = col }
end

--- Realign the table under the cursor of `win`, if there is one.
---@param buf? integer
---@param win? integer
---@return boolean changed
function M.format(buf, win)
  buf = buf or vim.api.nvim_get_current_buf()
  win = win or vim.api.nvim_get_current_win()
  if not enabled or applying[buf] or not vim.api.nvim_buf_is_valid(buf) or not vim.bo[buf].modifiable then
    return false
  end
  if not Format.supported(buf) or not vim.api.nvim_win_is_valid(win) or vim.api.nvim_win_get_buf(win) ~= buf then
    return false
  end
  -- Typing is not the moment to start a cargo build. The first `:MdFmt`
  -- builds the binary, and until it exists there is nothing to align with.
  if not Bin.exists() then
    return false
  end

  local cursor = vim.api.nvim_win_get_cursor(win)
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local args = Format.args(buf)
  vim.list_extend(args, { "--table", "--row", tostring(cursor[1]), "--col", tostring(cursor[2]) })

  local result = vim.system(args, { stdin = table.concat(lines, "\n") .. "\n", text = true }):wait(Config.timeout)
  -- A non-zero exit is how the binary says there is no table under the
  -- cursor, which is the common case and not worth a message.
  if result.code ~= 0 then
    return false
  end

  local edit = parse(result.stdout)
  if not edit then
    return false
  end

  local replacement = vim.deepcopy(lines)
  for index, text in ipairs(edit.lines) do
    replacement[edit.first + index - 1] = text
  end

  applying[buf] = true
  -- The realignment belongs to the keystroke that provoked it, not to an undo
  -- step of its own.
  vim.api.nvim_buf_call(buf, function()
    pcall(vim.cmd, "silent! undojoin")
  end)
  local changed = Cursor.apply(buf, replacement)
  applying[buf] = nil

  if changed and vim.api.nvim_win_is_valid(win) and vim.api.nvim_win_get_buf(win) == buf then
    local row = math.min(edit.row, vim.api.nvim_buf_line_count(buf))
    local line = vim.api.nvim_buf_get_lines(buf, row - 1, row, false)[1] or ""
    vim.api.nvim_win_set_cursor(win, { row, math.min(edit.col, #line) })
  end
  return changed
end

---@param buf integer
---@param token integer
local function run(buf, token)
  if not enabled or pending[buf] ~= token or not vim.api.nvim_buf_is_valid(buf) then
    return
  end
  pending[buf] = nil
  local win = vim.fn.bufwinid(buf)
  if win ~= -1 then
    M.format(buf, win)
  end
end

--- Ask for a realignment once typing has paused, or on the next tick when
--- `immediate`. A later call supersedes an earlier one.
---@param buf integer
---@param immediate? boolean
function M.schedule(buf, immediate)
  if not enabled or applying[buf] then
    return
  end
  generation = generation + 1
  local token = generation
  pending[buf] = token
  if immediate then
    vim.schedule(function()
      run(buf, token)
    end)
  else
    vim.defer_fn(function()
      run(buf, token)
    end, DEBOUNCE)
  end
end

---@param value boolean
function M.configure(value)
  enabled = value ~= false
  generation = generation + 1
  pending = {}
end

---@return boolean
function M.toggle()
  M.configure(not enabled)
  return enabled
end

---@return boolean
function M.enabled()
  return enabled
end

return M
