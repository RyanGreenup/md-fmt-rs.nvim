local M = {}

---@param opts? mdfmt.Config
function M.setup(opts)
  require("mdfmt.config").setup(opts)
end

--- Format the current buffer, or `opts.buf`.
---@param opts? {buf?: integer, sync?: boolean}
function M.format(opts)
  require("mdfmt.format").format(opts)
end

--- Compile the `md-fmt` binary.
---@param cb? fun(ok: boolean)
function M.build(cb)
  require("mdfmt.bin").build(cb)
end

--- Describe the state of the binary.
---@return string[]
function M.status()
  return require("mdfmt.bin").status()
end

return M
