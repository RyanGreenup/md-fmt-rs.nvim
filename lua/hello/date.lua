local Config = require("hello.config")
local Util = require("hello.util")

local M = {}

---@param format? string strftime format, defaults to `config.date_format`
---@return string
function M.text(format)
  return os.date(format or Config.date_format) --[[@as string]]
end

--- Insert the current date at the cursor.
---@param format? string
function M.insert(format)
  Util.insert_at_cursor(M.text(format))
end

return M
