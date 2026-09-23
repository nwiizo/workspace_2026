if vim.g.loaded_tategaki_nvim == 1 then
	return
end
vim.g.loaded_tategaki_nvim = 1

vim.api.nvim_create_user_command("Tategaki", function()
	require("tategaki").open()
end, { desc = "Read the current buffer vertically" })
