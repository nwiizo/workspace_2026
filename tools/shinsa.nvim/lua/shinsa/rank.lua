local M = {}

local function searchable(name)
  return type(name) == "string" and #name > 1 and name:match("^[%a_][%w_]*$") ~= nil
end

local function mentions(text, name)
  if not searchable(name) then
    return false
  end
  return (text or ""):find("%f[%w_]" .. vim.pesc(name) .. "%f[^%w_]") ~= nil
end

local function identifiers(text)
  local result = {}
  for name in (text or ""):gmatch("[%a_][%w_]*") do
    result[name] = true
  end
  return result
end

local function graph(items)
  local outgoing = {}
  local fan_in = {}
  local has_tests = false
  local by_name = {}
  for _, item in ipairs(items) do
    outgoing[item.id] = {}
    has_tests = has_tests or item.is_test
    if not item.file_level and searchable(item.name) then
      by_name[item.name] = by_name[item.name] or {}
      table.insert(by_name[item.name], item)
    end
  end
  for _, source in ipairs(items) do
    if not source.file_level then
      for name in pairs(identifiers(source.text)) do
        for _, target in ipairs(by_name[name] or {}) do
          if source.id ~= target.id then
            table.insert(outgoing[source.id], target.id)
            if not source.is_test then
              fan_in[target.id] = (fan_in[target.id] or 0) + 1
            end
          end
        end
      end
    end
  end

  local covered_by = {}
  for _, item in ipairs(items) do
    if item.is_test then
      local seen = {}
      local pending = { item.id }
      while #pending > 0 do
        local id = table.remove(pending)
        for _, target in ipairs(outgoing[id] or {}) do
          if not seen[target] then
            seen[target] = true
            covered_by[target] = covered_by[target] or {}
            covered_by[target][item.id] = true
            table.insert(pending, target)
          end
        end
      end
    end
  end
  return outgoing, fan_in, covered_by, has_tests
end

local function set_count(values)
  local total = 0
  for _ in pairs(values or {}) do
    total = total + 1
  end
  return total
end

local function score(item, fan_in, test_count, coverage_known)
  local value = 1
  local reasons = {}
  if item.deleted then
    value = value + 4
    table.insert(reasons, "deleted")
  elseif item.contract then
    value = value + 4
    table.insert(reasons, "contract")
  end
  if item.removal and not item.deleted then
    value = value + 3
    table.insert(reasons, "removed:" .. item.removed_lines)
  end
  if item.added then
    value = value + 2
    table.insert(reasons, "added")
  end
  if item.exported then
    value = value + 2
    table.insert(reasons, "public")
  end
  if fan_in > 0 then
    value = value + math.min(4, fan_in * 2)
    table.insert(reasons, "fan-in:" .. fan_in)
  end
  if coverage_known and not item.is_test then
    if test_count == 0 then
      value = value + 2
      table.insert(reasons, "no-test-path")
    else
      table.insert(reasons, "tests:" .. test_count)
    end
  end
  if item.changed_lines >= 20 then
    value = value + 2
    table.insert(reasons, "large-change")
  elseif item.changed_lines >= 5 then
    value = value + 1
    table.insert(reasons, "change:" .. item.changed_lines)
  end
  if item.end_line - item.line + 1 >= 100 then
    value = value + 1
    table.insert(reasons, "large-symbol")
  end
  if item.file_level then
    value = value + 1
    table.insert(reasons, "file-level")
  end
  return value, reasons
end

function M.apply(items)
  local outgoing, fan_in, covered_by, coverage_known = graph(items)
  for _, item in ipairs(items) do
    item.fan_in = fan_in[item.id] or 0
    item.test_count = coverage_known and set_count(covered_by[item.id]) or nil
    item.score, item.reasons = score(item, item.fan_in, item.test_count or 0, coverage_known)
    item.risk = item.score >= 7 and "high" or (item.score >= 4 and "medium" or "low")
    item.dependencies = outgoing[item.id] or {}
  end
  table.sort(items, function(a, b)
    if a.score ~= b.score then
      return a.score > b.score
    end
    if a.is_test ~= b.is_test then
      return not a.is_test
    end
    if a.path ~= b.path then
      return a.path < b.path
    end
    return a.line < b.line
  end)
  return items
end

M._mentions = mentions

return M
