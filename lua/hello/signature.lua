local Config = require("hello.config")
local Util = require("hello.util")

local M = {}

---@return string[]
function M.lines()
  return vim.deepcopy(Config.signature)
end

--- Insert the signature block at the cursor.
function M.insert()
  Util.insert_at_cursor(M.lines())
end

return M
