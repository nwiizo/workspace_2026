local M = {}

local scope
local entries = {}

local function inline_code(value)
  return tostring(value or ""):gsub("%c", " "):gsub("`", "'")
end

function M.reconcile(next_scope, items)
  if scope ~= next_scope then
    scope = next_scope
    entries = {}
  end
  local next_entries = {}
  for _, item in ipairs(items) do
    next_entries[item.id] = entries[item.id] or { reviewed = false, concern = nil }
  end
  entries = next_entries
end

function M.get(id)
  return entries[id]
end

function M.toggle_reviewed(id)
  local entry = entries[id]
  if not entry then
    return nil
  end
  entry.reviewed = not entry.reviewed
  return entry.reviewed
end

function M.set_concern(id, text)
  local entry = entries[id]
  if not entry then
    return nil, "unknown review item"
  end
  text = vim.trim(text or "")
  if text == "" then
    return nil, "concern is empty"
  end
  entry.concern = text
  return entry
end

function M.clear_concern(id)
  local entry = entries[id]
  if entry then
    entry.concern = nil
  end
end

function M.decorate(items)
  local result = {}
  for _, item in ipairs(items) do
    local copy = vim.deepcopy(item)
    local entry = entries[item.id] or { reviewed = false }
    copy.reviewed = entry.reviewed
    copy.concern = entry.concern
    table.insert(result, copy)
  end
  return result
end

function M.summary(items)
  local result = { total = #items, reviewed = 0, concerns = 0 }
  for _, item in ipairs(items) do
    local entry = entries[item.id]
    if entry and entry.reviewed then
      result.reviewed = result.reviewed + 1
    end
    if entry and entry.concern then
      result.concerns = result.concerns + 1
    end
  end
  return result
end

function M.markdown(context, items)
  local summary = M.summary(items)
  if summary.concerns == 0 then
    return nil, "no review concerns to export"
  end
  local lines = {
    "## Shinsa follow-ups",
    "",
    string.format("Reviewed: %d/%d items", summary.reviewed, summary.total),
    string.format("Compared with: `%s`", inline_code(context.base)),
    "",
    "Please address the following concerns and verify the surrounding behavior:",
    "",
  }
  local number = 0
  for _, item in ipairs(items) do
    local entry = entries[item.id]
    if entry and entry.concern then
      number = number + 1
      table.insert(
        lines,
        string.format("%d. `%s:%d` — `%s`", number, inline_code(item.path), item.line, inline_code(item.signature))
      )
      table.insert(lines, "   " .. entry.concern)
    end
  end
  return table.concat(lines, "\n")
end

function M.reset()
  for _, entry in pairs(entries) do
    entry.reviewed = false
    entry.concern = nil
  end
end

function M._reset()
  scope = nil
  entries = {}
end

return M
