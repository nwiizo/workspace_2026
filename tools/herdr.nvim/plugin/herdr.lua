if vim.g.loaded_herdr_nvim == 1 then
  return
end
vim.g.loaded_herdr_nvim = 1

vim.api.nvim_create_user_command("Herdr", function()
  require("herdr").toggle()
end, { desc = "Toggle the Herdr agent board" })

vim.api.nvim_create_user_command("HerdrRefresh", function()
  require("herdr").refresh()
end, { desc = "Refresh Herdr agent state" })

vim.api.nvim_create_user_command("HerdrStart", function(args)
  require("herdr").start(args.args ~= "" and args.args or nil)
end, {
  nargs = "?",
  complete = function()
    return require("herdr")._complete_agent_kinds()
  end,
  desc = "Start a persistent Herdr coding agent",
})

vim.api.nvim_create_user_command("HerdrAttach", function(args)
  require("herdr").attach(args.args ~= "" and args.args or nil, { takeover = args.bang })
end, {
  nargs = "?",
  bang = true,
  complete = function()
    return require("herdr")._complete_agents()
  end,
  desc = "Attach to a Herdr coding agent",
})

vim.api.nvim_create_user_command("HerdrSend", function(args)
  local target = args.args ~= "" and args.args or nil
  if args.range > 0 then
    require("herdr")._send_range(target, 0, args.line1, args.line2)
  else
    require("herdr").send(target)
  end
end, {
  nargs = "?",
  range = true,
  complete = function()
    return require("herdr")._complete_agents()
  end,
  desc = "Send an instruction or selected range to a Herdr agent",
})

vim.api.nvim_create_user_command("HerdrSendVisual", function(args)
  if args.range == 0 then
    vim.notify("HerdrSendVisual must be called with a visual range", vim.log.levels.ERROR, { title = "Herdr" })
    return
  end
  require("herdr")._send_visual(args.args ~= "" and args.args or nil, 0, args.line1, args.line2)
end, {
  nargs = "?",
  range = true,
  complete = function()
    return require("herdr")._complete_agents()
  end,
  desc = "Send an exact visual selection to a Herdr agent",
})

vim.api.nvim_create_user_command("HerdrSendFile", function(args)
  require("herdr")._send_file(args.args ~= "" and args.args or nil, 0)
end, {
  nargs = "?",
  complete = function()
    return require("herdr")._complete_agents()
  end,
  desc = "Send the current file reference to a Herdr agent",
})

vim.api.nvim_create_user_command("HerdrSendDiagnostics", function(args)
  require("herdr")._send_diagnostics(args.args ~= "" and args.args or nil, 0)
end, {
  nargs = "?",
  complete = function()
    return require("herdr")._complete_agents()
  end,
  desc = "Send current-buffer diagnostics to a Herdr agent",
})

vim.api.nvim_create_user_command("HerdrHealth", function()
  vim.cmd("checkhealth herdr")
end, { desc = "Run herdr.nvim health checks" })
