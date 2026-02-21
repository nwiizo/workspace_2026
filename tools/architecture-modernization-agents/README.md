# Architecture Modernization Agents

レガシーシステムのモダナイゼーションを支援する Claude Code サブエージェント群。コードベースを実際にスキャンし、専門フレームワークを適用して実務的な分析・提案を行う。

## 特徴

- **コード分析起点**: Glob/Grep/Bash でコードを実際に読み、具体的なシグナルからフレームワークを適用
- **Opus のセマンティック分析**: 静的解析ツールでは検出できないドメイン知識・暗黙の結合・ビジネス意図を推論
- **フレームワーク駆動**: Modernization Strategy Selector（MSS）、Core Domain Chart、Independent Service Heuristics（ISH）、Bounded Context Canvas（BCC）、Pain 公式などで構造化されたアウトプット

## コマンド

分析対象のリポジトリにエージェントとコマンドをコピーしたうえで実行する。

| コマンド | 説明 | 所要時間目安 |
|---------|------|-----------|
| `/modernize-assess` | クイック評価 — コード分析 → Core Domain Chart → MSS 戦略 | 短 |
| `/modernize-domain` | ドメイン境界分析 — ドメイン発見 → ISH 評価 → Context 設計 | 中 |
| `/modernize-migration` | 移行計画 — コード分析 → 25パターンから移行戦略選定 | 中 |
| `/modernize-full` | フル分析 — 10エージェント5フェーズ統合実行 | 長 |

## エージェント一覧

### 戦略・分析

| エージェント | 主要フレームワーク | 説明 |
|------------|-----------------|------|
| **modernization-strategist** | MSS（2軸×9戦略） | As-Is 分析、サブドメイン別戦略、段階的ロードマップ |
| **wardley-mapping-analyst** | 進化段階推定 | 依存関係からバリューチェーンを再構成、Build vs Buy 判断 |
| **technical-debt-assessor** | Core Domain Chart（8パターン）、7種の複雑さ | サブドメイン別投資判断、ポートフォリオ分析 |

### ドメイン発見・設計

| エージェント | 主要フレームワーク | 説明 |
|------------|-----------------|------|
| **domain-discovery-facilitator** | 6つの境界ヒューリスティック、ピボタルイベント | コードからドメインイベント抽出、サブドメイン境界提案 |
| **business-capability-mapper** | ISH（10問）、Product Taxonomy | 独立サービス候補の定量評価 |
| **bounded-context-designer** | BCC（11セクション）、Pain = S×V×D | Vlad Khononov の結合モデルで統合パターン設計 |
| **team-topologies-advisor** | Independent Value Stream（IVS）、Architecture Modernization Enabling Team（AMET、6目的）、Inverse Conway | git log からチーム構造推定、認知負荷評価 |

### 実行

| エージェント | 主要フレームワーク | 説明 |
|------------|-----------------|------|
| **platform-engineering-consultant** | 成熟度マトリックス、Thinnest Viable Platform（TVP） | インフラ自動スキャン、ゴールデンパス設計 |
| **strangler-fig-migration-planner** | 25パターン（11移行+6データ同期+5課題+3組織） | Feature Parity Trap 検出、移行順序最適化 |
| **legacy-code-analyzer** | ホットスポット × 複雑性マトリックス | 行動コード分析、God Module 検出、分離候補特定 |

### 統合

| エージェント | 説明 |
|------------|------|
| **modernization-orchestrator** | 3実行モード（フル/クイック/コード分析特化）、エージェント間の整合性検証 |

## インストール

### プロジェクトローカル

```bash
# 分析対象のプロジェクトにコピー
mkdir -p .claude/agents .claude/commands
cp agents/*.md .claude/agents/
cp .claude/commands/*.md .claude/commands/
```

### グローバル

```bash
cp agents/*.md ~/.claude/agents/
```

## 使用例

### コマンド経由（推奨）

```
# クイック評価
/modernize-assess

# ドメイン境界分析（特定ディレクトリにスコープ）
/modernize-domain src/

# フル分析
/modernize-full
```

### 単体エージェント

```
「modernization-strategist として、このシステムのモダナイゼーション戦略を策定してください」

「legacy-code-analyzer として、このリポジトリの複雑性を分析してください」
```

## エージェント間の連携

```
Phase 1: 戦略とコンテキスト（並行実行可能）
  ├── modernization-strategist → MSS
  ├── wardley-mapping-analyst → 進化段階
  └── technical-debt-assessor → Core Domain Chart

Phase 2: ドメイン発見
  ├── domain-discovery-facilitator → サブドメイン境界
  ├── business-capability-mapper → ISH 評価
  └── legacy-code-analyzer → ホットスポット

Phase 3: 設計
  ├── bounded-context-designer → BCC + Pain 分析
  └── team-topologies-advisor → IVS + AMET

Phase 4: 実行計画
  ├── platform-engineering-consultant → 成熟度 + TVP
  └── strangler-fig-migration-planner → 移行パターン

Phase 5: 統合
  └── modernization-orchestrator → 整合性検証 + 総合レポート
```

## フォーマット

各エージェントファイルは [awesome-claude-code-subagents](https://github.com/anthropics/awesome-claude-code-subagents) のフォーマットに準拠:

```markdown
---
name: agent-name
description: "エージェントの説明"
model: opus
tools:
  - Read
  - Write
  - ...
---

[System Prompt]
```

## License

Friend License (MIT-equivalent)
