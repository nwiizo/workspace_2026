# nwiizo-claude-plugins

nwiizo が実戦投入してきた Claude Code 資産（`~/.claude` の `home-*` skills / agents）を、
**配布・インストール可能なプラグイン**に再編成したマーケットプレイス。
ローカルに散らばっていた個人ツールキットを、テーマ単位で独立インストールできる 7 プラグインに整理した。

## インストール

```sh
# Claude Code 内で:
/plugin marketplace add nwiizo/claude-plugins     # GitHub に push 後
# またはローカルパス指定:
/plugin marketplace add /path/to/workspace_2026/tools/nwiizo-claude-plugins

# 必要なプラグインだけ入れる
/plugin install nwiizo-review@nwiizo-claude-plugins
```

## プラグイン一覧

| Plugin | 内容 | 同梱 |
|--------|------|------|
| `nwiizo-review` | 並列レビュー・ワークフロー（品質 / 独立観点 / 可読性の多角レビュー） | agents: code-reviewer, codex-reviewer, simplify-reviewer / skills: self-review, fix-review-comments, rust-code-review, design-review, cli-ux-review, proactive-suggestions |
| `engineering-discipline` | Karpathy 由来の実装前ゲート（仮定の明示 / 過剰実装防止 / 外科的編集 / 検証可能ゴール） | skills: karpathy-guidelines |
| `finops-investigation` | AWS / GCP のコスト調査・削減分析の実務知見 | skills: aws-finops-investigation, gcp-finops-investigation |
| `jj-workflow` | Jujutsu (jj) の並列 workspace・commit サイクル・履歴監査 | skills: jj-agent-spawn, jj-commit-cycle / agent: jj-reviewer |
| `prompt-engineering` | agent 向け指示の両面評価と反復改善 | skills: empirical-prompt-tuning, prompt-review |
| `dev-workflow` | ツール開発の分割・反復・OSS 検証・タスク同期 | skills: orchestrator, iterative-refinement, validate-on-oss, sync-tasks / agents: planner, memory-optimizer |
| `authoring` | Marp スライド・技術書翻訳の品質基準 | skills: marp-slide-editing, translation-quality |

## 構成

```
nwiizo-claude-plugins/
├── .claude-plugin/marketplace.json    # マーケットプレイス定義（7 プラグイン）
└── plugins/<plugin>/
    ├── .claude-plugin/plugin.json     # プラグイン metadata
    ├── skills/<skill>/SKILL.md        # スキル本体（参照ファイル同梱）
    └── agents/<agent>.md              # エージェント定義
```

## `~/.claude` の `home-*` との関係

このマーケットプレイスは `~/.claude/skills/home-*` / `~/.claude/agents/home-*` の**スナップショット**。
配布用に `home-` プレフィックスを除去し、プラグイン名で namespace している。
ローカルの `home-*` が単一情報源であり、本リポジトリは再編成・共有用の複製。
内容を更新したら本マーケットプレイスにも反映する。

## 公式マーケットプレイスとの併用（精選有効化）

この個人マーケットと併せて、`anthropics/claude-plugins-official`（203 プラグイン）からは
**既存資産と重複しないものだけ**を精選して有効化している（quality over quantity）。
設定の単一情報源は `~/.claude/settings.json` の `enabledPlugins`。

### 有効化（公式から）

| Plugin | 用途 | 採用理由 |
|--------|------|----------|
| `rust-analyzer-lsp` / `gopls-lsp` / `pyright-lsp` / `typescript-lsp` / `lua-lsp` | 各言語 LSP | 主要言語を網羅 |
| `frontend-design` | 高品質フロントエンド生成 | — |
| `slack` | Slack 連携 | — |
| `skill-creator` | スキルの新規作成・改善・性能計測 | kata-eval 等、スキル著作が多い |
| `plugin-dev` | プラグイン開発 | 本マーケット / `3-shake/claude-code-plugins` を保守 |
| `mcp-server-dev` | MCP サーバー設計・実装 | remote-mcp-devkit / kuroko 等 |
| `agent-sdk-dev` | Claude Agent SDK 開発 | architecture-modernization-agents 等 |
| `context7` | バージョン固有ドキュメント即時参照 | 汎用的に有用、低コスト |

### 見送り（本マーケットの自前プラグインと重複）

| 公式 Plugin | 重複する本マーケットのプラグイン / 既存資産 |
|-------------|---------------------------------------------|
| `code-review` / `code-simplifier` / `pr-review-toolkit` | `nwiizo-review`（code-reviewer / simplify-reviewer / codex-reviewer） |
| `claude-md-management` | `dev-workflow`（memory-optimizer） |
| `session-report` | `usage-analytics` plugin + `nippo` skill |
| `commit-commands` | `jj-workflow` + Conventional Commits 運用 |
| `karpathy-skills`（`multica-ai/andrej-karpathy-skills`） | `engineering-discipline`（karpathy-guidelines）。同じ 4 原則を内製済み。上流 `EXAMPLES.md` は before/after 例の参照元として有用 |
| `learning-output-style` / `explanatory-output-style` / `math-olympiad` | ユースケース外 |

### 検討候補（必要時のみ有効化）

- セキュリティ系（CTF / `vigil` と相性）: `security-guidance`（編集時警告 hook）, `semgrep`（SAST）
- クラウド/IaC: `terraform`, `aws-core`, `github` 等はプロジェクト単位で都度有効化（グローバル常時有効は避ける）

### 再現手順（別マシン）

```sh
# 公式マーケットプレイス追加（Claude Code 内）
/plugin marketplace add anthropics/claude-plugins-official
# 精選プラグインを /plugin install、または settings.json の enabledPlugins に追記:
#   agent-sdk-dev / context7 / mcp-server-dev / plugin-dev / skill-creator / typescript-lsp
#   （いずれも @claude-plugins-official）
# 更新: /plugin marketplace update claude-plugins-official
```

## ライセンス

Friend License (MIT-equivalent)
