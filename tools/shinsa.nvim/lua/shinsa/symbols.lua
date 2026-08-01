local M = {}

local git = require("shinsa.git")

local node_kinds = {
  rust = {
    function_item = "fn",
    struct_item = "struct",
    enum_item = "enum",
    trait_item = "trait",
    type_item = "type",
    const_item = "const",
    static_item = "static",
    mod_item = "module",
  },
  go = {
    function_declaration = "fn",
    method_declaration = "fn",
    type_declaration = "type",
    const_declaration = "const",
    var_declaration = "var",
  },
  python = {
    function_definition = "fn",
    class_definition = "class",
  },
  typescript = {
    function_declaration = "fn",
    method_definition = "fn",
    class_declaration = "class",
    interface_declaration = "interface",
    type_alias_declaration = "type",
    enum_declaration = "enum",
    variable_declarator = "var",
  },
  tsx = {
    function_declaration = "fn",
    method_definition = "fn",
    class_declaration = "class",
    interface_declaration = "interface",
    type_alias_declaration = "type",
    enum_declaration = "enum",
    variable_declarator = "var",
  },
  javascript = {
    function_declaration = "fn",
    method_definition = "fn",
    class_declaration = "class",
    variable_declarator = "var",
  },
  jsx = {
    function_declaration = "fn",
    method_definition = "fn",
    class_declaration = "class",
    variable_declarator = "var",
  },
  lua = {
    function_declaration = "fn",
    function_definition = "fn",
  },
}

local function is_test(path, name)
  local lower = path:lower()
  return lower:match("^tests?/") ~= nil
    or lower:match("/tests?/") ~= nil
    or lower:match("_test%.[^/]+$") ~= nil
    or lower:match("%.test%.[^/]+$") ~= nil
    or lower:match("%.spec%.[^/]+$") ~= nil
    or (name or ""):match("^test[_A-Z]") ~= nil
end

local function file_item(change, reason)
  local first = change.ranges[1]
  local removed_lines = 0
  for _, range in ipairs(change.ranges) do
    removed_lines = removed_lines + (range.old_count or 0)
  end
  return {
    id = change.path .. "::file",
    path = change.path,
    line = first and math.max(1, first.new_start) or 1,
    end_line = first and math.max(1, first.new_start + math.max(first.new_count, 1) - 1) or 1,
    kind = "file",
    name = vim.fs.basename(change.path),
    signature = change.path,
    text = "",
    contract = change.new_file or change.deleted or reason == "top-level change",
    added = change.new_file,
    deleted = change.deleted,
    changed_lines = change.added + change.removed,
    removed_lines = removed_lines,
    removal = removed_lines > 0,
    is_test = is_test(change.path),
    exported = false,
    file_level = true,
    fallback_reason = reason,
  }
end

local function language_for(path)
  local filetype = vim.filetype.match({ filename = path })
  if not filetype then
    return nil
  end
  local ok, language = pcall(vim.treesitter.language.get_lang, filetype)
  return (ok and language) or filetype
end

local function read(path, max_bytes)
  local stat = vim.uv.fs_lstat(path)
  if not stat then
    return nil, "file is unavailable"
  end
  if stat.type == "link" then
    return nil, "symbolic link"
  end
  if stat.type ~= "file" then
    return nil, "unsupported file type"
  end
  if stat.size > max_bytes then
    return nil, string.format("file exceeds max_file_bytes (%d bytes)", stat.size)
  end
  local handle, err = io.open(path, "rb")
  if not handle then
    return nil, err
  end
  local content = handle:read("*a")
  handle:close()
  return content
end

local function node_text(node, content)
  local ok, text = pcall(vim.treesitter.get_node_text, node, content)
  return ok and text or ""
end

local function named_field(node, content)
  local ok, fields = pcall(function()
    return node:field("name")
  end)
  if ok and fields and fields[1] then
    return node_text(fields[1], content)
  end
  return nil
end

