local M = { total = 0, failed = 0 }

local function inspect(value)
  return vim.inspect(value)
end

function M.eq(expected, actual, message)
  if not vim.deep_equal(expected, actual) then
    error((message or "values differ") .. "\nexpected: " .. inspect(expected) .. "\nactual:   " .. inspect(actual), 2)
  end
end

function M.truthy(value, message)
  if not value then
    error(message or "expected a truthy value", 2)
  end
end

function M.contains(value, expected, message)
  if type(value) ~= "string" or not value:find(expected, 1, true) then
    error((message or "string does not contain expected text") .. "\nvalue: " .. inspect(value), 2)
  end
end

function M.raises(expected, callback)
  local ok, err = pcall(callback)
  if ok then
    error("expected callback to raise", 2)
  end
  M.contains(tostring(err), expected)
end

function M.test(name, callback)
  M.total = M.total + 1
  local ok, err = xpcall(callback, debug.traceback)
  if ok then
    io.stdout:write("ok - " .. name .. "\n")
  else
    M.failed = M.failed + 1
    io.stderr:write("not ok - " .. name .. "\n" .. tostring(err) .. "\n")
  end
end

function M.finish()
  io.stdout:write(string.format("\n%d tests, %d failures\n", M.total, M.failed))
  if M.failed > 0 then
    vim.cmd("cquit 1")
  else
    vim.cmd("qa!")
  end
end

return M
