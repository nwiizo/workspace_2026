local h = require("tests.harness")
local config = require("herdr.config")
local notifier = require("herdr.notifier")

local function reset(opts)
  config._reset()
  config.setup(opts)
  notifier._reset()
end

h.test("notifier reports configured attention states once per revision", function()
  reset()
  local notifications = {}
  notifier._set_notify(function(message, level)
    table.insert(notifications, { message = message, level = level })
  end)
  local blocked = { terminal_id = "a", status = "blocked", revision = 2, name = "api", title = "approval" }
  notifier.on_transition(blocked)
  notifier.on_transition(blocked)
  notifier.on_transition({ terminal_id = "a", status = "working", revision = 3, name = "api" })
  notifier.on_transition({ terminal_id = "a", status = "done", revision = 4, name = "api" })
  h.eq(2, #notifications)
  h.contains(notifications[1].message, "approval")
  h.eq(vim.log.levels.WARN, notifications[1].level)
  h.eq(vim.log.levels.INFO, notifications[2].level)
end)

h.test("notifier respects disabled done notifications", function()
  reset({ notifications = { done = false } })
  local count = 0
  notifier._set_notify(function()
    count = count + 1
  end)
  notifier.on_transition({ terminal_id = "a", status = "done", revision = 1, name = "api" })
  h.eq(0, count)
end)
