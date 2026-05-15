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
| `kuroko/` | 軽量 AWS サービスエミュレータ (Rust, axum 0.8, MIT)。port 4566・認証不要・単一バイナリ。AWS JSON 1.0/1.1, Query, REST, Smithy RPC v2 CBOR の 4 プロトコル dispatcher。フル実装18サービス (**S3 / SQS / DynamoDB / SNS (SNS→SQS fanout) / KMS / Secrets Manager / STS / CloudWatch Logs / SSM Parameter Store / EventBridge (EventBridge→SQS fanout) / Lambda (echo-Invoke) / Kinesis / Step Functions / IAM / ECR / ELBv2 / Route 53 / API Gateway v1**) は AWS SDK for Rust + 公式仕様準拠で 153 テスト疎通検証済み。残り 58 サービスは登録済みで 501 を返す stub。`/_kuroko/reset` / `/_kuroko/health` / `/_kuroko/services` / `/_kuroko/info` の introspection endpoint と KUROKO_DATA_DIR ベースの JSON snapshot 永続化対応 |

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
