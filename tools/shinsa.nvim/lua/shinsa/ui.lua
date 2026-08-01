local M = {}

local git = require("shinsa.git")
local ledger = require("shinsa.ledger")

local function new_state()
  return {
    bufnr = nil,
    winid = nil,
    source_winid = nil,
    result = nil,
    items = {},
    line_items = {},
    item_lines = {},
    item_indices = {},
    actions = {},
  }
end

local state = new_state()

local namespace = vim.api.nvim_create_namespace("shinsa")

local function valid_buf(bufnr)
  return bufnr and vim.api.nvim_buf_is_valid(bufnr)
end

local function valid_win(winid)
  return winid and vim.api.nvim_win_is_valid(winid)
end

local function notify_error(message)
  vim.notify(message, vim.log.levels.ERROR, { title = "Shinsa" })
end

local function one_line(value)
  return tostring(value or ""):gsub("%c", " ")
end

local function set_lines(lines)
  vim.bo[state.bufnr].modifiable = true
  vim.api.nvim_buf_set_lines(state.bufnr, 0, -1, false, lines)
  vim.bo[state.bufnr].modifiable = false
end

local function open_window(config)
  if valid_win(state.winid) then
    vim.api.nvim_set_current_win(state.winid)
    return
  end
  local current = vim.api.nvim_get_current_win()
  if not valid_buf(state.bufnr) or vim.api.nvim_win_get_buf(current) ~= state.bufnr then
    state.source_winid = current
  end
  vim.cmd(config.panel.side == "left" and "topleft vsplit" or "botright vsplit")
  state.winid = vim.api.nvim_get_current_win()
  vim.api.nvim_win_set_width(state.winid, config.panel.width)
  vim.api.nvim_win_set_buf(state.winid, state.bufnr)
  vim.wo[state.winid].number = false
  vim.wo[state.winid].relativenumber = false
  vim.wo[state.winid].signcolumn = "no"
  vim.wo[state.winid].wrap = false
  vim.wo[state.winid].cursorline = true
end

local function selected_item()
  if not valid_win(state.winid) then
    return nil
  end
  return state.line_items[vim.api.nvim_win_get_cursor(state.winid)[1]]
end

local function open_item(item)
  if not item then
    notify_error("select a review item")
    return false
  end
  if item.deleted then
    notify_error("deleted files are available through the diff view")
    return false
  end
  local path = git.join(state.result.root, item.path)
  if not path or vim.fn.filereadable(path) ~= 1 then
    notify_error("source file is unavailable: " .. tostring(item.path))
    return false
  end
  local target = state.source_winid
  if not valid_win(target) or target == state.winid then
    vim.cmd("rightbelow vsplit")
    target = vim.api.nvim_get_current_win()
    state.source_winid = target
  else
    vim.api.nvim_set_current_win(target)
  end
  vim.cmd("edit " .. vim.fn.fnameescape(path))
  local last = vim.api.nvim_buf_line_count(0)
  vim.api.nvim_win_set_cursor(target, { math.max(1, math.min(item.line or 1, last)), 0 })
  vim.cmd("normal! zz")
  return true
end

local function item_line(item)
  local marker = item.concern and "!" or (item.reviewed and "✓" or "·")
  local risk = item.risk == "high" and "H" or (item.risk == "medium" and "M" or "L")
  local reasons = #item.reasons > 0 and ("  " .. table.concat(item.reasons, " ")) or ""
  return string.format(
    "%s %s %2d  %-7s %s  %s:%d%s",
    marker,
    risk,
    item.score,
    item.kind,
    one_line(item.signature),
    one_line(item.path),
    item.line,
    reasons
  )
end

local function concern(item)
  if not item then
    notify_error("select a review item")
    return
  end
  local entry = ledger.get(item.id)
  vim.ui.input({
    prompt = string.format("Concern for %s:%d: ", one_line(item.path), item.line),
    default = entry and entry.concern or nil,
  }, function(value)
    if value == nil then
      return
    end
    local _, err = ledger.set_concern(item.id, value)
    if err then
      notify_error(err)
      return
    end
    M.render()
  end)
end