local function header(text, language)
  local lines = vim.split(text or "", "\n", { plain = true })
  local selected = {}
  for index, line in ipairs(lines) do
    if index > 8 then
      break
    end
    table.insert(selected, vim.trim(line))
    if language == "python" then
      if line:match(":%s*$") then
        break
      end
    elseif language == "lua" then
      break
    elseif line:find("{", 1, true) or line:match(";%s*$") then
      break
    end
  end
  local signature = vim.trim(table.concat(selected, " "):gsub("%s+", " "))
  local before_body = signature:match("^(.-)%s*{")
  if before_body and before_body ~= "" then
    signature = vim.trim(before_body)
  end
  if #signature > 180 then
    signature = signature:sub(1, 177) .. "..."
  end
  return signature, math.max(1, #selected)
end

local function inferred_name(signature, kind, line)
  local patterns = {
    "function%s+([%w_%.:]+)",
    "fn%s+([%w_]+)",
    "class%s+([%w_]+)",
    "struct%s+([%w_]+)",
    "enum%s+([%w_]+)",
    "trait%s+([%w_]+)",
    "interface%s+([%w_]+)",
    "type%s+([%w_]+)",
  }
  for _, pattern in ipairs(patterns) do
    local name = signature:match(pattern)
    if name then
      return name
    end
  end
  local assigned = signature:match("([%w_]+)%s*=")
  return assigned or string.format("%s@%d", kind, line)
end

local function enclosing_symbol(node, language)
  local rules = node_kinds[language]
  while node and rules do
    local kind = rules[node:type()]
    if kind then
      return node, kind
    end
    node = node:parent()
  end
  return nil
end

local function range_bounds(range, line_count)
  if range.new_count == 0 then
    return nil
  end
  local start_line = math.max(1, math.min(range.new_start, line_count))
  local end_line = math.max(start_line, math.min(range.new_start + math.max(range.new_count, 1) - 1, line_count))
  return start_line, end_line
end

local function overlaps(start_a, end_a, start_b, end_b)
  return start_a <= end_b and start_b <= end_a
end

local function exported(node, signature, language, name)
  if signature:match("^pub[%s%(]") or signature:match("^export%s") then
    return true
  end
  local parent = node:parent()
  while parent do
    if parent:type() == "export_statement" then
      return true
    end
    parent = parent:parent()
  end
  return language == "go" and name:match("^[A-Z]") ~= nil
end

local function parser_context(root, change, config)
  local path = git.join(root, change.path)
  if not path then
    return nil, "unsafe path"
  end
  local content, read_err = read(path, config.max_file_bytes)
  if not content then
    return nil, read_err
  end
  local language = language_for(change.path)
  if not language or not node_kinds[language] then
    return nil, "unsupported filetype"
  end
  local ok, parser = pcall(vim.treesitter.get_string_parser, content, language)
  if not ok or not parser then
    return nil, "Tree-sitter parser unavailable for " .. language
  end
  local parsed_ok, trees = pcall(function()
    return parser:parse()
  end)
  if not parsed_ok or not trees or not trees[1] then
    return nil, "Tree-sitter parse failed for " .. language
  end
  return {
    content = content,
    language = language,
    root_node = trees[1]:root(),
    lines = vim.split(content, "\n", { plain = true }),
  }
end

local function symbol_item(change, context, node, kind, start_row, node_end)
  local text = node_text(node, context.content)
  local signature, header_lines = header(text, context.language)
  local item_line = start_row + 1
  local name = named_field(node, context.content) or inferred_name(signature, kind, item_line)
  return {
    path = change.path,
    line = item_line,
    end_line = node_end,
    header_end = item_line + header_lines - 1,
    kind = kind,
    name = name,
    signature = signature ~= "" and signature or (kind .. " " .. name),
    text = text:sub(1, 65536),
    added = change.new_file,
    deleted = false,
    removal = false,
    removed_lines = 0,
    is_test = is_test(change.path, name),
    exported = exported(node, signature, context.language, name),
    file_level = false,
  }
end

local function collect_symbols(change, context)
  local found = {}
  local unmapped = false
  local has_removal = false
  for _, changed in ipairs(change.ranges) do
    local start_line, end_line = range_bounds(changed, math.max(1, #context.lines))
    if not start_line then
      has_removal = true
    else
      for line = start_line, end_line do
        local row = line - 1
        local length = #(context.lines[line] or "")
        local descendant = context.root_node:named_descendant_for_range(row, 0, row, math.max(0, length - 1))
        local node, kind = enclosing_symbol(descendant, context.language)
        if node then
          local start_row, _, end_row, end_col = node:range()
          local node_end = end_row + (end_col == 0 and 0 or 1)
          node_end = math.max(start_row + 1, node_end)
          local key = string.format("%s:%d:%d", node:type(), start_row, node_end)
          if not found[key] then
            found[key] = symbol_item(change, context, node, kind, start_row, node_end)
          end
        else
          unmapped = true
        end
      end
    end
  end
  return found, unmapped, has_removal
end

local function finalize_symbols(change, context, found)
  local items = {}
  local identity_counts = {}
  for _, item in pairs(found) do
    item.contract = false
    item.changed_lines = 0
    for _, changed in ipairs(change.ranges) do
      local range_start, range_end = range_bounds(changed, math.max(1, #context.lines))
      if range_start and overlaps(item.line, item.header_end, range_start, range_end) then
        item.contract = true
      end
      if range_start then
        local overlap_start = math.max(item.line, range_start)
        local overlap_end = math.min(item.end_line, range_end)
        if overlap_start <= overlap_end then
          local new_lines = overlap_end - overlap_start + 1
          local removed_lines = math.ceil((changed.old_count or 0) * new_lines / changed.new_count)
          item.removed_lines = item.removed_lines + removed_lines
          item.removal = item.removed_lines > 0
          item.changed_lines = item.changed_lines + new_lines + removed_lines
        end
      end
    end
    local identity = string.format("%s::%s::%s", item.path, item.kind, item.name)
    identity_counts[identity] = (identity_counts[identity] or 0) + 1
    item._identity = identity
    table.insert(items, item)
  end
  table.sort(items, function(a, b)
    return a.line < b.line
  end)
  for _, item in ipairs(items) do
    item.id = item._identity .. (identity_counts[item._identity] > 1 and ("@" .. item.line) or "")
    item._identity = nil
  end
  return items
end

local function fallback_reason(items, unmapped, has_removal)
  if #items == 0 then
    return has_removal and "deleted lines" or "no changed symbol found"
  end
  return unmapped and "top-level change" or "deleted lines"
end

function M.extract(root, change, config)
  if change.deleted or change.binary then
    return { file_item(change, change.deleted and "deleted file" or "binary file") }
  end
  if change.metadata_only then
    return { file_item(change, change.fallback_reason or "metadata-only change") }
  end
  local context, context_err = parser_context(root, change, config)
  if not context then
    return { file_item(change, context_err) }
  end
  local found, unmapped, has_removal = collect_symbols(change, context)
  local items = finalize_symbols(change, context, found)
  if #items == 0 or unmapped or has_removal then
    table.insert(items, file_item(change, fallback_reason(items, unmapped, has_removal)))
  end
  return items
end

M._header = header
M._language_for = language_for
M._exported = exported

return M
