local M = {}

--- Insert lines at the cursor position in the current buffer.
---@param lines string|string[]
function M.insert_at_cursor(lines)
  lines = type(lines) == "string" and { lines } or lines
  local cursor = vim.api.nvim_win_get_cursor(0)
  local row, col = cursor[1], cursor[2]
  vim.api.nvim_buf_set_text(0, row - 1, col, row - 1, col, lines --[[@as string[] ]])
end

---@param msg string|string[]
---@param opts? {level?: integer}
function M.notify(msg, opts)
  opts = opts or {}
  msg = type(msg) == "table" and table.concat(msg, "\n") or msg
  vim.notify(msg --[[@as string]], opts.level or vim.log.levels.INFO, { title = "hello.nvim" })
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
  if require("hello.config").debug then
    M.notify(msg, { level = vim.log.levels.DEBUG })
  end
end

return M
