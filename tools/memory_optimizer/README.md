# Memory Optimizer

CLAUDE.md を最小限のスタートアップコンテキストにリファクタリングするツール。

## 使用タイミング

- CLAUDE.md が50行を超えたとき
- スタートアップが遅く感じるとき
- メモリ構造を再編成したいとき
- モノリシックなプロジェクト指示を分割したいとき

## コマンド

```
/optimize [ファイルパス]
```

例:
```
/optimize ../blogs/CLAUDE.md
```

## 抽出先の判断基準

| CLAUDE.md の内容 | 抽出先 | フロントマター |
|-----------------|--------|---------------|
| ファイル拡張子 (`.ts`, `.py`) やディレクトリ | `.claude/rules/{topic}.md` | `paths: {glob}` |
| 複数ステップのワークフロー (3+ ステップ) | `.claude/skills/{name}/SKILL.md` | `name:`, `description:` |
| ユーザートリガーのテンプレート | `.claude/commands/{name}.md` | `description:` |
| 限定ツールが必要な特殊タスク | `.claude/agents/{name}.md` | `name:`, `description:`, `tools:` |
| **すべての操作に必須** | CLAUDE.md に残す | — |

## ワークフロー

1. **分析**: CLAUDE.md を読み、行数をカウント
2. **分類**: 判断基準を各セクションに適用
3. **計画**: 抽出テーブルを提示し承認を得る
4. **抽出**: 適切なフロントマターでファイルを作成
5. **リファクタ**: CLAUDE.md を50行未満に削減
6. **報告**: before/after の行数を表示

## 目標

**CLAUDE.md: 50行未満、理想は20-30行**

## ファイル構成

| ファイル | 内容 |
|----------|------|
| `optimization-guide.md` | 詳細な最適化ガイド |
| `templates.md` | ファイルテンプレート集 |
| `examples.md` | 最適化の実例 |
