local config = require("herdr.config")
local state = require("herdr.state")

local M = {}

local buffer
local window
local line_agents = {}
local help_visible = false
local namespace = vim.api.nvim_create_namespace("herdr-board")
local augroup
local origin_root

local highlight_for = {
  blocked = "HerdrBlocked",
  working = "HerdrWorking",
  done = "HerdrDone",
  idle = "HerdrIdle",
  unknown = "HerdrUnknown",
}

local function is_buffer_valid()
  return buffer and vim.api.nvim_buf_is_valid(buffer)
end

function M.is_open()
  return window and vim.api.nvim_win_is_valid(window)
end

local function selected_terminal_id()
  if not M.is_open() then
    return nil
  end
  local line = vim.api.nvim_win_get_cursor(window)[1]
  local agent = line_agents[line]
  return agent and agent.terminal_id or nil
end

local function truncate(value, width)
  value = tostring(value or "")
  if width <= 0 then
    return ""
  end
  if vim.fn.strdisplaywidth(value) <= width then
    return value
  end
  local result = ""
  local char_count = vim.fn.strchars(value)
  for index = 0, char_count - 1 do
    local candidate = result .. vim.fn.strcharpart(value, index, 1)
    if vim.fn.strdisplaywidth(candidate .. "…") > width then
      break
    end
    result = candidate
  end
  return result .. "…"
end

local function pad(value, width)
  local result = truncate(value, width)
  return result .. string.rep(" ", math.max(0, width - vim.fn.strdisplaywidth(result)))
end

local function summary()
  local counts = state.counts()
  local markers = config.get().board.markers
  return string.format(
    "Herdr  %s%d  %s%d  %s%d",
    markers.blocked,
    counts.blocked,
    markers.working,
    counts.working,
    markers.done,
    counts.done
  )
end

local function render_guidance(lines, err)
  local command = config.get().herdr_cmd
  if err.kind == "executable" then
    table.insert(lines, "Install Herdr: https://herdr.dev/")
    table.insert(lines, "Run :checkhealth herdr")
  elseif err.kind == "server" then
    table.insert(lines, "Run: " .. command .. " server")
    table.insert(lines, "Press r to retry; :HerdrHealth for details")
  elseif err.kind == "protocol" then
    table.insert(lines, "Update Herdr, then run :HerdrHealth")
  else
    table.insert(lines, "Press r to retry; run :HerdrHealth")
  end
end

local function render_error(lines, status)
  if status.error then
    table.insert(lines, "")
    table.insert(lines, "Herdr unavailable")
    table.insert(lines, truncate(status.error.message or "unknown error", config.get().board.width - 2))
    render_guidance(lines, status.error)
  else
    table.insert(lines, "")
    table.insert(lines, "Loading Herdr agents…")
  end
end

