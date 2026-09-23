local render = require("tategaki.render")

local M = {}
local readers = {}

local function current()
	return readers[vim.api.nvim_get_current_buf()]
end

local function draw(reader, rebuild)
	if not vim.api.nvim_buf_is_valid(reader.source) then
		vim.notify("Tategaki: source buffer was closed", vim.log.levels.ERROR)
		return
	end
	if rebuild or not reader.pages then
		local lines = vim.api.nvim_buf_get_lines(reader.source, 0, -1, false)
		local height = vim.api.nvim_win_get_height(reader.window)
		local width = vim.api.nvim_win_get_width(reader.window)
		reader.pages = render.pages(render.parse(lines), height, width)
		reader.page = math.min(reader.page, #reader.pages)
	end
	local page = reader.pages[reader.page]
	local name = vim.fn.fnamemodify(vim.api.nvim_buf_get_name(reader.source), ":t")
	if name == "" then
		name = "[No Name]"
	end
	local header = string.format(
		"%s  %d/%d  %s",
		name,
		reader.page,
		#reader.pages,
		page.kind == "vertical" and "縦書き" or "横書き"
	)
	local output = { header, "" }
	vim.list_extend(output, page.lines)
	output[#output + 1] = "n 次へ  p 前へ  r 更新  q 閉じる"
	vim.bo[reader.buffer].modifiable = true
	vim.api.nvim_buf_set_lines(reader.buffer, 0, -1, false, output)
	vim.bo[reader.buffer].modifiable = false
	vim.api.nvim_win_set_cursor(reader.window, { 1, 0 })
end

function M.open()
	local source = vim.api.nvim_get_current_buf()
	vim.cmd("tabnew")
	local buffer = vim.api.nvim_get_current_buf()
	local window = vim.api.nvim_get_current_win()
	vim.bo[buffer].buftype = "nofile"
	vim.bo[buffer].bufhidden = "wipe"
	vim.bo[buffer].swapfile = false
	vim.bo[buffer].filetype = "tategaki"
	vim.wo[window].wrap = false
	vim.wo[window].number = false
	vim.wo[window].relativenumber = false
	vim.wo[window].list = false
	local reader = { source = source, buffer = buffer, window = window, page = 1 }
	readers[buffer] = reader
	for key, action in pairs({ n = M.next_page, p = M.previous_page, r = M.refresh, q = M.close }) do
		vim.keymap.set("n", key, action, { buffer = buffer, silent = true, nowait = true })
	end
	vim.api.nvim_create_autocmd("BufWipeout", {
		buffer = buffer,
		callback = function()
			readers[buffer] = nil
		end,
	})
	local resize_id = vim.api.nvim_create_autocmd("WinResized", {
		callback = function()
			if vim.api.nvim_win_is_valid(window) and vim.api.nvim_get_current_win() == window then
				draw(reader, true)
			end
		end,
	})
	vim.api.nvim_create_autocmd("BufWipeout", {
		buffer = buffer,
		callback = function()
			vim.api.nvim_del_autocmd(resize_id)
		end,
	})
	draw(reader)
end

function M.refresh()
	local reader = current()
	if reader then
		draw(reader, true)
	end
end

function M.next_page()
	local reader = current()
	if reader and reader.page < #reader.pages then
		reader.page = reader.page + 1
		draw(reader)
	end
end

function M.previous_page()
	local reader = current()
	if reader and reader.page > 1 then
		reader.page = reader.page - 1
		draw(reader)
	end
end

function M.close()
	if current() then
		vim.cmd("tabclose")
	end
end

return M
