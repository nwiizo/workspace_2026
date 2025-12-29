---
description: Optimize - CLAUDE.md を最適化 (project)
---

# Optimize - CLAUDE.md 最適化

指定された CLAUDE.md を分析し、最小限のスタートアップコンテキストにリファクタリングします。

## 使用方法

```
/optimize [ファイルパス]
```

例:
```
/optimize ../blogs/CLAUDE.md
/optimize ../../.claude/CLAUDE.md
```

---

## 実行内容

`optimization-guide.md` を読み込み、以下のワークフローを実行してください。

### ワークフロー

1. **分析**: 指定された CLAUDE.md を読み、行数をカウント
2. **分類**: 判断基準を各セクションに適用
3. **計画**: 抽出テーブルを提示し承認を得る
4. **抽出**: 適切なフロントマターでファイルを作成
5. **リファクタ**: CLAUDE.md を50行未満に削減
6. **報告**: before/after の行数を表示

---

## 判断基準

| CLAUDE.md の内容 | 抽出先 | フロントマター |
|-----------------|--------|---------------|
| ファイル拡張子・ディレクトリ | `.claude/rules/{topic}.md` | `paths: {glob}` |
| 複数ステップのワークフロー | `.claude/skills/{name}/SKILL.md` | `name:`, `description:` |
| ユーザートリガーのテンプレート | `.claude/commands/{name}.md` | `description:` |
| 限定ツールが必要な特殊タスク | `.claude/agents/{name}.md` | `name:`, `description:`, `allowed-tools:` |
| **すべての操作に必須** | CLAUDE.md に残す | — |

---

## 出力形式

### 抽出計画

```markdown
## 抽出計画

| Content | Extract To | Type | Trigger/Path |
|---------|------------|------|--------------|
| ... | ... | ... | ... |
```

### 最終報告

```markdown
## 最適化結果

- **Before**: XXX lines
- **After**: XXX lines
- **Reduction**: XX%

### 作成されたファイル

- `.claude/rules/xxx.md`
- `.claude/commands/xxx.md`
- ...
```

---

## 目標

**CLAUDE.md: 50行未満、理想は20-30行**

---

ARGUMENTS: $ARGUMENTS

ユーザーが指定したファイルを読み、`optimization-guide.md` の基準に従って最適化を実行してください。

**重要**: 抽出計画を提示し、ユーザーの承認を得てから実行してください。
