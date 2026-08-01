local M = {}

local fallback_bases = { "origin/main", "main", "origin/master", "master" }

local function schedule(callback, ...)
  local args = { ... }
  vim.schedule(function()
    callback(unpack(args))
  end)
end

local function argv(prefix, suffix)
  local result = vim.deepcopy(prefix)
  vim.list_extend(result, suffix)
  return result
end

local function run(config, cwd, args, callback)
  local command = argv(config.git_command, args)
  local ok, result = pcall(vim.system, command, { cwd = cwd, text = true }, function(completed)
    schedule(callback, completed)
  end)
  if not ok then
    schedule(callback, { code = -1, stdout = "", stderr = tostring(result) })
  end
end

local function run_limited(config, cwd, args, limit, callback)
  local command = argv(config.git_command, args)
  local chunks = {}
  local size = 0
  local truncated = false
  local stream_error
  local ok, result = pcall(vim.system, command, {
    cwd = cwd,
    text = true,
    stdout = function(err, data)
      if err then
        stream_error = err
        return
      end
      if not data or truncated then
        return
      end
      size = size + #data
      if size > limit then
        truncated = true
        chunks = {}
        return
      end
      table.insert(chunks, data)
    end,
  }, function(completed)
    completed.stdout = table.concat(chunks)
    completed.truncated = truncated
    if stream_error then
      completed.code = -1
      completed.stderr = stream_error
    end
    schedule(callback, completed)
  end)
  if not ok then
    schedule(callback, { code = -1, stdout = "", stderr = tostring(result), truncated = false })
  end
end

local function command_error(label, result)
  local detail = M.trim(result.stderr)
  if detail == "" then
    detail = "exit code " .. tostring(result.code)
  end
  return string.format("%s failed: %s", label, detail)
end

function M.trim(value)
  return (value or ""):gsub("^%s+", ""):gsub("%s+$", "")
end

function M.nul_paths(value)
  local paths = {}
  for path in (value or ""):gmatch("([^%z]+)%z") do
    table.insert(paths, path)
  end
  return paths
end

function M.name_status(value)
  local fields = M.nul_paths(value)
  local records = {}
  local index = 1
  while fields[index] do
    local status = fields[index]
    local path = fields[index + 1]
    if not path then
      break
    end
    if status:match("^[RC]") then
      path = fields[index + 2]
      index = index + 3
    else
      index = index + 2
    end
    if path then
      table.insert(records, { path = path, status = status:sub(1, 1) })
    end
  end
  return records
end

function M.root(bufnr)
  bufnr = bufnr or 0
  local name = vim.api.nvim_buf_get_name(bufnr)
  local path = name ~= "" and vim.fs.normalize(name) or vim.fn.getcwd()
  local root = vim.fs.root(path, { ".git" })
  if not root then
    return nil, "current buffer is not inside a Git repository"
  end
  return vim.fs.normalize(root)
end

