# CLAUDE.md - Architecture Modernization Agents

## 概要

アーキテクチャモダナイゼーションの知識体系をベースとした Claude Code サブエージェント群。各エージェントはコードベースを実際にスキャンし、専門フレームワークを適用して実務的な分析・提案を行う。

## 設計原則

- **コード分析起点**: Glob/Grep/Bash でコードを実際に読み、具体的なシグナルからフレームワークを適用する
- **Opus のセマンティック分析**: 静的解析ツールでは検出できないドメイン知識・暗黙の結合・ビジネス意図を推論する
- **フレームワーク駆動**: Modernization Strategy Selector（MSS）、Core Domain Chart、Independent Service Heuristics（ISH）、Bounded Context Canvas（BCC）、Pain 公式などの評価フレームワークで構造化されたアウトプットを出す

## コマンド

| コマンド | 説明 | 実行エージェント |
|---------|------|---------------|
| `/modernize-assess` | クイック評価（3エージェント） | code-analyzer → debt-assessor → strategist |
| `/modernize-domain` | ドメイン境界分析（3エージェント） | domain-discovery → capability-mapper → context-designer |
| `/modernize-migration` | 移行計画策定（2エージェント） | code-analyzer → migration-planner |
| `/modernize-full` | フル分析（10エージェント、5フェーズ） | 全エージェント統合実行 |

## エージェント一覧

| エージェント | 主要フレームワーク | 用途 |
|------------|-----------------|------|
| `modernization-strategist` | MSS（9戦略）、Nail it then scale it | 全体戦略・ロードマップ策定 |
| `wardley-mapping-analyst` | 進化段階推定、Build vs Buy | バリューチェーン分析・戦略的不整合検出 |
| `domain-discovery-facilitator` | 6つの境界ヒューリスティック、ピボタルイベント | ドメインイベント抽出・サブドメイン境界提案 |
| `business-capability-mapper` | ISH（10問）、Product Taxonomy | 独立サービス候補評価 |
| `technical-debt-assessor` | Core Domain Chart（8パターン）、7種類の複雑さ | 技術的負債の定量評価・投資判断 |
| `bounded-context-designer` | BCC（11セクション）、Pain 公式 | Context 境界設計・結合分析 |
| `team-topologies-advisor` | Independent Value Stream（IVS）、Architecture Modernization Enabling Team（AMET）、Inverse Conway | チーム構造評価・組織設計 |
| `platform-engineering-consultant` | 成熟度マトリックス（6カテゴリ×4レベル）、Thinnest Viable Platform（TVP） | プラットフォーム成熟度評価 |
| `strangler-fig-migration-planner` | 25パターンカタログ、Feature Parity Trap | 移行パターン選定・データ同期戦略 |
| `legacy-code-analyzer` | ホットスポット × 複雑性マトリックス | コード分析・サービス分離候補特定 |
| `modernization-orchestrator` | 整合性検証、3実行モード | マルチエージェント統合・総合評価 |

## 使用方法

```bash
# プロジェクトローカルで利用
cp agents/*.md .claude/agents/
cp -r .claude/commands/ .claude/commands/

# グローバルで利用
cp agents/*.md ~/.claude/agents/
```

## コンテンツのみ

ビルド・テスト不要。マークダウンファイルのみで構成。
