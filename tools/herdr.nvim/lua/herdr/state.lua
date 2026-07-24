local client = require("herdr.client")
local config = require("herdr.config")

local M = {}

local current
local by_id = {}
local stale = false
local last_error
local in_flight = false
local initialized = false
local board_visible = false
local polling = false
local timer
local generation = 0
local failure_notifications = {}
local timer_factory = function()
  return vim.uv.new_timer()
end
local emit_user = function(pattern, data)
  pcall(vim.api.nvim_exec_autocmds, "User", { pattern = pattern, data = data })
end
local transition_handler = function(event)
  require("herdr.notifier").on_transition(event)
end

local rank = {
  blocked = 1,
  done = 2,
  working = 3,
  idle = 4,
  unknown = 5,
}

local attention_status = {
  blocked = true,
  done = true,
}

local function emit_transition(agent, previous_status)
  local event = {
    terminal_id = agent.terminal_id,
    previous_status = previous_status,
    status = agent.status,
    revision = agent.revision,
    name = agent.name,
    title = agent.title,
    workspace_id = agent.workspace_id,
  }
  transition_handler(event)
  emit_user("HerdrAgentStatusChanged", event)
end

local function interval()
  return board_visible and config.get().refresh.board_ms or config.get().refresh.background_ms
end

local function stop_timer()
  if timer and not timer:is_closing() then
    timer:stop()
  end
end

local function restart_timer()
  stop_timer()
  if not polling then
    return
  end
  if not timer or timer:is_closing() then
    timer = timer_factory()
  end
  local delay = interval()
  timer:start(
    delay,
    delay,
    vim.schedule_wrap(function()
      M.refresh()
    end)
  )
end

local function counts(agents)
  local result = { blocked = 0, working = 0, done = 0, idle = 0, unknown = 0 }
  for _, agent in ipairs(agents or {}) do
    result[agent.status] = (result[agent.status] or 0) + 1
  end
  return result
end

function M._accept(snapshot)
  local previous = by_id
  local next_by_id = {}
  for _, agent in ipairs(snapshot.agents) do
    next_by_id[agent.terminal_id] = agent
  end

  current = snapshot
  by_id = next_by_id
  stale = false
  last_error = nil

  if initialized then
    for _, agent in ipairs(snapshot.agents) do
      local old = previous[agent.terminal_id]
      if old and old.status ~= agent.status then
        emit_transition(agent, old.status)
      elseif not old and attention_status[agent.status] then
        emit_transition(agent, nil)
      end
    end
  end
  initialized = true
  emit_user("HerdrUpdated", { stale = false, counts = counts(snapshot.agents) })
end

function M._fail(err, opts)
  opts = opts or {}
  stale = current ~= nil
  last_error = err
  if opts.stop then
    polling = false
    stop_timer()
  else
    restart_timer()
  end
  if err and err.kind == "executable" and not failure_notifications.executable then
    failure_notifications.executable = true
    vim.notify(
      "Herdr executable was not found. Install Herdr from https://herdr.dev/ and run :checkhealth herdr",
      vim.log.levels.ERROR,
      { title = "Herdr" }
    )
  end
  emit_user("HerdrUpdated", { stale = stale, error = err })
end

function M.refresh(opts, callback)
  opts = opts or {}
  callback = callback or function() end
  if in_flight then
    callback(false, { kind = "busy", message = "snapshot refresh already in progress" })
    return
  end
  if opts.explicit then
    polling = true
  end
  in_flight = true
  local request_generation = generation

  client.ensure_server(function(ok, server_err)
    if request_generation ~= generation then
      return
    end
    if not ok then
      in_flight = false
      M._fail(server_err, { stop = true })
      callback(false, server_err)
      return
    end
    client.snapshot(function(snapshot, err)
      if request_generation ~= generation then
        return
      end
      in_flight = false
      if err then
        if err.kind == "process" or err.kind == "executable" then
          client.mark_server_unavailable()
        end
        local terminal = err.kind == "executable" or err.kind == "protocol"
        M._fail(err, { stop = terminal })
        callback(false, err)
        return
      end
      M._accept(snapshot)
      if opts.explicit or not polling then
        polling = true
      end
      restart_timer()
      callback(true, snapshot)
    end)
  end)
end

function M.start()
  if polling then
    return
  end
  polling = true
  M.refresh({ explicit = true })
end

function M.set_board_visible(value)
  board_visible = value == true
  restart_timer()
end

function M.get()
  return {
    snapshot = current,
    stale = stale,
    error = last_error,
    initialized = initialized,
    in_flight = in_flight,
    polling = polling,
  }
end

function M.agents()
  return current and current.agents or {}
end

function M.agent(terminal_id)
  return by_id[terminal_id]
end

function M.counts()
  return counts(M.agents())
end

function M.grouped_agents()
  if not current then
    return {}
  end

  local groups_by_id = {}
  local groups = {}
  for _, workspace in ipairs(current.workspaces) do
    local group = { workspace_id = workspace.workspace_id, label = workspace.label, agents = {} }
    groups_by_id[workspace.workspace_id] = group
    table.insert(groups, group)
  end
  local source_order = {}
  for index, agent in ipairs(current.agents) do
    source_order[agent.terminal_id] = index
    local group = groups_by_id[agent.workspace_id]
    if not group then
      group = { workspace_id = agent.workspace_id, label = agent.workspace_id, agents = {} }
      groups_by_id[agent.workspace_id] = group
      table.insert(groups, group)
    end
    table.insert(group.agents, agent)
  end
  for _, group in ipairs(groups) do
    table.sort(group.agents, function(left, right)
      local left_rank = rank[left.status]
      local right_rank = rank[right.status]
      if left_rank == right_rank then
        return source_order[left.terminal_id] < source_order[right.terminal_id]
      end
      return left_rank < right_rank
    end)
  end
  return vim.tbl_filter(function(group)
    return #group.agents > 0
  end, groups)
end

function M.stop()
  generation = generation + 1
  polling = false
  in_flight = false
  if timer and not timer:is_closing() then
    timer:stop()
    timer:close()
  end
  timer = nil
end

function M._set_timer_factory(value)
  timer_factory = value
end

function M._set_emit(value)
  emit_user = value
end

function M._set_transition_handler(value)
  transition_handler = value
end

function M._reset()
  M.stop()
  current = nil
  by_id = {}
  stale = false
  last_error = nil
  in_flight = false
  initialized = false
  board_visible = false
  polling = false
  failure_notifications = {}
  timer_factory = function()
    return vim.uv.new_timer()
  end
  emit_user = function(pattern, data)
    pcall(vim.api.nvim_exec_autocmds, "User", { pattern = pattern, data = data })
  end
  transition_handler = function(event)
    require("herdr.notifier").on_transition(event)
  end
end

return M
