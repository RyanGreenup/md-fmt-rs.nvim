--- Tests for the diff application and cursor mapping.
---
--- Run with:
---     nvim --headless --clean -c 'set rtp+=.' -l tests/cursor_spec.lua

local Cursor = require("mdfmt.cursor")

local failures = 0

---@param name string
---@param got any
---@param want any
local function check(name, got, want)
  if vim.deep_equal(got, want) then
    print("ok   " .. name)
  else
    failures = failures + 1
    print(("FAIL %s\n  got  %s\n  want %s"):format(name, vim.inspect(got), vim.inspect(want)))
  end
end

--- Put `lines` in a scratch buffer with the cursor at (row, col), apply
--- `formatted`, and report where the cursor ended up.
---@return integer[] cursor, string[] lines
local function run(lines, row, col, formatted)
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.api.nvim_win_set_buf(0, buf)
  vim.api.nvim_win_set_cursor(0, { row, col })

  Cursor.apply(buf, formatted)

  return vim.api.nvim_win_get_cursor(0), vim.api.nvim_buf_get_lines(buf, 0, -1, false)
end

-- An untouched line keeps the cursor exactly, even when lines above it move.
do
  local cursor, lines = run(
    { "# Title", "", "para", "", "tail" },
    5,
    2,
    { "# Title", "", "para", "", "extra", "", "tail" }
  )
  check("lines added above", cursor, { 7, 2 })
  check("lines added above (text)", lines[7], "tail")
end

do
  local cursor = run({ "a", "b", "c", "keep" }, 4, 1, { "a", "keep" })
  check("lines removed above", cursor, { 2, 1 })
end

-- The word under the cursor moves to another line during a rewrap.
do
  local before = { "alpha bravo charlie delta echo foxtrot golf hotel india juliet" }
  local after = { "alpha bravo charlie delta echo", "foxtrot golf hotel india juliet" }
  -- Cursor on the "h" of "hotel", which is at byte 43 of the original line.
  local col = assert(before[1]:find("hotel")) - 1
  local cursor = run(before, 1, col, after)
  check("word followed across a rewrap", cursor, { 2, assert(after[2]:find("hotel")) - 1 })
end

-- The same word appears twice; the cursor keeps the occurrence it was on.
do
  local before = { "one two one two one two one two one two one two one two" }
  local after = { "one two one two one two", "one two one two one two one two" }
  -- The fourth "one", at byte 24.
  local col = 24
  check("fixture sanity", before[1]:sub(col + 1, col + 3), "one")
  local cursor = run(before, 1, col, after)
  -- Third occurrence of "one" on the second output line.
  check("repeated word keeps its occurrence", cursor, { 2, 0 })
end

-- Mid-word columns are preserved.
do
  local before = { "the quick brown fox jumps" }
  local after = { "the quick", "brown fox jumps" }
  local cursor = run(before, 1, 12, after) -- the "o" of "brown"
  check("offset within the word", cursor, { 2, 2 })
end

-- A cursor on whitespace inside a changed hunk still lands somewhere sane.
do
  local cursor = run({ "  ", "text" }, 1, 1, { "", "text" })
  check("whitespace-only line", cursor, { 1, 0 })
end

-- Nothing to do.
do
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "same" })
  check("identical input is a no-op", Cursor.apply(buf, { "same" }), false)
end

-- Marks outside the changed region survive, which is the point of the
-- minimal diff.
do
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "one", "two", "three", "four" })
  vim.api.nvim_win_set_buf(0, buf)
  local ns = vim.api.nvim_create_namespace("mdfmt_spec")
  local mark = vim.api.nvim_buf_set_extmark(buf, ns, 3, 0, {})
  Cursor.apply(buf, { "one", "TWO", "three", "four" })
  check("extmark below the hunk", vim.api.nvim_buf_get_extmark_by_id(buf, ns, mark, {}), { 3, 0 })
end

if failures > 0 then
  print(("\n%d failure(s)"):format(failures))
  vim.cmd("cquit 1")
end
print("\nall cursor tests passed")
