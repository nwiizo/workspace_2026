# CLAUDE.md - Tools

## 概要

汎用ツール・評価プロンプト・検証用コード

## ディレクトリ構成

| ディレクトリ | 内容 |
|-------------|------|
| `blog_evaluation/` | ブログ記事評価プロンプト v2.3 |
| `memory_optimizer/` | CLAUDE.md 最適化ツール |
| `rust-sql-anti-pattern/` | SQLアンチパターン検証コード（Rust） |
| `vigil/` | セキュリティ監査ツールキット（Opus 4.6 セマンティック分析、OWASP Top 10、Web シェル検出） |
| `wardley-quest/` | 戦略シミュレーションRPG（Wardley Mapping × DDD × Team Topologies、アドオン含む） |
| `architecture-modernization-agents/` | アーキテクチャモダナイゼーション Claude Code サブエージェント群（11体、戦略〜移行まで） |
| `rustlean/` | MIRベース最適化支援ツール（Clone/Copy削減、アロケーション検出、構造体レイアウト分析） |
| `rbp-lint/` | rust-best-practices ルールを rowan/ra_ap_syntax で検査する Rust リンター（unwrap/expect/panic/dead_code/Arc::clone/tracing/SAFETY/秘密鍵を検出） |
| `kata-eval/` | Skill 評価 CLI（waxa/waza スキーマ互換）。`claude -p` を bias 抑制で実行し、text / code (rhai) / self-report / llm の 4 grader で採点。`iterate` で ledger 付き RED/GREEN/REFACTOR、`compare` で複数モデル比較、`variant` でスキル A/B 検証 |
| `minagine-bikou-extension/` | Minagine 勤怠表 (`work.minagine.net`) の備考欄に「自己啓発・研鑽」を一括入力する Chrome 拡張 (MV3)。MUI ダイアログを順次開いて React 互換 setter で値投入 |
| `herdr.nvim/` | Herdr 上で永続実行する Codex / Claude Code を Neovim から一覧・状態監視・通知・起動・attach・context 送信する依存なし Lua プラグイン。attention-first の herd board と `:checkhealth herdr` を提供 |
| `nwiizo-claude-plugins/` | 個人用 Claude Code プラグインマーケットプレイス。`~/.claude` の `home-*` skills/agents を配布可能な 7 プラグインに再編成（nwiizo-review / engineering-discipline / finops-investigation / jj-workflow / prompt-engineering / dev-workflow / authoring）。`home-` プレフィックス除去でプラグイン名 namespace 化、`.claude-plugin/marketplace.json` 準拠。19 skills + 6 agents 同梱。README に `anthropics/claude-plugins-official` の精選有効化リスト・重複分析・再現手順も統合 |
| `cargo-boundary/` | アーキテクチャ境界診断 cargo サブコマンド (Rust, ra_ap_syntax CST)。layer 違反・internal 越境・pub 漏れ・禁止 import を severity 4 段階 + Grade A–F でスコア化。boundary.toml + heuristic 層推定、`--baseline` ratchet gate、`--ai`、blind spot manifest、`// boundary-allow:` suppress。ローカルツール (publish = false)。設計と Wave 記録は `.claude/docs/cargo-design-tools-plan.md` |
| `cargo-error-map/` | エラー設計診断 cargo サブコマンド (Rust, ra_ap_syntax CST)。anyhow 漏れ・error enum 肥大化・context 欠落・境界外 panic・dyn Error 露出を検出し伝播グラフ (`--graph[=dot]`) を出力。repo 相対の安定キーで `--baseline` ratchet、`--ai`、blind spot manifest、`// error-map-allow:` suppress。ローカルツール (publish = false) |
| `design-gate-core/` | 設計診断ツール群 (cargo-boundary / error-map / async-smell / trait-surface / feature-doctor) の共通 crate。Severity/Grade、IssueKey diff、baseline worktree (Drop ガード)、gate 判定 + check: PASS/FAIL、blind spot manifest、.gitignore 尊重 walker、suppress 注釈解決、cargo サブコマンド引数吸収、exit code 規約 (1=実行時/2=usage) を提供 |
| `cargo-async-smell/` | async 運用事故リスク検出 cargo サブコマンド (Rust, ra_ap_syntax CST + design-gate-core)。guard-across-await (tokio async lock は除外)・blocking-in-async・unbounded-spawn・detached-task・missing-timeout を検出。use alias 解決、`rel_path:Type::method` 形式の安定キー、`// async-smell-allow:` suppress |
| `cargo-trait-surface/` | trait 抽象境界の品質診断 cargo サブコマンド (Rust, ra_ap_syntax CST + design-gate-core)。oversized-trait・single/zero-impl-abstraction (Low 起点 + trait-surface.toml の intent 宣言)・object-safety-risk (async_trait / where Self: Sized 考慮)・broad-blanket-impl・unmockable-boundary (シグネチャのみ走査) を検出。`--trait <Name>` で単一 trait 詳細 |
| `cargo-feature-doctor/` | Cargo feature 事故のビルドなし静的検出 cargo サブコマンド (Rust, cargo_metadata + ra_ap_syntax + design-gate-core)。default-leak (依存 feature 再帰展開)・exclusive-undeclared (両方向判定)・untested-cfg-path (default/all-features 2 点判定)・optional-dep-exposure (feature 別名対応)・non-additive-feature を検出。`--matrix` / `--suggest-hack` (cargo-hack コマンド生成、否定極性対応) |
| `cargo-test-gap/` | テストが薄い危険箇所のランキング cargo サブコマンド (Rust, ra_ap_syntax + design-gate-core)。risk = churn (repo 全体 1 回の git log、ファイル粒度) × 複雑度 × 露出 ÷ (coverage + 1)。coverage は `--llvm-cov` JSON 取り込みか tests/ 含む到達性近似。`--top N` ランキング + 各軸の素点併記、Grade は High/Critical 比率ベースの相対評価 |
| `cargo-api-drift/` | public API 差分の SemVer リスク分類 cargo サブコマンド (Rust, ra_ap_syntax + design-gate-core)。`--against <ref>` の git diff から breaking / risky / safe を分類（struct フィールド・ジェネリクス・re-export・const/static・bound 強化/緩和対応、cosmetic 属性は正規化で除外）。`--changelog` で Keep a Changelog 断片生成。strict semver 監査は cargo-semver-checks に譲る棲み分けを blind spot 宣言 |
| `kuroko/` | 軽量 AWS サービスエミュレータ (Rust, axum 0.8, MIT)。port 4566・認証不要・単一バイナリ。AWS JSON 1.0/1.1, Query, REST, Smithy RPC v2 CBOR の 4 プロトコル dispatcher。**76 サービス全実装** (~50 はフル CRUD、14 は `resource_stub` パターンで最小制御プレーン)。AWS SDK for Rust + 公式仕様準拠で 341 テスト疎通検証済み。`/_kuroko/reset` / `/_kuroko/health` / `/_kuroko/services` / `/_kuroko/info` の introspection endpoint と KUROKO_DATA_DIR ベースの JSON snapshot 永続化対応 |

## コマンド

各ディレクトリで利用可能:

```
# blog_evaluation/
/blog-evaluate [ファイルパス]

# memory_optimizer/
/optimize [ファイルパス]
```

## ライセンス

Friend License (MIT-equivalent)
