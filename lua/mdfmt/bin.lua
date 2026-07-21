--- Locating, building, and keeping up to date the `md-fmt` binary.
---
--- The plugin owns the binary rather than asking the user to install one. The
--- crate lives in `rust/` next to this file, and the compiled artifact stays
--- under `rust/target/release`, which is gitignored. All the user needs is a
--- working `cargo`.
local Config = require("mdfmt.config")
local Util = require("mdfmt.util")

local M = {}

--- Root of the plugin checkout, derived from this file's own path rather than
--- the runtimepath, so it is right no matter how the plugin was loaded.
---
--- Resolved once, at load time. A runtimepath entry can be relative (`set
--- rtp+=.` is how you try a plugin out), which makes `source` relative too, and
--- the first `:cd` would then break it.
local root = vim.fs.normalize(vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":p:h:h:h"))

---@return string
function M.root()
  return root
end

---@return string
function M.path()
  return Config.bin or (M.root() .. "/rust/target/release/md-fmt")
end

---@return boolean
function M.exists()
  return vim.uv.fs_stat(M.path()) ~= nil
end

---@param path string
---@return number
local function mtime(path)
  local stat = vim.uv.fs_stat(path)
  return stat and stat.mtime.sec or 0
end

---@type boolean?
local fresh

--- True when the crate source is newer than the binary, which is how a plugin
--- update that changes the Rust code gets picked up.
---
--- Memoized for the session. cargo does not relink a binary whose inputs did
--- not really change, so a source file that is newer but equivalent (a
--- comment, a checkout that rewrote mtimes) would otherwise look stale
--- forever and run cargo before every single format.
---
--- A user-supplied `config.bin` is never considered stale: it is not ours.
---@return boolean
function M.stale()
  if Config.bin then
    return false
  end
  if fresh ~= nil then
    return not fresh
  end

  local binary = mtime(M.path())
  if binary == 0 then
    return true
  end

  local rust = M.root() .. "/rust"
  local newest = mtime(rust .. "/Cargo.toml")
  for name, kind in vim.fs.dir(rust .. "/src") do
    if kind == "file" and name:sub(-3) == ".rs" then
      newest = math.max(newest, mtime(rust .. "/src/" .. name))
    end
  end

  fresh = newest <= binary
  return not fresh
end

---@type fun(ok: boolean)[]?
local pending

--- Compile the binary. Callbacks queue behind a build already in flight rather
--- than starting a second cargo over the same target directory.
---@param cb? fun(ok: boolean)
function M.build(cb)
  if pending then
    table.insert(pending, cb or function() end)
    return
  end
  pending = { cb or function() end }

  if vim.fn.executable(Config.cargo) == 0 then
    Util.error({
      ("`%s` is not executable."):format(Config.cargo),
      "md-fmt-rs.nvim compiles its own formatter and needs a Rust toolchain.",
    })
    return M.finish(false)
  end

  Util.notify("building md-fmt...")
  vim.system({ Config.cargo, "build", "--release" }, {
    cwd = M.root() .. "/rust",
    text = true,
  }, function(result)
    vim.schedule(function()
      if result.code == 0 then
        Util.notify("built md-fmt")
      else
        Util.error({ "cargo build failed:", vim.trim(result.stderr or "") })
      end
      M.finish(result.code == 0)
    end)
  end)
end

---@param ok boolean
---@private
function M.finish(ok)
  local waiting = pending or {}
  pending = nil
  if ok then
    fresh = true
    -- cargo leaves the binary's mtime alone when it decides nothing needed
    -- rebuilding, so a source file that is newer but equivalent would look
    -- stale in every future session too. Stamping it here records what the
    -- build just established: this binary is current.
    local now = os.time()
    pcall(vim.uv.fs_utime, M.path(), now, now)
  end
  for _, cb in ipairs(waiting) do
    cb(ok)
  end
end

--- Hand a usable binary to `cb`, building one first if that is needed and
--- allowed.
---@param cb fun(ok: boolean)
function M.ensure(cb)
  if M.exists() and not M.stale() then
    return cb(true)
  end

  if not Config.auto_build then
    Util.error({
      "md-fmt is missing or out of date, and auto_build is off. Build it with:",
      ("  cargo build --release --manifest-path %s/rust/Cargo.toml"):format(M.root()),
    })
    return cb(false)
  end

  M.build(cb)
end

--- Human-readable state of the binary, for `:MdFmt status`.
---@return string[]
function M.status()
  local path = M.path()
  local lines = {
    "binary: " .. path,
    "exists: " .. tostring(M.exists()),
    "stale:  " .. tostring(M.stale()),
  }

  if M.exists() then
    local result = vim.system({ path, "--version" }, { text = true }):wait(Config.timeout)
    lines[#lines + 1] = "version: " .. vim.trim(result.stdout or result.stderr or "?")
  end

  return lines
end

return M
