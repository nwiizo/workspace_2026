if vim.g.loaded_shinsa_nvim == 1 then
  return
end
vim.g.loaded_shinsa_nvim = 1

vim.api.nvim_create_user_command("Shinsa", function(args)
  require("shinsa").analyze(args.args ~= "" and args.args or nil)
end, { nargs = "?", desc = "Build a risk-ranked review queue for the working tree" })

vim.api.nvim_create_user_command("ShinsaRefresh", function()
  require("shinsa").refresh()
end, { desc = "Refresh the Shinsa queue" })

vim.api.nvim_create_user_command("ShinsaToggle", function()
  require("shinsa").toggle()
end, { desc = "Toggle the Shinsa queue" })

vim.api.nvim_create_user_command("ShinsaClose", function()
  require("shinsa").close()
end, { desc = "Close the Shinsa queue" })

vim.api.nvim_create_user_command("ShinsaReset", function()
  require("shinsa").reset()
end, { desc = "Reset reviewed states and concerns" })

vim.api.nvim_create_user_command("ShinsaYank", function()
  require("shinsa").yank()
end, { desc = "Yank concerns as agent-ready Markdown" })

vim.api.nvim_create_user_command("ShinsaHealth", function()
  vim.cmd("checkhealth shinsa")
end, { desc = "Run shinsa.nvim health checks" })
