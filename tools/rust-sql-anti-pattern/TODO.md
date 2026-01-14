# 続編ブログテーマ（TODO）

本ドキュメントは、SPECIFICATION.md の内容のうち、5つのメインブログでカバーしきれなかったテーマをリストアップする。

---

## 優先度: 高 ✅ 完了

### 1. 大規模Webサービスのためのスケーラビリティ設計 ✅

- **ファイル**: `blog_06_scalability_design.md`
- **参照章**: 9章 メタデータトリブル
- **内容**:
  - テーブル分割の罠（年度別テーブル、ユーザー別テーブル）
  - PostgreSQLパーティショニング戦略（RANGE, LIST, HASH）
  - シャーディングの検討と実装パターン
  - sqlxでのパーティションテーブル操作

### 2. ファイルアップロード機能の正しい実装 ✅

- **ファイル**: `blog_07_file_upload.md`
- **参照章**: 12章 ファントムファイル
- **内容**:
  - ファイルシステム vs データベース格納の判断基準
  - BLOB/BYTEA の活用シーン
  - S3連携とトランザクション管理
  - 孤立ファイルの防止策
  - Rustでのマルチパートアップロード実装

---

## 優先度: 中 ✅ 完了

### 3. 認証・セキュリティのベストプラクティス ✅

- **ファイル**: `blog_08_security.md`
- **参照章**: 20章 リーダブルパスワード
- **内容**:
  - パスワードハッシュ化（argon2/bcrypt）の Rust実装
  - ソルトの生成と管理
  - セッション管理とトークン設計
  - OWASP Top 10 対策

### 4. マイグレーション戦略とスキーマ進化 ✅

- **ファイル**: `blog_09_migration.md`
- **参照章**: 24章 ディプロマティック・イミュニティ
- **内容**:
  - sqlxマイグレーションの活用
  - ゼロダウンタイムマイグレーション
  - 後方互換性を保つスキーマ変更
  - ロールバック戦略

---

## 優先度: 低 ✅ 完了

### 5. Dieselで実装するアンチパターン回避術 ✅

- **ファイル**: `blog_10_diesel_comparison.md`
- **内容**:
  - sqlx版との比較
  - Diesel特有のパターンと制約
  - 型安全なクエリビルダーの活用
  - マイグレーション管理の違い

### 6. PostgreSQL固有機能のRust活用ガイド ✅

- **ファイル**: `blog_11_postgresql_features.md`
- **内容**:
  - LISTEN/NOTIFY によるリアルタイム通知
  - Advisory Locks での分散ロック
  - Row Level Security（RLS）
  - JSONB の高度な操作
  - 拡張機能（PostGIS, TimescaleDB）との連携

---

## 完成したブログ一覧

### メインブログ（5本）
1. `blog_01_db_design_pitfalls.md` - DB設計の落とし穴
2. `blog_02_performance_optimization.md` - パフォーマンス最適化
3. `blog_03_complex_data_structures.md` - 複雑なデータ構造
4. `blog_04_sqlx_safe_sql.md` - sqlxで安全なSQL
5. `blog_05_fulltext_search.md` - PostgreSQL全文検索

### 続編ブログ（7本）
6. `blog_06_scalability_design.md` - スケーラビリティ設計
7. `blog_07_file_upload.md` - ファイルアップロード実装
8. `blog_08_security.md` - 認証・セキュリティ
9. `blog_09_migration.md` - マイグレーション戦略
10. `blog_10_diesel_comparison.md` - Diesel比較
11. `blog_11_postgresql_features.md` - PostgreSQL固有機能
12. `blog_12_soft_delete_patterns.md` - 論理削除を安全に実装する6つのパターン

---

## メモ

- 全12本のブログ記事が完成
- 各テーマは独立した記事として読める構成
- SPECIFICATION.md の全27章をカバー
- 論理削除の安全な実装パターン集を追加（Newtype、トレイト、ビュー、RLS、リポジトリ、マクロ）
