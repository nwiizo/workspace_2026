local function check(name, fn)
	local ok, err = pcall(fn)
	if not ok then
		io.stderr:write("FAIL " .. name .. ": " .. tostring(err) .. "\n")
		vim.cmd("cquit 1")
	end
	io.stdout:write("PASS " .. name .. "\n")
end

local function equal(expected, actual)
	assert(vim.deep_equal(expected, actual), "expected " .. vim.inspect(expected) .. ", got " .. vim.inspect(actual))
end

check("source order and horizontal blocks", function()
	local blocks = require("tategaki.render").parse({
		"# 見出し",
		"",
		"日本語の本文です。",
		"",
		"```mermaid",
		"graph LR",
		"A --> B",
		"```",
		"",
		"| 列 | 値 |",
		"| -- | -- |",
		"| A | B |",
		"",
		"![構成図](diagram.png)",
		"",
		"次の本文",
	})
	equal(
		{ "prose", "horizontal", "horizontal", "horizontal", "prose" },
		vim.tbl_map(function(block)
			return block.kind
		end, blocks)
	)
	equal({ "graph LR", "A --> B" }, blocks[2].lines)
	equal({ "| 列 | 値 |", "| -- | -- |", "| A | B |" }, blocks[3].lines)
	equal({ "![構成図](diagram.png)" }, blocks[4].lines)
end)

check("vertical pages read top to bottom, right to left", function()
	local render = require("tategaki.render")
	local pages = render.pages({ { kind = "prose", text = "あいうえおかきく" } }, 5, 7)
	equal(2, #pages)
	equal("vertical", pages[1].kind)
	equal({ "  う あ", "  え い" }, { pages[1].lines[1], pages[1].lines[2] })
	equal("  き お", pages[2].lines[1])
end)

check("inline code identifiers and punctuation remain readable", function()
	local render = require("tategaki.render")
	local blocks = render.parse({ "`foo_bar`とfoo_bar_bazと**強調**。" })
	equal("foo_barとfoo_bar_bazと強調。", blocks[1].text)
	local page = render.pages(blocks, 20, 7)[1]
	assert(table.concat(page.lines, "\n"):find("︒", 1, true))
end)

check("tables without outside pipes remain horizontal", function()
	local blocks = require("tategaki.render").parse({ "名前 | 値", "--- | ---", "A | B" })
	equal(1, #blocks)
	equal("horizontal", blocks[1].kind)
	equal({ "名前 | 値", "--- | ---", "A | B" }, blocks[1].lines)
end)

check("horizontal code does not transpose and pages keep order", function()
	local render = require("tategaki.render")
	local pages = render.pages({
		{ kind = "prose", text = "本文" },
		{ kind = "horizontal", lines = { "fn main() {", '  println!("hi");', "}" } },
		{ kind = "prose", text = "続き" },
	}, 6, 12)
	equal(
		{ "vertical", "horizontal", "vertical" },
		vim.tbl_map(function(page)
			return page.kind
		end, pages)
	)
	equal("fn main() {", pages[2].lines[1])
end)

check("command opens a reader without modifying the source", function()
	vim.opt.runtimepath:prepend(vim.fn.getcwd())
	vim.cmd("runtime plugin/tategaki.lua")
	local source = vim.api.nvim_get_current_buf()
	vim.bo[source].filetype = "markdown"
	vim.api.nvim_buf_set_lines(source, 0, -1, false, { "本文", "", "```lua", "print(1)", "```" })
	vim.cmd("Tategaki")
	assert(vim.api.nvim_get_current_buf() ~= source)
	assert(vim.bo.buftype == "nofile")
	require("tategaki").next_page()
	equal("print(1)", vim.api.nvim_buf_get_lines(0, 2, 3, false)[1])
	vim.api.nvim_buf_set_lines(source, 3, 4, false, { "print(2)" })
	require("tategaki").refresh()
	equal("print(2)", vim.api.nvim_buf_get_lines(0, 2, 3, false)[1])
	require("tategaki").close()
	equal(source, vim.api.nvim_get_current_buf())
	equal({ "本文", "", "```lua", "print(2)", "```" }, vim.api.nvim_buf_get_lines(source, 0, -1, false))
end)

vim.cmd("qa!")
