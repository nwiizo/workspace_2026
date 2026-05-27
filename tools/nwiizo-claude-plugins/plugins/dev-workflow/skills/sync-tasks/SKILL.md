---
name: sync-tasks
description: Claude Code TaskList と vibe-ticket の同期。タスク管理の状態を双方向で同期する。
disable-model-invocation: true
argument-hint: "[--to-vibe|-v] [--from-vibe|-f] [--both|-b]"
---

# sync-tasks

Claude Code TaskList と vibe-ticket の同期。

## Usage: `/sync-tasks [option]`

| Option | Direction |
|--------|-----------|
| `--to-vibe` / `-v` | TaskList → vibe-ticket |
| `--from-vibe` / `-f` | vibe-ticket → TaskList |
| `--both` / `-b` | 双方向 |
| (なし) | 状態表示のみ |

## Status Mapping

| Claude Code | vibe-ticket |
|-------------|-------------|
| `pending` | `todo` |
| `in_progress` | `doing` |
| `completed` | `done` |

## Sync Logic

### To vibe-ticket
1. TaskList の各 Task から slug 生成（小文字、スペース→ハイフン）
2. vibe-ticket に存在しなければ `vibe-ticket_new` で作成
3. 存在すれば `vibe-ticket_edit` でステータス更新

### From vibe-ticket
1. `vibe-ticket_list --open` で pending/in_progress を取得
2. TaskList に存在しなければ TaskCreate
3. 存在すれば TaskUpdate でステータス更新

### Both
1. from-vibe → 2. to-vibe（競合時は最終更新が新しい方を優先）

## Metadata
- Claude Code Task: `metadata.vibe_task_id`
- vibe-ticket: `metadata.claude_task_id`
