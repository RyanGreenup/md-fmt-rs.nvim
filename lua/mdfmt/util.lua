local M = {}

---@param msg string|string[]
---@param opts? {level?: integer}
function M.notify(msg, opts)
  opts = opts or {}
  msg = type(msg) == "table" and table.concat(msg, "\n") or msg
  vim.notify(msg --[[@as string]], opts.level or vim.log.levels.INFO, { title = "md-fmt-rs.nvim" })
end

---@param msg string|string[]
function M.warn(msg)
  M.notify(msg, { level = vim.log.levels.WARN })
end

---@param msg string|string[]
function M.error(msg)
  M.notify(msg, { level = vim.log.levels.ERROR })
end

---@param msg string|string[]
function M.debug(msg)
  if require("mdfmt.config").debug then
    M.notify(msg, { level = vim.log.levels.DEBUG })
  end
end

return M
