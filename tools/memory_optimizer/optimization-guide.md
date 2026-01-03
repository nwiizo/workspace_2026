# Memory Optimizer - 最適化ガイド

CLAUDE.md を最小限のスタートアップコンテキストにリファクタリングするための詳細ガイド。

## なぜ最適化が必要か

- **スタートアップトークンの削減**: 大きなCLAUDE.mdは毎回読み込まれる
- **Progressive Disclosure**: 必要なときに必要な情報だけを読み込む
- **メンテナンス性向上**: 分割することで管理しやすくなる

---

## 判断基準（Decision Table）

| CLAUDE.md の内容 | 抽出先 | フロントマター |
|-----------------|--------|---------------|
| ファイル拡張子 (`.ts`, `.rs`, `.go`, `.py`) やディレクトリ (`src/api/`) | `.claude/rules/{topic}.md` | `paths: {glob}` |
| 複数ステップのワークフロー (3+ ステップ) | `.claude/skills/{name}/SKILL.md` | `name:`, `description:` |
| ユーザートリガーのテンプレート | `.claude/commands/{name}.md` | `description:` |
| 限定ツールが必要な特殊タスク | `.claude/agents/{name}.md` | `name:`, `description:`, `allowed-tools:` |
| **すべての操作に必須** | CLAUDE.md に残す | — |

---

## ワークフロー

### 1. 分析 (Analyze)

```
CLAUDE.md を読み、以下を確認:
- 現在の行数
- セクションごとの内容
- 抽出可能な部分の特定
```

### 2. 分類 (Categorize)

判断基準を各セクションに適用し、抽出先を決定。

### 3. 計画 (Plan)

抽出テーブルを提示し、ユーザーの承認を得る:

```markdown
| Content        | Extract To                     | Type    | Trigger/Path        |
| -------------- | ------------------------------ | ------- | ------------------- |
| TS conventions | .claude/rules/typescript.md    | Rule    | `**/*.ts`           |
| Rust conventions | .claude/rules/rust.md        | Rule    | `**/*.rs`           |
| Go conventions | .claude/rules/go.md            | Rule    | `**/*.go`           |
| Python conventions | .claude/rules/python.md    | Rule    | `**/*.py`           |
| Deploy process | .claude/skills/deploy/SKILL.md | Skill   | "deploy", "release" |
| PR template    | .claude/commands/review.md     | Command | `/review`           |
| Security check | .claude/agents/security.md     | Agent   | security tasks      |
```

### 4. 抽出 (Extract)

適切なフロントマターでファイルを作成。

### 5. リファクタ (Refactor)

CLAUDE.md を50行未満に削減。

### 6. 報告 (Report)

```
Before: 150 lines
After:  35 lines
Reduction: 77%
```

---

## 抽出先の詳細

### Rules（ルール）

**用途**: 特定のファイルパターンに適用するルール

**配置**: `.claude/rules/{topic}.md`

**トリガー**: ファイルパスがマッチしたとき自動適用

**例**:
- TypeScript規約 → `.claude/rules/typescript.md`
- Rust規約 → `.claude/rules/rust.md`
- Go規約 → `.claude/rules/go.md`
- Python規約 → `.claude/rules/python.md`

### Skills（スキル）

**用途**: 複数ステップのワークフロー

**配置**: `.claude/skills/{name}/SKILL.md`

**トリガー**: ユーザーが `/skill-name` で呼び出し

**例**:
- デプロイプロセス → `.claude/skills/deploy/SKILL.md`
- コードレビュー → `.claude/skills/review/SKILL.md`

### Commands（コマンド）

**用途**: ユーザーが呼び出すテンプレート

**配置**: `.claude/commands/{name}.md`

**トリガー**: ユーザーが `/command-name` で呼び出し

**例**:
- PRテンプレート → `.claude/commands/pr.md`
- コミットメッセージ → `.claude/commands/commit.md`

### Agents（エージェント）

**用途**: 限定ツールが必要な特殊タスク

**配置**: `.claude/agents/{name}.md`

**トリガー**: Task ツールから呼び出し

**例**:
- セキュリティチェック → `.claude/agents/security.md`
- パフォーマンス分析 → `.claude/agents/performance.md`

---

## CLAUDE.md に残すもの

以下は抽出せず、CLAUDE.md に残す:

1. **プロジェクト概要** (1-2文)
2. **基本コマンド** (build, test, lint)
3. **重要な参照リンク**
4. **すべての操作に必須のルール**

---

## 最適化後の CLAUDE.md テンプレート

```markdown
# {Project Name}

{1-2 sentence overview}

## Commands

- Build: `{cmd}`
- Test: `{cmd}`
- Lint: `{cmd}`

## Key Rules

- {Essential rule 1}
- {Essential rule 2}

## References

- @README.md
- @docs/architecture.md
```

**目標: 50行未満、理想は20-30行**

---

## 検証チェックリスト

抽出後に確認:

- [ ] CLAUDE.md の行数が削減された
- [ ] 各ファイルに有効なYAMLフロントマターがある
- [ ] Rules に `paths:` がある
- [ ] Skills/Commands に `description:` がある
- [ ] description に "Use when" が含まれている

---

## description の書き方

**フォーマット**:
```
{Verb} {what}. Use when {trigger1}, {trigger2}, or {trigger3}.
```

**悪い例**:
```
Helps with code review
```

**良い例**:
```
Reviews code for security and performance issues. Use when reviewing PRs, checking code quality, or after major changes.
```
