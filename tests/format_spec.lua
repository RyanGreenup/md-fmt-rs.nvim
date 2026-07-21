--- End-to-end tests: real buffers through the real binary.
---
--- Run with:
---     nvim --headless --clean -c 'set rtp+=.' -l tests/format_spec.lua

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

---@param name string
---@param cond boolean
local function ok(name, cond)
  check(name, not not cond, true)
end

require("mdfmt").setup({ width = 80 })

local Bin = require("mdfmt.bin")
if not Bin.exists() then
  print("md-fmt is not built; run `cargo build --release --manifest-path rust/Cargo.toml`")
  vim.cmd("cquit 1")
end

--- Load `path` into the current window and put the cursor on the first
--- occurrence of `needle`.
---@param path string
---@param needle? string
local function open(path, needle)
  vim.cmd.edit({ args = { path }, bang = true })
  if needle then
    local found = vim.fn.searchpos(needle, "w")
    vim.api.nvim_win_set_cursor(0, { found[1], found[2] - 1 })
  end
  return vim.api.nvim_get_current_buf()
end

-- MDX round trip through the whole stack.
do
  local buf = open("rust/tests/fixtures/kitchen-sink.mdx", "formatter")
  ok("mdx detected", require("mdfmt.format").is_mdx(buf))

  require("mdfmt").format({ sync = true })
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)

  check("setext heading became atx", lines[8], "# Setext title")
  ok("frontmatter survived", lines[1] == "---" and lines[2]:match("^title:") ~= nil)
  ok("blob frozen", vim.tbl_contains(lines, "Some **markdown**    frozen inside the blob."))

  -- The cursor was on "formatter" inside the paragraph that gets rewrapped.
  local cursor = vim.api.nvim_win_get_cursor(0)
  local line = lines[cursor[1]]
  check("cursor still on its word", line:sub(cursor[2] + 1, cursor[2] + 9), "formatter")

  ok("modified", vim.bo[buf].modified)
  vim.cmd("edit!")
end

-- Plain Markdown takes the non-MDX path.
do
  local buf = open("rust/tests/fixtures/plain.md")
  ok("md is not mdx", not require("mdfmt.format").is_mdx(buf))

  require("mdfmt").format({ sync = true })
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  ok("html comment kept", vim.tbl_contains(lines, "<!-- an ordinary HTML comment -->"))
  ok("no whitespace-only lines", not vim.iter(lines):any(function(l)
    return l ~= "" and vim.trim(l) == ""
  end))
  vim.cmd("edit!")
end

-- Formatting an already-formatted buffer must be a no-op, or every save
-- becomes a diff.
do
  local buf = open("rust/tests/fixtures/kitchen-sink.mdx")
  require("mdfmt").format({ sync = true })
  local once = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local tick = vim.api.nvim_buf_get_changedtick(buf)

  require("mdfmt").format({ sync = true })
  check("second pass changed nothing", vim.api.nvim_buf_get_lines(buf, 0, -1, false), once)
  check("second pass did not touch the buffer", vim.api.nvim_buf_get_changedtick(buf), tick)
  vim.cmd("edit!")
end

-- Broken MDX leaves the buffer alone.
do
  local buf = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_buf_set_name(buf, "/tmp/mdfmt-broken.mdx")
  vim.api.nvim_win_set_buf(0, buf)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "<Callout>", "never closed" })

  local before = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local notified = {}
  local notify = vim.notify
  vim.notify = function(msg)
    notified[#notified + 1] = msg
  end
  require("mdfmt").format({ sync = true })
  vim.notify = notify

  check("broken mdx left the buffer alone", vim.api.nvim_buf_get_lines(buf, 0, -1, false), before)
  ok("broken mdx reported the parse error", (notified[1] or ""):match("could not parse MDX") ~= nil)
end

-- A buffer that is neither Markdown nor MDX is refused.
do
  local buf = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_win_set_buf(0, buf)
  vim.bo[buf].filetype = "lua"
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "local x = 1" })

  local notify = vim.notify
  local notified
  vim.notify = function(msg)
    notified = msg
  end
  require("mdfmt").format({ sync = true })
  vim.notify = notify

  ok("lua buffer refused", (notified or ""):match("^not a Markdown") ~= nil)
end

if failures > 0 then
  print(("\n%d failure(s)"):format(failures))
  vim.cmd("cquit 1")
end
print("\nall format tests passed")
