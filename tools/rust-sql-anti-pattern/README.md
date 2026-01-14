# Rust + PostgreSQL アンチパターン ブログシリーズ

SQLアンチパターンをRust + PostgreSQL（sqlx）で解説するブログシリーズの検証コード集です。

## プロジェクト構成

```
.
├── PLAN.md                          # ブログ構成計画
├── TODO.md                          # 続編テーマリスト
├── SPECIFICATION.md                 # 元のアンチパターン仕様書
├── INDEX.md                         # 目次
├── blog_01_db_design_pitfalls/      # DB設計の落とし穴
├── blog_02_performance_optimization/ # パフォーマンス最適化
├── blog_03_complex_data_structures/ # 複雑なデータ構造
├── blog_04_sqlx_safe_sql/           # sqlxで安全なSQL
└── blog_05_fulltext_search/         # PostgreSQL全文検索
```

## 環境準備

### PostgreSQL起動（Lima/Docker）

```bash
limactl start docker
limactl shell docker nerdctl run -d \
  --name postgres-antipattern \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=antipattern \
  -p 5432:5432 \
  postgres:16
```

### 接続確認

```bash
psql postgres://postgres:postgres@localhost:5432/antipattern
```

## 各ブログの実行

```bash
# Blog 1: DB設計の落とし穴
cd blog_01_db_design_pitfalls && cargo run

# Blog 2: パフォーマンス最適化
cd blog_02_performance_optimization && cargo run

# Blog 3: 複雑なデータ構造
cd blog_03_complex_data_structures && cargo run

# Blog 4: sqlxで安全なSQL
cd blog_04_sqlx_safe_sql && cargo run

# Blog 5: PostgreSQL全文検索
cd blog_05_fulltext_search && cargo run
```

## ブログ一覧

| # | タイトル | 対象者 | 主なトピック |
|---|---------|--------|-------------|
| 1 | DB設計の落とし穴 | 新規構築する開発者 | ジェイウォーク、IDリクワイアド、キーレスエントリ、ラウンディングエラー、31フレーバー |
| 2 | パフォーマンス最適化 | 「遅い」に悩む開発者 | N+1問題、インデックス設計、スパゲッティクエリ、GROUP BY、ランダム選択 |
| 3 | 複雑なデータ構造 | ドメインモデル設計者 | 階層構造、ポリモーフィック関連、EAV、マルチカラム属性 |
| 4 | sqlxで安全なSQL | sqlx初心者〜中級者 | NULL処理、SELECT *、SQLインジェクション、エラーハンドリング、型変換 |
| 5 | PostgreSQL全文検索 | 検索機能実装者 | LIKE限界、tsvector/tsquery、pg_trgm、日本語検索 |

## 使用ライブラリ

- `sqlx` - 型安全なSQLクライアント
- `tokio` - 非同期ランタイム
- `rust_decimal` - 正確な金額計算
- `chrono` - 日時処理
- `serde` / `serde_json` - JSONシリアライズ

## ライセンス

MIT