function M.render()
  if not is_buffer_valid() then
    return
  end
  local selected = selected_terminal_id()
  local status = state.get()
  local lines = { summary() .. (status.stale and "  ~ stale" or "") }
  local highlights = {}
  line_agents = {}

  if not status.snapshot then
    render_error(lines, status)
  else
    if status.stale and status.error then
      table.insert(
        lines,
        truncate("Last refresh failed: " .. (status.error.message or status.error.kind), config.get().board.width)
      )
      render_guidance(lines, status.error)
    end
    local groups = state.grouped_agents()
    if #groups == 0 then
      table.insert(lines, "")
      table.insert(lines, "No agents. Press a to start one.")
    end
    for _, group in ipairs(groups) do
      table.insert(lines, "")
      table.insert(lines, truncate(group.label, config.get().board.width - 1))
      for _, agent in ipairs(group.agents) do
        local marker = config.get().board.markers[agent.status]
        local detail = agent.title ~= "" and agent.title or (agent.cwd or "")
        local line = string.format(
          " %s %s %s %s %s",
          marker,
          pad(agent.name, 12),
          pad(agent.kind, 7),
          pad(agent.status, 7),
          detail
        )
        line = truncate(line, config.get().board.width)
        table.insert(lines, line)
        line_agents[#lines] = agent
        highlights[#lines] = highlight_for[agent.status]
      end
    end
  end

  if help_visible then
    vim.list_extend(lines, {
      "",
      "<CR> attach   a start   s send",
      "r refresh     q close   ? help",
    })
  end

  vim.bo[buffer].modifiable = true
  vim.api.nvim_buf_set_lines(buffer, 0, -1, false, lines)
  vim.bo[buffer].modifiable = false
  vim.api.nvim_buf_clear_namespace(buffer, namespace, 0, -1)
  for line, group in pairs(highlights) do
    vim.api.nvim_buf_add_highlight(buffer, namespace, group, line - 1, 0, -1)
  end

  if M.is_open() then
    local target_line
    if selected then
      for line, agent in pairs(line_agents) do
        if agent.terminal_id == selected then
          target_line = line
          break
        end
      end
    end
    if not target_line then
      for line = 1, #lines do
        if line_agents[line] then
          target_line = line
          break
        end
      end
    end
    target_line = target_line or 1
    pcall(vim.api.nvim_win_set_cursor, window, { target_line, 0 })
  end
end

local function current_agent()
  if not M.is_open() then
    return nil
  end
  return line_agents[vim.api.nvim_win_get_cursor(window)[1]]
end

local function map(lhs, callback, description)
  vim.keymap.set("n", lhs, callback, { buffer = buffer, silent = true, nowait = true, desc = description })
end

local function require_agent_row()
  local agent = current_agent()
  if not agent then
    vim.notify("Select an agent row first", vim.log.levels.INFO, { title = "Herdr" })
  end
  return agent
end

local function configure_buffer()
  vim.bo[buffer].buftype = "nofile"
  vim.bo[buffer].bufhidden = "hide"
  vim.bo[buffer].swapfile = false
  vim.bo[buffer].modifiable = false
  vim.bo[buffer].filetype = "herdr"
  vim.bo[buffer].buflisted = false
  pcall(vim.api.nvim_buf_set_name, buffer, "herdr://board")

  map("<CR>", function()
    local agent = require_agent_row()
    if agent then
      require("herdr").attach(agent.terminal_id)
    end
  end, "Attach to Herdr agent")
  map("a", function()
    require("herdr").start(nil, { cwd = origin_root })
  end, "Start Herdr agent")
  map("s", function()
    local agent = require_agent_row()
    if agent then
      require("herdr").send(agent.terminal_id)
    end
  end, "Send instruction to Herdr agent")
  map("r", function()
    require("herdr").refresh()
  end, "Refresh Herdr agents")
  map("q", M.close, "Close Herdr board")
  map("?", function()
    help_visible = not help_visible
    M.render()
  end, "Toggle Herdr board help")

  vim.api.nvim_create_autocmd("BufWinLeave", {
    buffer = buffer,
    callback = function()
      vim.schedule(function()
        if not M.is_open() then
          state.set_board_visible(false)
        end
      end)
    end,
  })
end

local function open_split()
  local board_config = config.get().board
  local modifier = board_config.side == "left" and "topleft" or "botright"
  vim.cmd(string.format("%s %dvnew", modifier, board_config.width))
  window = vim.api.nvim_get_current_win()
  if is_buffer_valid() then
    vim.api.nvim_win_set_buf(window, buffer)
  else
    buffer = vim.api.nvim_get_current_buf()
    configure_buffer()
  end
  vim.api.nvim_win_set_width(window, board_config.width)
  vim.wo[window].wrap = false
  vim.wo[window].linebreak = false
end

function M.open()
  if M.is_open() then
    vim.api.nvim_set_current_win(window)
    return
  end
  origin_root = require("herdr.context").project_root(0)
  open_split()
  state.set_board_visible(true)
  M.render()
end

function M.close()
  if M.is_open() then
    vim.api.nvim_win_close(window, true)
  end
  window = nil
  state.set_board_visible(false)
end

function M.toggle()
  if M.is_open() then
    M.close()
  else
    M.open()
  end
end

function M.setup()
  vim.api.nvim_set_hl(0, "HerdrBlocked", { default = true, link = "DiagnosticError" })
  vim.api.nvim_set_hl(0, "HerdrWorking", { default = true, link = "DiagnosticInfo" })
  vim.api.nvim_set_hl(0, "HerdrDone", { default = true, link = "DiagnosticOk" })
  vim.api.nvim_set_hl(0, "HerdrIdle", { default = true, link = "Comment" })
  vim.api.nvim_set_hl(0, "HerdrUnknown", { default = true, link = "DiagnosticWarn" })
  augroup = vim.api.nvim_create_augroup("HerdrBoard", { clear = true })
  vim.api.nvim_create_autocmd("User", {
    group = augroup,
    pattern = "HerdrUpdated",
    callback = function()
      if M.is_open() then
        M.render()
      end
    end,
  })
end

function M._buffer()
  return buffer
end

function M._line_agents()
  return line_agents
end

function M._reset()
  M.close()
  if is_buffer_valid() then
    vim.api.nvim_buf_delete(buffer, { force = true })
  end
  buffer = nil
  window = nil
  line_agents = {}
  help_visible = false
  origin_root = nil
  if augroup then
    pcall(vim.api.nvim_del_augroup_by_id, augroup)
  end
  augroup = nil
end

return M
