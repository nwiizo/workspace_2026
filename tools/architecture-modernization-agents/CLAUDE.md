# CLAUDE.md - Architecture Modernization Agents

## 概要

Nick Tune 著「Architecture Modernization」の知識体系をベースとした Claude Code サブエージェント群。各フレームワーク・手法に特化した実務コンサルティング型エージェントを提供する。

## エージェント一覧

| エージェント | 対応章 | 用途 |
|------------|--------|------|
| `modernization-strategist` | Ch.1-3, 16 | 全体戦略策定、As-Is/To-Be 分析、ロードマップ |
| `wardley-mapping-analyst` | Ch.5 | バリューチェーン分析、進化段階評価、ASCII Map 生成 |
| `domain-discovery-facilitator` | Ch.4, 7, 9 | EventStorming、ドメインイベント抽出、サブドメイン分類 |
| `business-capability-mapper` | Ch.6 | ケイパビリティ階層化、Core/Supporting/Generic 分類 |
| `team-topologies-advisor` | Ch.11, 15 | チーム構造評価、認知負荷管理、AMET 設計 |
| `bounded-context-designer` | Ch.8, 9, 12 | Bounded Context 設計、コンテキストマップ、統合パターン |
| `platform-engineering-consultant` | Ch.13, 14 | IDP 設計、ゴールデンパス、Data Mesh |
| `technical-debt-assessor` | Ch.6, 10 | 技術的負債の定量評価、ポートフォリオ分析 |
| `strangler-fig-migration-planner` | Ch.10 | Strangler Fig 等の移行パターン設計 |
| `legacy-code-analyzer` | - | コードベースの複雑性・結合度分析 |
| `modernization-orchestrator` | 全体 | マルチエージェント統合、全体評価レポート |

## 使用方法

```bash
# プロジェクトローカルで利用
cp agents/*.md .claude/agents/

# グローバルで利用
cp agents/*.md ~/.claude/agents/
```

## コンテンツのみ

ビルド・テスト不要。マークダウンファイルのみで構成。
