--- Tests for automatic, table-only formatting.
---
--- The alignment itself is the binary's business, and `rust/tests/table.rs`
--- covers it there. What is left for these is the wiring: that the buffer and
--- the cursor reach the binary, that its answer lands on the right lines, and
--- that nothing happens when the feature is off.

local failures = 0

local function check(name, got, want)
  if vim.deep_equal(got, want) then
    print("ok   " .. name)
  else
    failures = failures + 1
    print(("FAIL %s\n  got  %s\n  want %s"):format(name, vim.inspect(got), vim.inspect(want)))
  end
end

local function ok(name, value)
  check(name, not not value, true)
end

require("mdfmt").setup({ table_auto_format = true })
local Table = require("mdfmt.table")

local function buffer(content, row, col)
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, content)
  vim.bo[buf].filetype = "markdown"
  vim.api.nvim_win_set_buf(0, buf)
  vim.api.nvim_win_set_cursor(0, { row or 1, col or 0 })
  return buf
end

local function lines(buf)
  return vim.api.nvim_buf_get_lines(buf, 0, -1, false)
end

do
  local buf = buffer({ "before", "", "| Name|Age |", "|:--|--:|", "|Ryan|42|", "", "after" }, 5, 6)
  ok("formats active table", Table.format(buf, 0))
  check("only active table changed", lines(buf), {
    "before",
    "",
    "| Name | Age |",
    "| :--- | --: |",
    "| Ryan | 42  |",
    "",
    "after",
  })
  local cursor = vim.api.nvim_win_get_cursor(0)
  check("cursor stays in its cell", lines(buf)[cursor[1]]:sub(cursor[2] + 1, cursor[2] + 2), "42")
  check("formatted table is idempotent", Table.format(buf, 0), false)
end

do
  -- A `- | -` delimiter would be a bullet list rather than a table: without
  -- the outer pipes the dashes have to number at least three.
  local buf = buffer({ "A | B", "--- | ---", "one | two", "four" }, 3, 6)
  Table.format(buf, 0)
  check("pads rows that are missing cells", lines(buf), {
    "| A    | B   |",
    "| ---- | --- |",
    "| one  | two |",
    "| four |     |",
  })
end

do
  -- GFM ignores a cell the header did not declare, so there is no way to
  -- render this table that keeps `three`. It is left alone instead.
  local original = { "| A | B |", "| - | - |", "| one | two | three |" }
  local buf = buffer(original, 3, 3)
  check("a row with a surplus cell is left alone", Table.format(buf, 0), false)
  check("the surplus cell survives", lines(buf), original)
end

do
  local buf =
    buffer({ "| Key | Value |", "| --- | --- |", "| emoji | 🦀 |", "| pipe | a\\|b |", "| code | `a\\|b` |" }, 3, 10)
  Table.format(buf, 0)
  check("handles display width and literal pipes", lines(buf), {
    "| Key   | Value  |",
    "| ----- | ------ |",
    "| emoji | 🦀     |",
    "| pipe  | a\\|b   |",
    "| code  | `a\\|b` |",
  })
end

do
  local original = { "```", "| not | a table |", "| --- | --- |", "| x | y |", "```" }
  local buf = buffer(original, 4, 3)
  check("table-like code fence is ignored", Table.format(buf, 0), false)
  check("code fence remains unchanged", lines(buf), original)
end

do
  local original = { "| header | only |", "| body | without delimiter |" }
  local buf = buffer(original, 1, 3)
  check("invalid table is ignored", Table.format(buf, 0), false)
  check("invalid table remains unchanged", lines(buf), original)
end

do
  local buf = buffer({ "| A|B|", "|-|-|", "|x|y|" }, 3, 3)
  Table.schedule(buf)
  ok(
    "debounced formatting completes",
    vim.wait(600, function()
      return lines(buf)[1] == "| A   | B   |"
    end, 10)
  )
end

do
  check("automatic formatting defaults on", Table.enabled(), true)
  vim.cmd("MdFmt table-toggle")
  check("command disables formatting globally", Table.enabled(), false)
  local other = buffer({ "| A|B|", "|-|-|" }, 1, 2)
  check("disabled formatter is a no-op", Table.format(other, 0), false)
  vim.cmd("MdFmt table-toggle")
  check("command enables formatting globally", Table.enabled(), true)
  ok("toggle command completes", vim.tbl_contains(require("mdfmt.cmd").complete("", "MdFmt table-"), "table-toggle"))
end

do
  require("mdfmt").setup({ table_auto_format = false })
  check("setup can disable automatic formatting", Table.enabled(), false)
  require("mdfmt").setup({ table_auto_format = true })
  check("setup can re-enable automatic formatting", Table.enabled(), true)
  check("typing autocmd is registered", #vim.api.nvim_get_autocmds({ group = "mdfmt", event = "TextChangedI" }), 1)
  check("insert-leave autocmd is registered", #vim.api.nvim_get_autocmds({ group = "mdfmt", event = "InsertLeave" }), 1)
end

if failures > 0 then
  print(("\n%d failure(s)"):format(failures))
  vim.cmd("cquit 1")
end
print("\nall table tests passed")
