local M = {}

---@param opts? hello.Config
function M.setup(opts)
  require("hello.config").setup(opts)
end

--- Insert a greeting at the cursor.
function M.greet()
  require("hello.greeting").insert()
end

--- Insert the current date at the cursor.
function M.date()
  require("hello.date").insert()
end

--- Insert a signature block at the cursor.
function M.sign()
  require("hello.signature").insert()
end

return M
