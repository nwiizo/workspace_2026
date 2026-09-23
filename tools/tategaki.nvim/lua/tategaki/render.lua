local M = {}

local function is_fence(line)
	local mark = line:match("^%s*(```+)") or line:match("^%s*(~~~+)")
	return mark
end

local function is_image(line)
	return line:match("^%s*!%[.-%]%(.+%)%s*$") ~= nil
end

local function is_table(line)
	return line:match("^%s*|.*|%s*$") ~= nil
end

local function is_table_separator(line)
	return line:find("|", 1, true) ~= nil and line:find("-", 1, true) ~= nil and line:gsub("[%s|:%-]", "") == ""
end

local function clean(line)
	line = line:gsub("^%s*#+%s+", "")
	line = line:gsub("^%s*>%s?", "")
	line = line:gsub("^%s*[-*+]%s+", "・")
	line = line:gsub("%[([^%]]+)%]%([^%)]+%)", "%1")
	line = line:gsub("`([^`]+)`", "%1")
	line = line:gsub("%*%*([^*]+)%*%*", "%1")
	line = line:gsub("%*([^*]+)%*", "%1")
	return vim.trim(line)
end

function M.parse(lines)
	local blocks, prose = {}, {}
	local function flush()
		if #prose > 0 then
			blocks[#blocks + 1] = { kind = "prose", text = table.concat(prose, "　") }
			prose = {}
		end
	end

	local i = 1
	while i <= #lines do
		local line = lines[i]
		local fence = is_fence(line)
		if fence then
			flush()
			local body = {}
			local marker = fence:sub(1, 1)
			i = i + 1
			while i <= #lines do
				local closing = lines[i]:match("^%s*([`~]+)%s*$")
				if closing and closing:sub(1, 1) == marker and #closing >= #fence then
					break
				end
				body[#body + 1] = lines[i]
				i = i + 1
			end
			blocks[#blocks + 1] = { kind = "horizontal", lines = body }
		elseif is_table(line) or (line:find("|", 1, true) and lines[i + 1] and is_table_separator(lines[i + 1])) then
			flush()
			local body = {}
			while i <= #lines and lines[i]:find("|", 1, true) do
				body[#body + 1] = lines[i]
				i = i + 1
			end
			blocks[#blocks + 1] = { kind = "horizontal", lines = body }
			i = i - 1
		elseif is_image(line) or line:match("^    %S") or line:match("^%s*<svg[%s>]") then
			flush()
			local body = { line }
			if line:match("^    %S") then
				while i + 1 <= #lines and lines[i + 1]:match("^    %S") do
					i = i + 1
					body[#body + 1] = lines[i]
				end
			elseif line:match("^%s*<svg[%s>]") then
				while not body[#body]:find("</svg>", 1, true) and i + 1 <= #lines do
					i = i + 1
					body[#body + 1] = lines[i]
				end
			end
			blocks[#blocks + 1] = { kind = "horizontal", lines = body }
		elseif line:match("^%s*$") then
			if #prose > 0 and prose[#prose] ~= "" then
				prose[#prose + 1] = ""
			end
		else
			local value = clean(line)
			if value ~= "" then
				prose[#prose + 1] = value
			end
		end
		i = i + 1
	end
	flush()
	return blocks
end

local vertical_forms = {
	["、"] = "︑",
	["。"] = "︒",
	["（"] = "︵",
	["）"] = "︶",
	["「"] = "﹁",
	["」"] = "﹂",
	["『"] = "﹃",
	["』"] = "﹄",
	["［"] = "﹇",
	["］"] = "﹈",
	["…"] = "︙",
	["ー"] = "丨",
}

local function characters(value)
	local result = {}
	for _, char in ipairs(vim.fn.split(value, "\\zs")) do
		if vim.fn.strdisplaywidth(char) == 0 and #result > 0 then
			result[#result] = result[#result] .. char
		else
			result[#result + 1] = vertical_forms[char] or char
		end
	end
	return result
end

local function vertical_page(chars, first, height, columns, width)
	local rows = {}
	for row = 1, height do
		local cells = {}
		for column = columns - 1, 0, -1 do
			local char = chars[first + column * height + row - 1] or ""
			local cell_width = vim.fn.strdisplaywidth(char)
			cells[#cells + 1] = char .. string.rep(" ", math.max(0, 2 - cell_width))
		end
		rows[row] = string.rep(" ", math.max(0, width - (columns * 3 - 1))) .. table.concat(cells, " "):gsub("%s+$", "")
	end
	return rows
end

function M.pages(blocks, height, width)
	local pages = {}
	local content_height = math.max(1, height - 3)
	local columns = math.max(1, math.floor((width + 1) / 3))
	for _, block in ipairs(blocks) do
		if block.kind == "horizontal" then
			if #block.lines == 0 then
				pages[#pages + 1] = { kind = "horizontal", lines = { "" } }
			else
				for first = 1, #block.lines, content_height do
					pages[#pages + 1] = {
						kind = "horizontal",
						lines = vim.list_slice(block.lines, first, first + content_height - 1),
					}
				end
			end
		else
			local chars = characters(block.text)
			for first = 1, #chars, content_height * columns do
				pages[#pages + 1] = {
					kind = "vertical",
					lines = vertical_page(chars, first, content_height, columns, width),
				}
			end
		end
	end
	if #pages == 0 then
		pages[1] = { kind = "vertical", lines = { "" } }
	end
	return pages
end

return M
