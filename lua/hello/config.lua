---@class hello.Config.mod: hello.Config
local M = {}

---@class hello.Config
local defaults = {
  -- Name used by the greeting feature
  name = "World",
  -- Template for the greeting. `%s` is replaced with `name`.
  greeting = "Hello, %s!",
  -- strftime format used by the date feature
  date_format = "%Y-%m-%d",
  -- Lines inserted by the signature feature
  signature = {
    "--",
    "Sent from hello.nvim",
  },
  debug = false,
}

---@type hello.Config
local options

---@param opts? hello.Config
function M.setup(opts)
  ---@type hello.Config
  options = vim.tbl_deep_extend("force", {}, options or defaults, opts or {})

  vim.api.nvim_create_user_command("Hello", function(...)
    require("hello.cmd").execute(...)
  end, {
    nargs = "*",
    complete = function(...)
      return require("hello.cmd").complete(...)
    end,
    desc = "hello.nvim",
  })

  return options
end

return setmetatable(M, {
  __index = function(_, key)
    options = options or M.setup()
    return options[key]
  end,
})