local function jump_unreviewed(direction)
  if #state.items == 0 or not valid_win(state.winid) then
    return
  end
  local current_line = vim.api.nvim_win_get_cursor(state.winid)[1]
  local current_item = state.line_items[current_line]
  local current_index = current_item and state.item_indices[current_item.id] or 0
  if current_index == 0 and direction < 0 then
    current_index = 1
  end
  for offset = 1, #state.items do
    local index = ((current_index - 1 + direction * offset) % #state.items) + 1
    local item = state.items[index]
    local entry = ledger.get(item.id)
    if entry and not entry.reviewed then
      vim.api.nvim_win_set_cursor(state.winid, { state.item_lines[item.id], 0 })
      return item
    end
  end
end

local function show_help()
  vim.notify(
    table.concat({
      "<CR>  open source",
      "d     open precise diff",
      "x     toggle reviewed",
      "c     add or edit a concern",
      "C     clear concern",
      "]u/[u next/previous unseen item",
      "s     open highest-risk unseen item",
      "y     yank concerns for an agent",
      "g     hand off to LazyGit or Neogit",
      "r     refresh analysis",
      "q     close queue",
    }, "\n"),
    vim.log.levels.INFO,
    { title = "Shinsa mappings" }
  )
end

local function map(lhs, callback, description)
  vim.keymap.set("n", lhs, callback, { buffer = state.bufnr, silent = true, nowait = true, desc = description })
end

local function ensure_buffer()
  if valid_buf(state.bufnr) then
    return
  end
  state.bufnr = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_name(state.bufnr, "shinsa://queue")
  vim.bo[state.bufnr].buftype = "nofile"
  vim.bo[state.bufnr].bufhidden = "hide"
  vim.bo[state.bufnr].swapfile = false
  vim.bo[state.bufnr].filetype = "shinsa"
  map("<CR>", function()
    open_item(selected_item())
  end, "Open review item")
  map("d", function()
    local item = selected_item()
    if item and (item.deleted or open_item(item)) and state.actions.diff then
      state.actions.diff(item)
    end
  end, "Open precise diff")
  map("x", function()
    local item = selected_item()
    if item then
      ledger.toggle_reviewed(item.id)
      M.render()
    end
  end, "Toggle reviewed")
  map("c", function()
    concern(selected_item())
  end, "Add or edit concern")
  map("C", function()
    local item = selected_item()
    if item then
      ledger.clear_concern(item.id)
      M.render()
    end
  end, "Clear concern")
  map("]u", function()
    jump_unreviewed(1)
  end, "Next unseen review item")
  map("[u", function()
    jump_unreviewed(-1)
  end, "Previous unseen review item")
  map("s", M.start, "Start or resume review")
  map("y", function()
    if state.actions.yank then
      state.actions.yank()
    end
  end, "Yank concerns")
  map("g", function()
    if state.actions.git then
      state.actions.git()
    end
  end, "Open Git UI")
  map("r", function()
    if state.actions.refresh then
      state.actions.refresh()
    end
  end, "Refresh review queue")
  map("q", M.close, "Close review queue")
  map("g?", show_help, "Shinsa help")
end

function M.loading(config, message, actions)
  ensure_buffer()
  open_window(config)
  if actions then
    state.actions = actions
  end
  state.line_items = {}
  state.item_lines = {}
  state.item_indices = {}
  set_lines({ "Shinsa", "", "  " .. message .. "…" })
end

function M.failure(config, message)
  ensure_buffer()
  open_window(config)
  if state.result then
    M.render()
    return
  end
  local detail = tostring(message or "unknown error"):gsub("[\r\n]+", " ")
  set_lines({ "Shinsa", "", "Analysis failed", "", detail, "", "Press r to retry or q to close." })
end

function M.show(config, result, actions)
  ensure_buffer()
  open_window(config)
  state.result = result
  state.actions = actions or {}
  M.render()
end

function M.render()
  if not state.result or not valid_buf(state.bufnr) then
    return
  end
  state.items = ledger.decorate(state.result.items)
  local summary = ledger.summary(state.result.items)
  local high_unseen = 0
  for _, item in ipairs(state.items) do
    if item.risk == "high" and not item.reviewed then
      high_unseen = high_unseen + 1
    end
  end
  local lines = {
    string.format("Shinsa · working tree vs %s", state.result.base),
    string.format(
      "%d/%d reviewed · %d concerns · %d high-risk unseen",
      summary.reviewed,
      summary.total,
      summary.concerns,
      high_unseen
    ),
    "<CR> open  d diff  x reviewed  c concern  ]u unseen  y yank  g git  g? help",
    "",
  }
  state.line_items = {}
  state.item_lines = {}
  state.item_indices = {}
  if #state.items == 0 then
    table.insert(lines, "No working-tree changes found.")
  else
    for index, item in ipairs(state.items) do
      table.insert(lines, item_line(item))
      state.line_items[#lines] = item
      state.item_lines[item.id] = #lines
      state.item_indices[item.id] = index
    end
  end
  set_lines(lines)
  vim.api.nvim_buf_clear_namespace(state.bufnr, namespace, 0, -1)
  for _, item in ipairs(state.items) do
    local group = item.concern and "DiagnosticError"
      or (item.reviewed and "Comment" or (item.risk == "high" and "WarningMsg" or "Normal"))
    vim.api.nvim_buf_add_highlight(state.bufnr, namespace, group, state.item_lines[item.id] - 1, 0, -1)
  end
end

function M.close()
  if valid_win(state.winid) then
    vim.api.nvim_win_close(state.winid, true)
  end
  state.winid = nil
end

function M.open(config)
  if not valid_buf(state.bufnr) or not state.result then
    return false
  end
  open_window(config)
  return true
end

function M.is_open()
  return valid_win(state.winid)
end

function M.is_current()
  return valid_buf(state.bufnr) and vim.api.nvim_get_current_buf() == state.bufnr
end

function M.start()
  if not valid_win(state.winid) then
    return false
  end
  for _, item in ipairs(state.items) do
    local entry = ledger.get(item.id)
    if entry and not entry.reviewed then
      vim.api.nvim_win_set_cursor(state.winid, { state.item_lines[item.id], 0 })
      return open_item(item)
    end
  end
  vim.notify("All review items are marked reviewed", vim.log.levels.INFO, { title = "Shinsa" })
  return false
end

function M._state()
  return state
end

function M._reset()
  M.close()
  if valid_buf(state.bufnr) then
    vim.api.nvim_buf_delete(state.bufnr, { force = true })
  end
  state = new_state()
end

return M