function M.join(root, relative)
  if type(relative) ~= "string" or relative == "" or relative:sub(1, 1) == "/" then
    return nil
  end
  local normalized_root = vim.fs.normalize(root)
  local path = vim.fs.normalize(normalized_root .. "/" .. relative)
  if path ~= normalized_root and path:sub(1, #normalized_root + 1) ~= normalized_root .. "/" then
    return nil
  end
  return path
end

local function configured_base(config, cwd)
  if type(config.base) ~= "function" then
    return config.base
  end
  local ok, value = pcall(config.base, { cwd = cwd })
  if not ok then
    return nil, "base resolver failed: " .. tostring(value)
  end
  if value ~= nil and (type(value) ~= "string" or value == "") then
    return nil, "base resolver must return a non-empty string or nil"
  end
  return value
end

local function find_fallback(config, cwd, index, callback)
  local candidate = fallback_bases[index]
  if not candidate then
    callback(nil, "could not resolve a base branch; pass one to :Shinsa or configure base")
    return
  end
  run(config, cwd, { "rev-parse", "--verify", "--quiet", candidate .. "^{commit}" }, function(result)
    if result.code == 0 then
      callback(candidate)
    else
      find_fallback(config, cwd, index + 1, callback)
    end
  end)
end

local function resolve_base(config, cwd, explicit, callback)
  if explicit and explicit ~= "" then
    callback(explicit)
    return
  end
  local selected, err = configured_base(config, cwd)
  if err then
    callback(nil, err)
    return
  end
  if selected then
    callback(selected)
    return
  end
  run(config, cwd, { "symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD" }, function(result)
    local detected = M.trim(result.stdout)
    if result.code == 0 and detected ~= "" then
      run(config, cwd, { "rev-parse", "--verify", "--quiet", detected .. "^{commit}" }, function(verify)
        if verify.code == 0 then
          callback(detected)
        else
          find_fallback(config, cwd, 1, callback)
        end
      end)
    else
      find_fallback(config, cwd, 1, callback)
    end
  end)
end

local function resolve_merge_base(config, cwd, base, callback)
  run(config, cwd, { "merge-base", base, "HEAD" }, function(result)
    local merge_base = M.trim(result.stdout)
    if result.code ~= 0 or merge_base == "" then
      callback(nil, command_error("git merge-base", result))
    else
      callback(merge_base)
    end
  end)
end

local function untracked_patch(config, cwd, path, patch_budget, scan_budget)
  if patch_budget <= 0 then
    return nil, "diff exceeds analysis limit", 0
  end
  local absolute = M.join(cwd, path)
  if not absolute then
    return nil, "unsafe path", 0
  end
  local stat = vim.uv.fs_lstat(absolute)
  if not stat then
    return nil, "file is unavailable", 0
  end
  if stat.type == "link" then
    return nil, "symbolic link", 0
  end
  if stat.type ~= "file" then
    return nil, "unsupported file type", 0
  end
  if stat.size > config.max_file_bytes then
    return nil, string.format("file exceeds max_file_bytes (%d bytes)", stat.size), 0
  end
  if stat.size > scan_budget then
    return nil, "untracked scan budget exceeded", 0
  end
  if path:find("[\r\n]") then
    return nil, "path contains a newline", 0
  end
  local handle, err = io.open(absolute, "rb")
  if not handle then
    return nil, err, 0
  end
  local content = handle:read("*a")
  handle:close()
  if content:find("\0", 1, true) then
    return nil, "binary file", stat.size
  end
  local lines = content == "" and {} or vim.split(content, "\n", { plain = true })
  if content:sub(-1) == "\n" then
    table.remove(lines)
  end
  local patch = {
    string.format("diff --git a/%s b/%s", path, path),
    "new file mode 100644",
    "--- /dev/null",
    "+++ b/" .. path,
  }
  if #lines > 0 then
    table.insert(patch, string.format("@@ -0,0 +1,%d @@", #lines))
    for _, line in ipairs(lines) do
      table.insert(patch, "+" .. line)
    end
  end
  local text = table.concat(patch, "\n") .. "\n"
  if #text > patch_budget then
    return nil, "diff exceeds analysis limit", stat.size
  end
  return text, nil, stat.size
end

local function collect_diff(config, cwd, merge_base, callback)
  local tracked
  local status
  local untracked = config.include_untracked and nil or { code = 0, stdout = "", stderr = "" }
  local limit = math.max(config.max_file_bytes, config.max_file_bytes * 8)
  local finishing = false

  local function finish_if_ready()
    if finishing or not tracked or not status or not untracked then
      return
    end
    finishing = true
    if tracked.code ~= 0 then
      callback(nil, command_error("git diff", tracked))
      return
    end
    if untracked.code ~= 0 then
      callback(nil, command_error("git ls-files", untracked))
      return
    end
    if status.code ~= 0 then
      callback(nil, command_error("git diff --name-status", status))
      return
    end

    local records = M.name_status(status.stdout)
    if tracked.truncated then
      for _, record in ipairs(records) do
        record.fallback_reason = "diff exceeds analysis limit"
      end
    end
    local parts = { tracked.stdout or "" }
    local patch_budget = math.max(0, limit - #(tracked.stdout or ""))
    local scan_budget = limit
    local untracked_paths = M.nul_paths(untracked.stdout)
    local index = 1

    local function process_batch()
      local batch_end = math.min(#untracked_paths, index + 31)
      while index <= batch_end do
        local path = untracked_paths[index]
        local record = { path = path, status = "?" }
        local patch, reason, scanned = untracked_patch(config, cwd, path, patch_budget, scan_budget)
        scan_budget = scan_budget - scanned
        if patch then
          table.insert(parts, patch)
          patch_budget = patch_budget - #patch
        else
          record.fallback_reason = reason
        end
        table.insert(records, record)
        index = index + 1
      end
      if index <= #untracked_paths then
        vim.schedule(process_batch)
      else
        callback(table.concat(parts, "\n"), nil, records)
      end
    end

    process_batch()
  end

  run_limited(
    config,
    cwd,
    {
      "-c",
      "core.quotePath=false",
      "diff",
      "--no-ext-diff",
      "--no-textconv",
      "--no-color",
      "--no-renames",
      "--unified=0",
      merge_base,
      "--",
    },
    limit,
    function(result)
      tracked = result
      finish_if_ready()
    end
  )
  run(config, cwd, {
    "-c",
    "core.quotePath=false",
    "diff",
    "--name-status",
    "-z",
    "--no-renames",
    merge_base,
    "--",
  }, function(result)
    status = result
    finish_if_ready()
  end)
  if config.include_untracked then
    run(config, cwd, { "ls-files", "--others", "--exclude-standard", "-z" }, function(result)
      untracked = result
      finish_if_ready()
    end)
  else
    finish_if_ready()
  end
end

function M.snapshot(config, cwd, explicit_base, callback, on_progress)
  on_progress = on_progress or function() end
  on_progress("Resolving base")
  resolve_base(config, cwd, explicit_base, function(base, base_err)
    if not base then
      callback(nil, base_err)
      return
    end
    on_progress("Finding merge base")
    resolve_merge_base(config, cwd, base, function(merge_base, merge_err)
      if not merge_base then
        callback(nil, merge_err)
        return
      end
      on_progress("Collecting working-tree diff")
      collect_diff(config, cwd, merge_base, function(diff, diff_err, paths)
        if not diff then
          callback(nil, diff_err)
          return
        end
        callback({ root = cwd, base = base, merge_base = merge_base, diff = diff, paths = paths })
      end)
    end)
  end)
end

M._fallback_bases = fallback_bases

return M
