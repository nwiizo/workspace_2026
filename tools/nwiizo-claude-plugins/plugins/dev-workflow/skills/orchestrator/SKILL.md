---
name: orchestrator
description: 複雑タスクを逐次ステップに分割し、各ステップ内で並列サブタスクを実行する。大きなタスクを依頼されたとき、または「分割して」「段階的に」と指示されたときに使用。
disable-model-invocation: true
---

# Orchestrator

複雑タスクを逐次ステップに分割し、各ステップ内で並列サブタスクを実行する。

## Process

1. **分析** — タスク全体のスコープ、依存関係、実行順序を把握
2. **計画** — 2-4の逐次ステップに分割。各ステップ内は並列可能
3. **実行** — ステップ順に実行。サブタスクは並列、要約(100-200語)を収集
4. **適応** — 各ステップ完了後に残りの計画を見直し。不要なステップはスキップ
5. **集約** — 結果を段階的に統合

## Adaptive Planning

```
Plan: Step 1 → 2 → 3 → 4

After Step 2 (no errors):  Skip 3 → Simplified 4
After Step 2 (critical issue):  Insert 2.5 → Modify 3
```

## Rules
- 最初に分析タスク1つで全体把握
- ステップ間は要約のみ渡す（全出力は渡さない）
- 各ステップ後に計画を再評価
