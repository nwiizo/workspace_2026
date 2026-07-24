local config = require("herdr.config")

local M = {}

local notify = vim.notify
local seen = {}

function M.on_transition(event)
  if not config.get().notifications[event.status] then
    return
  end
  local key = table.concat({ event.terminal_id, event.status, tostring(event.revision) }, ":")
  if seen[key] then
    return
  end
  seen[key] = true
  local message = string.format("%s is %s", event.name or event.terminal_id, event.status)
  if event.title and event.title ~= "" then
    message = message .. ": " .. event.title
  end
  local level = event.status == "blocked" and vim.log.levels.WARN or vim.log.levels.INFO
  notify(message, level, { title = "Herdr" })
end

function M._set_notify(value)
  notify = value or vim.notify
end

function M._reset()
  notify = vim.notify
  seen = {}
end

return M
