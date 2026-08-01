local M = {}

local defaults = {
  git_command = { "git" },
  base = nil,
  include_untracked = true,
  max_file_bytes = 2 * 1024 * 1024,
  panel = {
    side = "left",
    width = 64,
  },
  handoff = {
    diff = "auto",
    git = "auto",
  },
  clipboard_register = "+",
}

local values
local islist = vim.islist or vim.tbl_islist

local function fail(path, expected)
  error(string.format("shinsa.nvim: %s must be %s", path, expected), 3)
end

local function validate_command(value, path)
  if type(value) ~= "table" or not islist(value) or #value == 0 then
    fail(path, "a non-empty argv list")
  end
  for index, part in ipairs(value) do
    if type(part) ~= "string" or part == "" then
      fail(string.format("%s[%d]", path, index), "a non-empty string")
    end
  end
end

local function validate_handoff(value, path, allowed)
  if type(value) == "function" or value == false then
    return
  end
  for _, candidate in ipairs(allowed) do
    if value == candidate then
      return
    end
  end
  fail(path, "one of " .. table.concat(allowed, ", ") .. ", false, or a function")
end

local function validate(config)
  if type(config) ~= "table" then
    fail("setup options", "a table")
  end
  validate_command(config.git_command, "git_command")
  if config.base ~= nil and type(config.base) ~= "string" and type(config.base) ~= "function" then
    fail("base", "a string, function, or nil")
  end
  if type(config.base) == "string" and config.base == "" then
    fail("base", "a non-empty string or nil")
  end
  if type(config.include_untracked) ~= "boolean" then
    fail("include_untracked", "a boolean")
  end
  if type(config.max_file_bytes) ~= "number" or config.max_file_bytes <= 0 or config.max_file_bytes % 1 ~= 0 then
    fail("max_file_bytes", "a positive integer")
  end
  if type(config.panel) ~= "table" then
    fail("panel", "a table")
  end
  if config.panel.side ~= "left" and config.panel.side ~= "right" then
    fail("panel.side", '"left" or "right"')
  end
  if type(config.panel.width) ~= "number" or config.panel.width < 30 or config.panel.width % 1 ~= 0 then
    fail("panel.width", "an integer of at least 30")
  end
  if type(config.handoff) ~= "table" then
    fail("handoff", "a table")
  end
  validate_handoff(config.handoff.diff, "handoff.diff", { "auto", "codediff", "diffview", "builtin" })
  validate_handoff(config.handoff.git, "handoff.git", { "auto", "lazygit", "neogit" })
  if type(config.clipboard_register) ~= "string" or config.clipboard_register == "" then
    fail("clipboard_register", "a non-empty string")
  end
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
  return values or M.setup()
end

function M.defaults()
  return vim.deepcopy(defaults)
end

function M._reset()
  values = nil
end

return M
