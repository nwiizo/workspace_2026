local M = {}

local defaults = {
  herdr_cmd = "herdr",
  auto_start_server = true,
  server_retry_ms = { 100, 300, 1000 },
  agents = {
    codex = { command = { "codex" } },
    claude = { command = { "claude" } },
  },
  refresh = {
    board_ms = 1000,
    background_ms = 5000,
    timeout_ms = 3000,
  },
  board = {
    side = "left",
    width = 44,
    markers = {
      blocked = "!",
      working = "*",
      done = "✓",
      idle = "·",
      unknown = "?",
    },
  },
  terminal = {
    side = "right",
    width = 0.4,
    auto_insert = true,
  },
  notifications = {
    blocked = true,
    done = true,
  },
  context = {
    max_lines = 500,
    max_bytes = 65536,
  },
}

local values

local function fail(path, expected)
  error(string.format("herdr.nvim: %s must be %s", path, expected), 3)
end

local function validate_string(value, path)
  if type(value) ~= "string" or value == "" then
    fail(path, "a non-empty string")
  end
end

local function validate_positive_integer(value, path)
  if type(value) ~= "number" or value <= 0 or value % 1 ~= 0 then
    fail(path, "a positive integer")
  end
end

local function validate_table(value, path)
  if type(value) ~= "table" then
    fail(path, "a table")
  end
end

local function validate_command(command, path)
  if type(command) ~= "table" or not vim.islist(command) or #command == 0 then
    fail(path, "a non-empty argv list")
  end
  for index, value in ipairs(command) do
    validate_string(value, string.format("%s[%d]", path, index))
  end
end

local function validate(config)
  validate_table(config, "setup options")
  validate_string(config.herdr_cmd, "herdr_cmd")
  if type(config.auto_start_server) ~= "boolean" then
    fail("auto_start_server", "a boolean")
  end

  if
    type(config.server_retry_ms) ~= "table"
    or not vim.islist(config.server_retry_ms)
    or #config.server_retry_ms == 0
  then
    fail("server_retry_ms", "a non-empty list of positive integers")
  end
  for index, delay in ipairs(config.server_retry_ms) do
    validate_positive_integer(delay, string.format("server_retry_ms[%d]", index))
  end

  if type(config.agents) ~= "table" or vim.islist(config.agents) or next(config.agents) == nil then
    fail("agents", "a non-empty table")
  end
  for name, agent in pairs(config.agents) do
    validate_string(name, "agents key")
    if type(agent) ~= "table" then
      fail("agents." .. name, "a table")
    end
    validate_command(agent.command, "agents." .. name .. ".command")
  end

  validate_table(config.refresh, "refresh")
  for _, key in ipairs({ "board_ms", "background_ms", "timeout_ms" }) do
    validate_positive_integer(config.refresh[key], "refresh." .. key)
  end

  validate_table(config.board, "board")
  if config.board.side ~= "left" and config.board.side ~= "right" then
    fail("board.side", '"left" or "right"')
  end
  validate_positive_integer(config.board.width, "board.width")
  validate_table(config.board.markers, "board.markers")
  for _, status in ipairs({ "blocked", "working", "done", "idle", "unknown" }) do
    validate_string(config.board.markers[status], "board.markers." .. status)
  end

  validate_table(config.terminal, "terminal")
  if config.terminal.side ~= "left" and config.terminal.side ~= "right" then
    fail("terminal.side", '"left" or "right"')
  end
  if type(config.terminal.width) ~= "number" or config.terminal.width <= 0 then
    fail("terminal.width", "a positive number")
  end
  if config.terminal.width >= 1 and config.terminal.width % 1 ~= 0 then
    fail("terminal.width", "a fraction below 1 or an integer column count")
  end
  if type(config.terminal.auto_insert) ~= "boolean" then
    fail("terminal.auto_insert", "a boolean")
  end

  validate_table(config.notifications, "notifications")
  for _, status in ipairs({ "blocked", "done" }) do
    if type(config.notifications[status]) ~= "boolean" then
      fail("notifications." .. status, "a boolean")
    end
  end

  validate_table(config.context, "context")
  validate_positive_integer(config.context.max_lines, "context.max_lines")
  validate_positive_integer(config.context.max_bytes, "context.max_bytes")
end

function M.setup(opts)
  if opts ~= nil and type(opts) ~= "table" then
    fail("setup options", "a table or nil")
  end
  local candidate = vim.tbl_deep_extend("force", vim.deepcopy(defaults), vim.deepcopy(opts or {}))
  validate(candidate)
  values = candidate
  return values
end

function M.get()
  if not values then
    return M.setup()
  end
  return values
end

function M.defaults()
  return vim.deepcopy(defaults)
end

function M._reset()
  values = nil
end

return M
