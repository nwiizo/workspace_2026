# Architecture Modernization Agents

Nick Tune 著「[Architecture Modernization](https://www.manning.com/books/architecture-modernization)」の知識体系をベースとした Claude Code サブエージェント群。

レガシーシステムのモダナイゼーションにおける各フレームワーク・手法に特化した実務コンサルティング型エージェントを提供します。

## エージェント一覧

### 戦略・分析フェーズ

| エージェント | 説明 |
|------------|------|
| **modernization-strategist** | 全体戦略策定。ビジネス目標との整合性評価、As-Is/To-Be 分析、段階的ロードマップ作成 |
| **wardley-mapping-analyst** | Wardley Map による戦略分析。バリューチェーンの可視化、進化段階評価、ASCII Map 生成 |
| **business-capability-mapper** | ビジネスケイパビリティの階層構造化。Core/Supporting/Generic 分類、投資判断支援 |
| **technical-debt-assessor** | 技術的負債の定量評価。4象限分類、ポートフォリオ分析、優先順位付け |

### ドメイン発見・設計フェーズ

| エージェント | 説明 |
|------------|------|
| **domain-discovery-facilitator** | EventStorming ファシリテーション。ドメインイベント抽出、サブドメイン分類、ユビキタス言語策定 |
| **bounded-context-designer** | Bounded Context 設計。コンテキスト境界定義、コンテキストマップ作成、統合パターン選定 |
| **team-topologies-advisor** | Team Topologies ベースのチーム設計。4チームタイプ × 3インタラクションモード、AMET 設計 |

### 実行フェーズ

| エージェント | 説明 |
|------------|------|
| **platform-engineering-consultant** | Internal Developer Platform 設計。ゴールデンパス、セルフサービス化、Data Mesh 戦略 |
| **strangler-fig-migration-planner** | 段階的移行パターン設計。Strangler Fig、Branch by Abstraction、Parallel Run |
| **legacy-code-analyzer** | レガシーコードの定量分析。複雑性ホットスポット、結合度、依存関係グラフ、分離候補特定 |

### 統合

| エージェント | 説明 |
|------------|------|
| **modernization-orchestrator** | マルチエージェント統合。全エージェントの呼び出し順序制御と結果統合 |

## インストール

### プロジェクトローカル

```bash
# プロジェクトの .claude/agents/ にコピー
mkdir -p .claude/agents
cp agents/*.md .claude/agents/
```

### グローバル

```bash
# ~/.claude/agents/ にコピー（全プロジェクトで利用可能）
cp agents/*.md ~/.claude/agents/
```

## 使用例

### 単体エージェントの利用

Claude Code で特定のエージェントを呼び出して、専門的な分析を依頼します。

```
# 全体戦略の策定
「modernization-strategist として、このシステムのモダナイゼーション戦略を策定してください」

# コードベースの分析
「legacy-code-analyzer として、このリポジトリの複雑性を分析してください」

# Wardley Map の作成
「wardley-mapping-analyst として、ECサイトのバリューチェーンを Wardley Map で可視化してください」
```

### オーケストレーターによる統合分析

```
「modernization-orchestrator として、このシステムの包括的なモダナイゼーション評価を行ってください」
```

オーケストレーターは以下のフェーズで各エージェントを順次呼び出します:

1. **Phase 1**: 戦略評価（strategist + wardley + debt assessor）
2. **Phase 2**: ドメイン発見（domain discovery + capability mapper + code analyzer）
3. **Phase 3**: 設計（bounded context + team topologies）
4. **Phase 4**: 実行計画（platform + migration planner）

## エージェント間の連携

```
modernization-strategist ──→ wardley-mapping-analyst
         │                          │
         ▼                          ▼
technical-debt-assessor    business-capability-mapper
         │                          │
         ▼                          ▼
legacy-code-analyzer ──→ domain-discovery-facilitator
                                    │
                                    ▼
                          bounded-context-designer
                           │                │
                           ▼                ▼
               team-topologies-advisor    strangler-fig-migration-planner
                           │                │
                           ▼                ▼
              platform-engineering-consultant
                           │
                           ▼
               modernization-orchestrator（統合）
```

## 対応する書籍の章

| 章 | 内容 | 対応エージェント |
|----|------|----------------|
| Ch.1-3 | モダナイゼーション概要、準備、ビジネス目標 | modernization-strategist |
| Ch.4 | リスニングツアー | domain-discovery-facilitator |
| Ch.5 | Wardley Mapping | wardley-mapping-analyst |
| Ch.6 | プロダクト分類・ケイパビリティ | technical-debt-assessor, business-capability-mapper |
| Ch.7 | EventStorming | domain-discovery-facilitator |
| Ch.8 | ドメインモダナイゼーション | bounded-context-designer |
| Ch.9 | サブドメイン・Bounded Context | bounded-context-designer, domain-discovery-facilitator |
| Ch.10 | 戦略的 IT ポートフォリオ・移行パターン | technical-debt-assessor, strangler-fig-migration-planner |
| Ch.11 | Team Topologies | team-topologies-advisor |
| Ch.12 | 疎結合アーキテクチャ | bounded-context-designer |
| Ch.13 | Internal Developer Platform | platform-engineering-consultant |
| Ch.14 | Data Mesh | platform-engineering-consultant |
| Ch.15 | AMET | team-topologies-advisor |
| Ch.16 | 戦略とロードマップ | modernization-strategist |

## フォーマット

各エージェントファイルは [awesome-claude-code-subagents](https://github.com/anthropics/awesome-claude-code-subagents) のフォーマットに準拠しています:

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

## 参考文献

- Nick Tune, Jean-Georges Perrin. "Architecture Modernization". Manning Publications, 2024.
- Matthew Skelton, Manuel Pais. "Team Topologies". IT Revolution Press, 2019.
- Simon Wardley. "Wardley Maps". 2018.
- Eric Evans. "Domain-Driven Design". Addison-Wesley, 2003.
- Alberto Brandolini. "Introducing EventStorming". Leanpub, 2021.
- Zhamak Dehghani. "Data Mesh". O'Reilly Media, 2022.
- Sam Newman. "Monolith to Microservices". O'Reilly Media, 2019.

## License

Friend License (MIT-equivalent)
