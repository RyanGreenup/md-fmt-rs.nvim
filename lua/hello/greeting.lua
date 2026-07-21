local Config = require("hello.config")
local Util = require("hello.util")

local M = {}

---@return string
function M.text()
  return Config.greeting:format(Config.name)
end

--- Insert the greeting at the cursor.
function M.insert()
  Util.insert_at_cursor(M.text())
end

return M
