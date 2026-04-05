# CLAUDE.md - Tools

## 概要

汎用ツール・評価プロンプト・検証用コード

## ディレクトリ構成

| ディレクトリ | 内容 |
|-------------|------|
| `blog_evaluation/` | ブログ記事評価プロンプト v2.2 |
| `memory_optimizer/` | CLAUDE.md 最適化ツール |
| `rust-sql-anti-pattern/` | SQLアンチパターン検証コード（Rust） |
| `vigil/` | セキュリティ監査ツールキット（Opus 4.6 セマンティック分析、OWASP Top 10、Web シェル検出） |
| `wardley-quest/` | 戦略シミュレーションRPG（Wardley Mapping × DDD × Team Topologies、アドオン含む） |
| `architecture-modernization-agents/` | アーキテクチャモダナイゼーション Claude Code サブエージェント群（11体、戦略〜移行まで） |
| `rustlean/` | MIRベース最適化支援ツール（Clone/Copy削減、アロケーション検出、構造体レイアウト分析） |

## コマンド

各ディレクトリで利用可能:

```
# blog_evaluation/
/evaluate [ファイルパス]

# memory_optimizer/
/optimize [ファイルパス]
```

## ライセンス

Friend License (MIT-equivalent)
