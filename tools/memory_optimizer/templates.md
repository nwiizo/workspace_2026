# Memory Optimizer - テンプレート集

各ファイルタイプのテンプレート。

---

## Rule テンプレート

```yaml
---
paths: src/**/*.ts  # or *.rs, *.go, *.py
---
# {Topic}

## Rules

- {Rule 1}
- {Rule 2}

## Examples

{Examples if needed}
```

**必須フロントマター**: `paths:`

**paths の書き方**:
- 単一: `paths: src/**/*.ts`
- 複数: `paths: ["**/*.ts", "**/*.rs", "**/*.go", "**/*.py"]`

---

## Skill テンプレート

```yaml
---
name: {kebab-case-name}
description: {What it does}. Use when {trigger1}, {trigger2}, or {trigger3}.
allowed-tools: Read, Grep, Glob
---
# {Name}

{Overview}

## Steps

1. {Step 1}
2. {Step 2}
3. {Step 3}

## Output

{Expected output format}
```

**必須フロントマター**: `name:`, `description:`

**オプションフロントマター**:
- `allowed-tools:` - 許可するツール（カンマ区切り）
- `model:` - 使用するモデルの指定

---

## Command テンプレート

```yaml
---
description: {Brief description} (project)
---
# {Command Name}

{Instructions}

## Usage

```
/{command-name} [arguments]
```

## Arguments

- `$ARGUMENTS` - {description}

---

ARGUMENTS: $ARGUMENTS

{Actual prompt template}
```

**必須フロントマター**: `description:`

---

## Agent テンプレート

```yaml
---
name: {name}
description: {What it does}. Use proactively for {triggers}.
allowed-tools: Read, Grep, Glob
skills: skill-name-1, skill-name-2
---
# {Name}

{Instructions}

## When to Use

- {Trigger 1}
- {Trigger 2}

## Workflow

1. {Step 1}
2. {Step 2}

## Output

{Expected output}
```

**必須フロントマター**: `name:`, `description:`

**オプションフロントマター**:
- `allowed-tools:` - 許可するツール（カンマ区切り）
- `skills:` - アクセスを許可するスキル（カンマ区切り）

**利用可能なツール**:
- `Read` - ファイル読み込み
- `Write` - ファイル書き込み
- `Edit` - ファイル編集
- `Glob` - ファイル検索
- `Grep` - 内容検索
- `Bash` - コマンド実行

---

## CLAUDE.md テンプレート（最適化後）

```markdown
# {Project Name}

{1-2 sentence overview}

## Commands

```sh
{build command}
{test command}
{lint command}
```

## Key Rules

- {Rule 1}
- {Rule 2}

## References

- @README.md
- @docs/architecture.md

## .claude Structure

- `rules/` - Path-specific rules
- `commands/` - User commands
- `skills/` - Multi-step workflows
- `agents/` - Specialized tasks
```
