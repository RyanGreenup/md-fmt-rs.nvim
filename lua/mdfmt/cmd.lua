local Util = require("mdfmt.util")

local M = {}

---@type table<string, fun(args: string[])>
M.commands = {
  format = function()
    require("mdfmt.format").format()
  end,
  build = function()
    require("mdfmt.bin").build()
  end,
  status = function()
    Util.notify(require("mdfmt.bin").status())
  end,
}

function M.execute(input)
  local prefix, args = M.parse(input.args)
  prefix = prefix and prefix ~= "" and prefix or "format"
  if not M.commands[prefix or ""] then
    return Util.error("Invalid command: " .. prefix)
  end
  M.commands[prefix](args)
end

function M.complete(_, line)
  local prefix, args = M.parse(line)
  if #args > 0 then
    return {}
  end

  ---@param key string
  return vim.tbl_filter(function(key)
    return key:find(prefix, 1, true) == 1
  end, vim.tbl_keys(M.commands))
end

---@return string, string[]
function M.parse(args)
  local parts = vim.split(vim.trim(args), "%s+")
  if parts[1]:find("MdFmt") then
    table.remove(parts, 1)
  end
  if args:sub(-1) == " " then
    parts[#parts + 1] = ""
  end
  return table.remove(parts, 1) or "", parts
end

return M
