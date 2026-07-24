local config = require("herdr.config")

local M = {}

local diagnostic_get = vim.diagnostic.get

local severity_names = {
  [vim.diagnostic.severity.ERROR] = "ERROR",
  [vim.diagnostic.severity.WARN] = "WARN",
  [vim.diagnostic.severity.INFO] = "INFO",
  [vim.diagnostic.severity.HINT] = "HINT",
}

local function buffer_path(bufnr)
  local path = vim.api.nvim_buf_get_name(bufnr)
  if path == "" then
    return nil, "buffer has no file name"
  end
  return vim.fs.normalize(path)
end

function M.project_root(bufnr)
  local ok, root = pcall(vim.fs.root, bufnr, ".git")
  if ok and root and root ~= "" then
    return vim.fs.normalize(root)
  end
  return vim.fs.normalize(vim.fn.getcwd())
end

local function relative_path(bufnr, path, target_cwd)
  if target_cwd == false then
    return path
  end
  local root = target_cwd and vim.fs.normalize(target_cwd) or M.project_root(bufnr)
  local prefix = root:sub(-1) == "/" and root or root .. "/"
  if path:sub(1, #prefix) == prefix then
    return path:sub(#prefix + 1)
  end
  if target_cwd then
    return path
  end
  local cwd = vim.fs.normalize(vim.fn.getcwd())
  prefix = cwd:sub(-1) == "/" and cwd or cwd .. "/"
  if path:sub(1, #prefix) == prefix then
    return path:sub(#prefix + 1)
  end
  return path
end

local function check_size(text, line_count)
  local limits = config.get().context
  if line_count and line_count > limits.max_lines then
    return nil, string.format("context has %d lines; maximum is %d", line_count, limits.max_lines)
  end
  if #text > limits.max_bytes then
    return nil, string.format("context has %d bytes; maximum is %d", #text, limits.max_bytes)
  end
  return text
end

local function code_fence(text)
  local longest = 2
  for run in text:gmatch("`+") do
    longest = math.max(longest, #run)
  end
  return string.rep("`", longest + 1)
end

function M.file(bufnr, opts)
  bufnr = bufnr or 0
  opts = opts or {}
  local path, err = buffer_path(bufnr)
  if not path then
    return nil, err
  end
  if vim.bo[bufnr].modified then
    return nil, "buffer has unsaved changes; save it before sending a file reference"
  end
  local text = string.format("Please inspect @%s in the current project.", relative_path(bufnr, path, opts.target_cwd))
  return check_size(text, 1)
end

local function inclusive_byte_end(line, byte_col)
  if byte_col >= #line then
    return #line
  end
  local ok, char_index = pcall(vim.str_utfindex, line, "utf-32", byte_col)
  if not ok then
    char_index = vim.str_utfindex(line, byte_col)
  end
  local byte_ok, next_byte = pcall(vim.str_byteindex, line, "utf-32", char_index + 1)
  if not byte_ok then
    next_byte = vim.str_byteindex(line, char_index + 1)
  end
  return next_byte
end

local function slice_lines_fallback(lines, mode, start_col, end_col)
  if mode == "V" or not start_col or not end_col then
    return lines
  end
  if mode == "\22" then
    for index, line in ipairs(lines) do
      lines[index] = line:sub(start_col + 1, inclusive_byte_end(line, end_col))
    end
    return lines
  end
  if #lines == 1 then
    lines[1] = lines[1]:sub(start_col + 1, inclusive_byte_end(lines[1], end_col))
  else
    lines[1] = lines[1]:sub(start_col + 1)
    lines[#lines] = lines[#lines]:sub(1, inclusive_byte_end(lines[#lines], end_col))
  end
  return lines
end

local function selected_lines(bufnr, line1, line2, opts)
  if opts.mode and opts.start_col and opts.end_col and vim.fn.exists("*getregion") == 1 then
    return vim.fn.getregion(
      { bufnr, line1, opts.start_col + 1, 0 },
      { bufnr, line2, opts.end_col + 1, 0 },
      { type = opts.mode, exclusive = false }
    )
  end
  local lines = vim.api.nvim_buf_get_lines(bufnr, line1 - 1, line2, false)
  return slice_lines_fallback(lines, opts.mode, opts.start_col, opts.end_col)
end

function M.range(bufnr, line1, line2, opts)
  bufnr = bufnr or 0
  opts = opts or {}
  local path, err = buffer_path(bufnr)
  if not path then
    return nil, err
  end
  if type(line1) ~= "number" or type(line2) ~= "number" or line1 < 1 or line2 < line1 then
    return nil, "invalid buffer range"
  end
  local line_count = line2 - line1 + 1
  if line_count > config.get().context.max_lines then
    return nil, string.format("context has %d lines; maximum is %d", line_count, config.get().context.max_lines)
  end
  local lines = selected_lines(bufnr, line1, line2, opts)
  local selected = table.concat(lines, "\n")
  local fence = code_fence(selected)
  local filetype = vim.bo[bufnr].filetype
  local language = filetype ~= "" and filetype or "text"
  local text = table.concat({
    string.format("Context from @%s (lines %d-%d):", relative_path(bufnr, path, opts.target_cwd), line1, line2),
    fence .. language,
    selected,
    fence,
  }, "\n")
  return check_size(text, line_count)
end

function M.range_from_marks(bufnr, line1, line2, opts)
  bufnr = bufnr or 0
  local start_mark = vim.api.nvim_buf_get_mark(bufnr, "<")
  local end_mark = vim.api.nvim_buf_get_mark(bufnr, ">")
  opts = vim.deepcopy(opts or {})
  if start_mark[1] ~= line1 or end_mark[1] ~= line2 then
    return nil, "visual selection marks do not match the command range"
  end
  local mode = opts.mode or vim.fn.visualmode()
  if mode ~= "v" and mode ~= "V" and mode ~= "\22" then
    return nil, "no valid visual selection is available"
  end
  opts.mode = mode
  opts.start_col = start_mark[2]
  opts.end_col = end_mark[2]
  return M.range(bufnr, line1, line2, opts)
end

function M.diagnostics(bufnr, opts)
  bufnr = bufnr or 0
  opts = opts or {}
  local path, err = buffer_path(bufnr)
  if not path then
    return nil, err
  end
  local diagnostics = diagnostic_get(bufnr)
  if #diagnostics == 0 then
    return nil, "buffer has no diagnostics"
  end

  table.sort(diagnostics, function(left, right)
    if left.lnum == right.lnum then
      return (left.col or 0) < (right.col or 0)
    end
    return left.lnum < right.lnum
  end)

  local lines = { string.format("Diagnostics for @%s:", relative_path(bufnr, path, opts.target_cwd)) }
  for _, diagnostic in ipairs(diagnostics) do
    local severity = severity_names[diagnostic.severity] or "UNKNOWN"
    local metadata = {}
    if diagnostic.source and diagnostic.source ~= "" then
      table.insert(metadata, diagnostic.source)
    end
    if diagnostic.code ~= nil and tostring(diagnostic.code) ~= "" then
      table.insert(metadata, tostring(diagnostic.code))
    end
    local suffix = #metadata > 0 and " [" .. table.concat(metadata, "/") .. "]" or ""
    local message = tostring(diagnostic.message or ""):gsub("%s+", " ")
    table.insert(
      lines,
      string.format("- %d:%d %s%s %s", diagnostic.lnum + 1, (diagnostic.col or 0) + 1, severity, suffix, message)
    )
  end
  local text = table.concat(lines, "\n")
  return check_size(text, #lines)
end

function M._set_diagnostic_get(value)
  diagnostic_get = value or vim.diagnostic.get
end

function M._reset()
  diagnostic_get = vim.diagnostic.get
end

return M
