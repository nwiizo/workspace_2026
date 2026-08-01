local M = {}

local function count(value)
  if value == "" or value == nil then
    return 1
  end
  return tonumber(value) or 0
end

local function path_from_header(value)
  value = (value or ""):gsub("^%s+", "")
  if value == "/dev/null" then
    return nil
  end
  if value:sub(1, 1) == '"' then
    return nil
  end
  if value:sub(1, 2) == "a/" or value:sub(1, 2) == "b/" then
    return value:sub(3)
  end
  return value
end

function M.parse(text, metadata)
  local files = {}
  local current
  local header_paths = {}
  local metadata_paths = {}
  for _, record in ipairs(metadata or {}) do
    header_paths[string.format("diff --git a/%s b/%s", record.path, record.path)] = record.path
    metadata_paths[record.path] = true
  end

  local function finish()
    if not current then
      return
    end
    current.path = current.new_path or current.old_path or current.path
    if metadata ~= nil and not metadata_paths[current.path] then
      current.path = nil
    end
    if current.path then
      table.insert(files, current)
    end
    current = nil
  end

  for line in ((text or "") .. "\n"):gmatch("(.-)\n") do
    if line:match("^diff %-%-git ") then
      finish()
      current = {
        path = header_paths[line],
        ranges = {},
        added = 0,
        removed = 0,
        new_file = false,
        deleted = false,
        binary = false,
      }
    elseif current then
      if line:match("^new file mode ") then
        current.new_file = true
      elseif line:match("^deleted file mode ") then
        current.deleted = true
      elseif line:match("^Binary files ") then
        current.binary = true
      elseif line:match("^%-%-%- ") then
        current.old_path = path_from_header(line:sub(5))
      elseif line:match("^%+%+%+ ") then
        current.new_path = path_from_header(line:sub(5))
      elseif line:match("^@@ ") then
        local old_start, old_count, new_start, new_count = line:match("^@@ %-(%d+),?(%d*) %+(%d+),?(%d*) @@")
        if old_start then
          local old_lines = count(old_count)
          local new_lines = count(new_count)
          table.insert(current.ranges, {
            old_start = tonumber(old_start),
            old_count = old_lines,
            new_start = tonumber(new_start),
            new_count = new_lines,
          })
          current.added = current.added + new_lines
          current.removed = current.removed + old_lines
        end
      end
    end
  end
  finish()
  local by_path = {}
  for _, file in ipairs(files) do
    by_path[file.path] = file
  end
  for _, record in ipairs(metadata or {}) do
    local file = by_path[record.path]
    if file then
      file.status = record.status
      file.fallback_reason = record.fallback_reason
    else
      file = {
        path = record.path,
        ranges = {},
        added = 0,
        removed = 0,
        new_file = record.status == "A" or record.status == "?",
        deleted = record.status == "D",
        binary = record.fallback_reason == "binary file",
        metadata_only = true,
        fallback_reason = record.fallback_reason or "metadata-only change",
        status = record.status,
      }
      table.insert(files, file)
      by_path[file.path] = file
    end
  end
  return files
end

return M
