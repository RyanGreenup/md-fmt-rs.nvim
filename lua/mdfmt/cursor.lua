--- Applying the formatter's output to a buffer without losing the user's
--- place.
---
--- Two things matter here. Replacing the whole buffer in one call would reset
--- every mark, fold, and extmark in it and collapse the undo history into a
--- single opaque step, so the new text goes in as a minimal diff instead: only
--- the lines that actually changed are written.
---
--- That alone keeps the cursor put whenever its line survives untouched, which
--- with a minimal diff is the common case. When the cursor's own line is
--- rewritten (prose rewrapping at 80 columns rewrites a lot of lines) the
--- cursor is carried by the word it was sitting on: find that word again in
--- the replacement text and follow it wherever it went.

local M = {}

--- Characters that make up an anchor word. Deliberately wider than `%w` so
--- that `kebab-case`, `snake_case`, and `file.ext` each stay one anchor.
local WORD = "[%w_%-%.]"

local diff = vim.text.diff or vim.diff

---@class mdfmt.Hunk
---@field first integer 1-based first old line, may exceed `last` for an insert
---@field last integer 1-based last old line
---@field bfirst integer 1-based first new line
---@field blast integer 1-based last new line

--- Normalize a `vim.diff` index tuple.
---
--- A zero count means the hunk is a pure insertion or deletion, and the start
--- index then names the line *before* the gap rather than a line in it, which
--- is why these cannot be used as-is.
---@param hunk integer[]
---@return mdfmt.Hunk
local function normalize(hunk)
  local start_a, count_a, start_b, count_b = hunk[1], hunk[2], hunk[3], hunk[4]
  return {
    first = count_a == 0 and start_a + 1 or start_a,
    last = count_a == 0 and start_a or start_a + count_a - 1,
    bfirst = count_b == 0 and start_b + 1 or start_b,
    blast = count_b == 0 and start_b or start_b + count_b - 1,
  }
end

---@param char string
---@return boolean
local function is_word(char)
  return char ~= "" and char:match(WORD) ~= nil
end

--- The word under the cursor, or failing that the nearest one to its left,
--- or failing that the nearest one to its right.
---@param line string
---@param col integer 0-based byte column
---@return string? word, integer start, integer offset
local function word_at(line, col)
  local cursor = col + 1
  local best, best_start = nil, 0
  local from = 1

  while true do
    local start, stop = line:find(WORD .. "+", from)
    if not start then
      break
    end

    if start <= cursor and cursor <= stop then
      return line:sub(start, stop), start, cursor - start
    end
    if stop < cursor then
      -- Nearest to the left so far; keep looking in case one contains us.
      best, best_start = line:sub(start, stop), start
    elseif best == nil then
      -- Nothing to the left, so take the first one to the right.
      return line:sub(start, stop), start, 0
    end

    from = stop + 1
  end

  if best then
    return best, best_start, #best
  end
  return nil, 0, 0
end

--- Byte offsets at which `word` appears in `text` as a whole word.
---@param text string
---@param word string
---@return integer[]
local function occurrences(text, word)
  local found, from = {}, 1
  while true do
    local start, stop = text:find(word, from, true)
    if not start then
      return found
    end
    if not is_word(text:sub(start - 1, start - 1)) and not is_word(text:sub(stop + 1, stop + 1)) then
      found[#found + 1] = start
    end
    from = start + 1
  end
end

--- Turn a 1-based byte offset into `text` into a (line, column) pair relative
--- to the first line of `text`.
---@param text string
---@param offset integer
---@return integer line, integer col
local function position_of(text, offset)
  local prefix = text:sub(1, offset - 1)
  local _, breaks = prefix:gsub("\n", "")
  local line_start = prefix:match(".*()\n") or 0
  return breaks, offset - line_start - 1
end

--- Where the cursor should land when its own line was rewritten.
---@param hunk mdfmt.Hunk
---@param old_lines string[]
---@param new_lines string[]
---@param row integer
---@param col integer
---@return integer row, integer col
local function follow(hunk, old_lines, new_lines, row, col)
  local before = table.concat(vim.list_slice(old_lines, hunk.first, row - 1), "\n")
  local after = table.concat(vim.list_slice(new_lines, hunk.bfirst, hunk.blast), "\n")

  local word, start, offset = word_at(old_lines[row] or "", col)
  if word then
    -- How many of this word the hunk already held above the cursor. Counting
    -- across the whole hunk rather than the single line is what survives a
    -- rewrap, which moves words between lines freely.
    local prior = #occurrences(before, word) + #occurrences((old_lines[row] or ""):sub(1, start - 1), word)

    local matches = occurrences(after, word)
    local at = matches[prior + 1] or matches[#matches]
    if at then
      local line, column = position_of(after, at)
      return hunk.bfirst + line, column + offset
    end
  end

  -- The text genuinely changed, or the cursor sat in whitespace. Land at the
  -- proportionally equivalent line in the replacement.
  local old_count = math.max(hunk.last - hunk.first + 1, 1)
  local new_count = math.max(hunk.blast - hunk.bfirst + 1, 1)
  local through = (row - hunk.first) / old_count
  return hunk.bfirst + math.floor(through * new_count), col
end

--- Where the cursor at (row, col) ends up once the hunks are applied.
---@param hunks mdfmt.Hunk[]
---@param old_lines string[]
---@param new_lines string[]
---@param row integer 1-based
---@param col integer 0-based byte column
---@return integer row, integer col
function M.map(hunks, old_lines, new_lines, row, col)
  local delta = 0

  for _, hunk in ipairs(hunks) do
    if row < hunk.first then
      break
    end
    if row > hunk.last then
      -- Both counts are `stop - start + 1`, and the +1s cancel. An insert has
      -- a count of zero on the old side, a delete on the new side, and this
      -- still holds for both.
      delta = delta + (hunk.blast - hunk.bfirst) - (hunk.last - hunk.first)
    else
      return follow(hunk, old_lines, new_lines, row, col)
    end
  end

  return row + delta, col
end

--- Replace `buf`'s contents with `new_lines`, writing only what changed, and
--- carry the cursor of every window showing the buffer along with it.
---@param buf integer
---@param new_lines string[]
---@return boolean changed
function M.apply(buf, new_lines)
  local old_lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local old_text = table.concat(old_lines, "\n") .. "\n"
  local new_text = table.concat(new_lines, "\n") .. "\n"
  if old_text == new_text then
    return false
  end

  local hunks = vim.tbl_map(normalize, diff(old_text, new_text, { result_type = "indices" }))

  -- Where each window's cursor is headed, worked out before the buffer moves
  -- under it.
  local windows = {}
  for _, win in ipairs(vim.fn.win_findbuf(buf)) do
    local cursor = vim.api.nvim_win_get_cursor(win)
    local view = vim.api.nvim_win_call(win, vim.fn.winsaveview)
    local row, col = M.map(hunks, old_lines, new_lines, cursor[1], cursor[2])
    windows[#windows + 1] = { win = win, view = view, from = cursor[1], row = row, col = col }
  end

  -- Bottom up, so the indices of the hunks still to come stay valid.
  for i = #hunks, 1, -1 do
    local hunk = hunks[i]
    vim.api.nvim_buf_set_lines(buf, hunk.first - 1, hunk.last, true, vim.list_slice(new_lines, hunk.bfirst, hunk.blast))
  end

  local count = vim.api.nvim_buf_line_count(buf)
  for _, target in ipairs(windows) do
    local row = math.max(1, math.min(target.row, count))
    local col = math.max(0, math.min(target.col, #(new_lines[row] or "")))
    -- Shifting the top line by as much as the cursor moved keeps the cursor at
    -- the same height on screen, so the view does not jump.
    target.view.topline = math.max(1, math.min(target.view.topline + (row - target.from), count))
    target.view.lnum, target.view.col, target.view.curswant = row, col, col
    vim.api.nvim_win_call(target.win, function()
      vim.fn.winrestview(target.view)
    end)
  end

  return true
end

return M
