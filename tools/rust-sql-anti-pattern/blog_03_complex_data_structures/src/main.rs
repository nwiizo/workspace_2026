//! 複雑なデータ構造 - アンチパターン検証コード（詳細版）
//!
//! このコードは以下のアンチパターンと解決策を実演します:
//! 1. ナイーブツリー（階層構造）
//!    → 隣接リスト / WITH RECURSIVE / 閉包テーブル / ltree拡張
//!    → パフォーマンス比較
//! 2. ポリモーフィック関連
//!    → 共通基底テーブル / Rust enumでの型安全な表現
//! 3. EAV（Entity-Attribute-Value）
//!    → JSONB解決策 / 高度なJSONB操作 / GINインデックス
//! 4. マルチカラムアトリビュート
//!    → 正規化 / PostgreSQL配列型

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Instant;

const DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/antipattern";

#[tokio::main]
async fn main() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await?;

    println!("=== 複雑なデータ構造 デモ ===\n");

    setup_tables(&pool).await?;
    insert_sample_data(&pool).await?;

    demo_naive_tree(&pool).await?;
    demo_tree_comparison(&pool).await?;
    demo_polymorphic(&pool).await?;
    demo_eav(&pool).await?;
    demo_jsonb_advanced(&pool).await?;
    demo_multicolumn(&pool).await?;

    cleanup_tables(&pool).await?;

    Ok(())
}

async fn setup_tables(pool: &PgPool) -> Result<()> {
    // ナイーブツリー: 隣接リスト（アンチパターン）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS comments_naive (
            id SERIAL PRIMARY KEY,
            parent_id INTEGER REFERENCES comments_naive(id),
            content TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ナイーブツリー: 閉包テーブル（解決策）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS comments_closure (
            id SERIAL PRIMARY KEY,
            content TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS comment_paths (
            ancestor_id INTEGER REFERENCES comments_closure(id) ON DELETE CASCADE,
            descendant_id INTEGER REFERENCES comments_closure(id) ON DELETE CASCADE,
            depth INTEGER NOT NULL,
            PRIMARY KEY (ancestor_id, descendant_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ポリモーフィック関連: アンチパターン
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS comments_polymorphic (
            id SERIAL PRIMARY KEY,
            commentable_type TEXT NOT NULL,  -- 'article' or 'video'
            commentable_id INTEGER NOT NULL,
            content TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ポリモーフィック関連: 解決策（共通基底テーブル）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS contents (
            id SERIAL PRIMARY KEY,
            content_type TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS articles_content (
            content_id INTEGER PRIMARY KEY REFERENCES contents(id),
            title TEXT NOT NULL,
            body TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS videos_content (
            content_id INTEGER PRIMARY KEY REFERENCES contents(id),
            url TEXT NOT NULL,
            duration INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS comments_content (
            id SERIAL PRIMARY KEY,
            content_id INTEGER REFERENCES contents(id) ON DELETE CASCADE,
            comment_text TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // EAV: アンチパターン
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS products_eav (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS product_attributes (
            id SERIAL PRIMARY KEY,
            product_id INTEGER REFERENCES products_eav(id),
            attribute_name TEXT NOT NULL,
            attribute_value TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // EAV: JSONB解決策
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS products_jsonb (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            attributes JSONB DEFAULT '{}'
        )
        "#,
    )
    .execute(pool)
    .await?;

    // マルチカラムアトリビュート: アンチパターン
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users_multicolumn (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            phone1 TEXT,
            phone2 TEXT,
            phone3 TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    // マルチカラムアトリビュート: 解決策
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users_normalized (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_phones (
            id SERIAL PRIMARY KEY,
            user_id INTEGER REFERENCES users_normalized(id) ON DELETE CASCADE,
            phone_type TEXT NOT NULL,  -- 'home', 'work', 'mobile'
            phone_number TEXT NOT NULL,
            is_primary BOOLEAN DEFAULT FALSE
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_sample_data(pool: &PgPool) -> Result<()> {
    // ナイーブツリーのサンプルデータ
    // コメント階層: 1 -> 2 -> 4, 1 -> 3
    sqlx::query("INSERT INTO comments_naive (id, parent_id, content) VALUES (1, NULL, 'ルートコメント')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO comments_naive (id, parent_id, content) VALUES (2, 1, '返信1')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO comments_naive (id, parent_id, content) VALUES (3, 1, '返信2')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO comments_naive (id, parent_id, content) VALUES (4, 2, '返信1への返信')")
        .execute(pool)
        .await?;

    // 閉包テーブルのサンプルデータ
    for i in 1..=4 {
        sqlx::query("INSERT INTO comments_closure (id, content) VALUES ($1, $2)")
            .bind(i)
            .bind(format!("コメント{}", i))
            .execute(pool)
            .await?;
    }

    // 閉包テーブルのパス情報
    // 自己参照
    for i in 1..=4 {
        sqlx::query("INSERT INTO comment_paths (ancestor_id, descendant_id, depth) VALUES ($1, $1, 0)")
            .bind(i)
            .execute(pool)
            .await?;
    }
    // 1 -> 2, 1 -> 3
    sqlx::query("INSERT INTO comment_paths (ancestor_id, descendant_id, depth) VALUES (1, 2, 1)")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO comment_paths (ancestor_id, descendant_id, depth) VALUES (1, 3, 1)")
        .execute(pool)
        .await?;
    // 1 -> 4 (through 2), 2 -> 4
    sqlx::query("INSERT INTO comment_paths (ancestor_id, descendant_id, depth) VALUES (1, 4, 2)")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO comment_paths (ancestor_id, descendant_id, depth) VALUES (2, 4, 1)")
        .execute(pool)
        .await?;

    // EAVのサンプルデータ
    sqlx::query("INSERT INTO products_eav (id, name) VALUES (1, 'ノートPC')")
        .execute(pool)
        .await?;
    for (attr, val) in &[("cpu", "Intel i7"), ("memory", "16GB"), ("storage", "512GB SSD")] {
        sqlx::query("INSERT INTO product_attributes (product_id, attribute_name, attribute_value) VALUES (1, $1, $2)")
            .bind(attr)
            .bind(val)
            .execute(pool)
            .await?;
    }

    // JSONB版
    sqlx::query(
        r#"INSERT INTO products_jsonb (id, name, attributes) VALUES (1, 'ノートPC', '{"cpu": "Intel i7", "memory": "16GB", "storage": "512GB SSD"}')"#,
    )
    .execute(pool)
    .await?;

    // マルチカラムのサンプルデータ
    sqlx::query("INSERT INTO users_multicolumn (id, name, phone1, phone2, phone3) VALUES (1, '田中太郎', '090-1234-5678', '03-1234-5678', NULL)")
        .execute(pool)
        .await?;

    // 正規化版
    sqlx::query("INSERT INTO users_normalized (id, name) VALUES (1, '田中太郎')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO user_phones (user_id, phone_type, phone_number, is_primary) VALUES (1, 'mobile', '090-1234-5678', TRUE)")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO user_phones (user_id, phone_type, phone_number, is_primary) VALUES (1, 'work', '03-1234-5678', FALSE)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn cleanup_tables(pool: &PgPool) -> Result<()> {
    let tables = [
        "user_phones",
        "users_normalized",
        "users_multicolumn",
        "products_jsonb",
        "product_attributes",
        "products_eav",
        "comments_content",
        "videos_content",
        "articles_content",
        "contents",
        "comments_polymorphic",
        "comment_paths",
        "comments_closure",
        "comments_naive",
    ];
    for table in tables {
        sqlx::query(&format!("DROP TABLE IF EXISTS {} CASCADE", table))
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// 1. ナイーブツリー: 階層構造を効率的に格納する
async fn demo_naive_tree(pool: &PgPool) -> Result<()> {
    println!("--- 1. ナイーブツリー ---");

    // アンチパターン: 隣接リストで子孫を取得するのは困難
    println!("  隣接リスト: 直接の子のみ簡単に取得可能");
    let children: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, content FROM comments_naive WHERE parent_id = 1")
            .fetch_all(pool)
            .await?;
    for (id, content) in &children {
        println!("    ID {}: {}", id, content);
    }

    // WITH RECURSIVEで全子孫を取得
    println!("\n  WITH RECURSIVEで全子孫を取得:");
    let descendants: Vec<(i32, String, i32)> = sqlx::query_as(
        r#"
        WITH RECURSIVE descendants AS (
            SELECT id, content, 0 as depth
            FROM comments_naive
            WHERE id = 1

            UNION ALL

            SELECT c.id, c.content, d.depth + 1
            FROM comments_naive c
            JOIN descendants d ON c.parent_id = d.id
        )
        SELECT id, content, depth FROM descendants
        "#,
    )
    .fetch_all(pool)
    .await?;

    for (id, content, depth) in &descendants {
        let indent = "  ".repeat(*depth as usize);
        println!("    {}ID {}: {}", indent, id, content);
    }

    // 解決策: 閉包テーブルで全子孫を取得
    println!("\n  閉包テーブルで子孫を取得:");
    let descendants: Vec<(i32, String, i32)> = sqlx::query_as(
        r#"
        SELECT c.id, c.content, p.depth
        FROM comments_closure c
        JOIN comment_paths p ON c.id = p.descendant_id
        WHERE p.ancestor_id = 1
        ORDER BY p.depth
        "#,
    )
    .fetch_all(pool)
    .await?;

    for (id, content, depth) in &descendants {
        let indent = "  ".repeat(*depth as usize);
        println!("    {}ID {}: {}", indent, id, content);
    }
    println!();

    Ok(())
}

/// 2. ポリモーフィック関連: 複数の親テーブルを参照する
async fn demo_polymorphic(pool: &PgPool) -> Result<()> {
    println!("--- 2. ポリモーフィック関連 ---");

    // アンチパターン: commentable_type + commentable_id
    sqlx::query(
        "INSERT INTO comments_polymorphic (commentable_type, commentable_id, content) VALUES ('article', 1, '記事へのコメント')",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO comments_polymorphic (commentable_type, commentable_id, content) VALUES ('video', 1, '動画へのコメント')",
    )
    .execute(pool)
    .await?;

    println!("  アンチパターン: 外部キー制約が使えない");
    let comments: Vec<(String, i32, String)> = sqlx::query_as(
        "SELECT commentable_type, commentable_id, content FROM comments_polymorphic",
    )
    .fetch_all(pool)
    .await?;
    for (ctype, cid, content) in &comments {
        println!("    {} #{}: {}", ctype, cid, content);
    }

    // 解決策: 共通基底テーブル
    println!("\n  解決策: 共通基底テーブル（contents）");

    // 記事を作成
    sqlx::query("INSERT INTO contents (id, content_type) VALUES (1, 'article')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO articles_content (content_id, title, body) VALUES (1, 'テスト記事', '本文...')")
        .execute(pool)
        .await?;

    // 動画を作成
    sqlx::query("INSERT INTO contents (id, content_type) VALUES (2, 'video')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO videos_content (content_id, url, duration) VALUES (2, 'https://example.com/video', 120)")
        .execute(pool)
        .await?;

    // コメント（外部キー制約あり）
    sqlx::query("INSERT INTO comments_content (content_id, comment_text) VALUES (1, '記事へのコメント')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO comments_content (content_id, comment_text) VALUES (2, '動画へのコメント')")
        .execute(pool)
        .await?;

    // 存在しないcontentへのコメントは失敗する
    let result = sqlx::query(
        "INSERT INTO comments_content (content_id, comment_text) VALUES (999, '存在しないコンテンツへのコメント')",
    )
    .execute(pool)
    .await;
    println!("    FK制約により不正なコメントは拒否: {:?}\n", result.is_err());

    Ok(())
}

/// 3. EAV（Entity-Attribute-Value）: 可変属性を扱う
async fn demo_eav(pool: &PgPool) -> Result<()> {
    println!("--- 3. EAV（Entity-Attribute-Value） ---");

    // アンチパターン: EAVテーブル
    println!("  EAVテーブルから属性を取得（複数行）:");
    let attrs: Vec<(String, String)> = sqlx::query_as(
        "SELECT attribute_name, attribute_value FROM product_attributes WHERE product_id = 1",
    )
    .fetch_all(pool)
    .await?;
    for (name, value) in &attrs {
        println!("    {}: {}", name, value);
    }

    // EAVの問題: 行から列への変換が必要
    println!("\n  EAV: 行→列変換（PIVOT）が必要:");
    let pivoted: (String, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        r#"
        SELECT
            p.name,
            MAX(CASE WHEN pa.attribute_name = 'cpu' THEN pa.attribute_value END) as cpu,
            MAX(CASE WHEN pa.attribute_name = 'memory' THEN pa.attribute_value END) as memory,
            MAX(CASE WHEN pa.attribute_name = 'storage' THEN pa.attribute_value END) as storage
        FROM products_eav p
        LEFT JOIN product_attributes pa ON p.id = pa.product_id
        WHERE p.id = 1
        GROUP BY p.name
        "#,
    )
    .fetch_one(pool)
    .await?;
    println!(
        "    {} - CPU: {:?}, Memory: {:?}, Storage: {:?}",
        pivoted.0, pivoted.1, pivoted.2, pivoted.3
    );

    // 解決策: JSONB
    println!("\n  JSONB: シンプルに取得:");

    #[derive(Debug, Serialize, Deserialize)]
    struct ProductAttributes {
        cpu: Option<String>,
        memory: Option<String>,
        storage: Option<String>,
    }

    let row: (String, serde_json::Value) =
        sqlx::query_as("SELECT name, attributes FROM products_jsonb WHERE id = 1")
            .fetch_one(pool)
            .await?;

    let attrs: ProductAttributes = serde_json::from_value(row.1)?;
    println!("    {} - {:?}", row.0, attrs);

    // JSONBでの検索
    println!("\n  JSONB: 属性値での検索:");
    let products: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM products_jsonb WHERE attributes->>'memory' = '16GB'",
    )
    .fetch_all(pool)
    .await?;
    for (name,) in &products {
        println!("    {}", name);
    }
    println!();

    Ok(())
}

/// 4. マルチカラムアトリビュート: 複数列で複数値を表現しない
async fn demo_multicolumn(pool: &PgPool) -> Result<()> {
    println!("--- 4. マルチカラムアトリビュート ---");

    // アンチパターン: phone1, phone2, phone3
    println!("  アンチパターン: 電話番号検索が複雑");
    let users: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM users_multicolumn WHERE phone1 = '090-1234-5678' OR phone2 = '090-1234-5678' OR phone3 = '090-1234-5678'",
    )
    .fetch_all(pool)
    .await?;
    println!("    検索結果: {:?}", users);

    // 解決策: 正規化
    println!("\n  正規化: シンプルな検索");
    let users: Vec<(String, String, String, bool)> = sqlx::query_as(
        r#"
        SELECT u.name, p.phone_type, p.phone_number, p.is_primary
        FROM users_normalized u
        JOIN user_phones p ON u.id = p.user_id
        WHERE p.phone_number = '090-1234-5678'
        "#,
    )
    .fetch_all(pool)
    .await?;
    for (name, ptype, number, primary) in &users {
        println!(
            "    {} - {} ({}) [primary: {}]",
            name, number, ptype, primary
        );
    }

    // 正規化: 全電話番号の取得
    println!("\n  正規化: ユーザーの全電話番号:");
    let phones: Vec<(String, String, bool)> = sqlx::query_as(
        "SELECT phone_type, phone_number, is_primary FROM user_phones WHERE user_id = 1 ORDER BY is_primary DESC",
    )
    .fetch_all(pool)
    .await?;
    for (ptype, number, primary) in &phones {
        let marker = if *primary { " (main)" } else { "" };
        println!("    {} ({}){}", number, ptype, marker);
    }

    // 新しい電話番号の追加（スキーマ変更不要）
    sqlx::query(
        "INSERT INTO user_phones (user_id, phone_type, phone_number) VALUES (1, 'home', '045-123-4567')",
    )
    .execute(pool)
    .await?;
    println!("\n  電話番号追加後:");
    let phones: Vec<(String, String)> =
        sqlx::query_as("SELECT phone_type, phone_number FROM user_phones WHERE user_id = 1")
            .fetch_all(pool)
            .await?;
    for (ptype, number) in &phones {
        println!("    {} ({})", number, ptype);
    }
    println!();

    Ok(())
}

/// 1b. 階層構造: 各アプローチの比較
async fn demo_tree_comparison(pool: &PgPool) -> Result<()> {
    println!("--- 1b. 階層構造アプローチ比較 ---");

    // 大量の階層データを生成（深さ5、各ノード3子）
    sqlx::query("DROP TABLE IF EXISTS tree_perf CASCADE").execute(pool).await?;
    sqlx::query(
        "CREATE TABLE tree_perf (
            id SERIAL PRIMARY KEY,
            parent_id INTEGER REFERENCES tree_perf(id),
            name TEXT NOT NULL,
            path TEXT  -- 経路列挙用
        )"
    )
    .execute(pool)
    .await?;

    // ルートノード
    sqlx::query("INSERT INTO tree_perf (id, parent_id, name, path) VALUES (1, NULL, 'Root', '1')")
        .execute(pool)
        .await?;

    // 子ノードを追加（簡略化）
    for i in 2..=10 {
        let parent = (i - 2) / 3 + 1;
        sqlx::query("INSERT INTO tree_perf (id, parent_id, name, path) VALUES ($1, $2, $3, $4)")
            .bind(i as i32)
            .bind(parent as i32)
            .bind(format!("Node {}", i))
            .bind(format!("1.{}", i))
            .execute(pool)
            .await?;
    }

    // WITH RECURSIVE のパフォーマンス
    let start = Instant::now();
    let _: Vec<(i32, String)> = sqlx::query_as(
        "WITH RECURSIVE tree AS (
            SELECT id, name, 0 as depth FROM tree_perf WHERE id = 1
            UNION ALL
            SELECT t.id, t.name, tree.depth + 1
            FROM tree_perf t
            JOIN tree ON t.parent_id = tree.id
        )
        SELECT id, name FROM tree"
    )
    .fetch_all(pool)
    .await?;
    println!("  WITH RECURSIVE: {:?}", start.elapsed());

    // 経路列挙の検索（LIKE使用）
    let start = Instant::now();
    let _: Vec<(i32, String)> = sqlx::query_as(
        "SELECT id, name FROM tree_perf WHERE path LIKE '1.%' OR id = 1"
    )
    .fetch_all(pool)
    .await?;
    println!("  経路列挙 (LIKE): {:?}", start.elapsed());

    // 閉包テーブルの検索
    let start = Instant::now();
    let _: Vec<(i32, String, i32)> = sqlx::query_as(
        "SELECT c.id, c.content, p.depth
         FROM comments_closure c
         JOIN comment_paths p ON c.id = p.descendant_id
         WHERE p.ancestor_id = 1"
    )
    .fetch_all(pool)
    .await?;
    println!("  閉包テーブル: {:?}", start.elapsed());

    // 各アプローチの比較表
    println!("\n  階層構造アプローチ比較:");
    println!("    ┌───────────────┬────────────┬────────────┬────────────┬──────────────┐");
    println!("    │ アプローチ    │ 読取速度   │ 書込速度   │ 参照整合性 │ メモリ使用量 │");
    println!("    ├───────────────┼────────────┼────────────┼────────────┼──────────────┤");
    println!("    │ 隣接リスト    │ × 再帰必要 │ ◎ 高速   │ ◎ FK可    │ ◎ 最小     │");
    println!("    │ 経路列挙      │ ○ LIKE    │ × 全更新  │ × なし    │ ○ 中程度   │");
    println!("    │ 閉包テーブル  │ ◎ 高速    │ △ O(n)挿入│ ◎ FK可    │ × O(n²)   │");
    println!("    │ WITH RECURSIVE│ ○ 柔軟    │ ◎ 高速   │ ◎ FK可    │ ◎ 最小     │");
    println!("    └───────────────┴────────────┴────────────┴────────────┴──────────────┘");

    // ltree拡張の紹介
    println!("\n  PostgreSQL ltree拡張（推奨）:");
    println!("    CREATE EXTENSION ltree;");
    println!("    path ltree NOT NULL  -- '1.2.3' のようなパス");
    println!("    path @> '1.2.*'      -- 祖先検索");
    println!("    path <@ '1'          -- 子孫検索");
    println!("    → GiSTインデックスで高速検索");
    println!();

    Ok(())
}

/// 3b. JSONB高度な操作
async fn demo_jsonb_advanced(pool: &PgPool) -> Result<()> {
    println!("--- 3b. JSONB高度な操作 ---");

    // GINインデックスの作成
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_products_jsonb_attrs ON products_jsonb USING GIN(attributes)")
        .execute(pool)
        .await?;

    // JSONB演算子
    println!("  JSONB演算子:");

    // -> : JSON値として取得
    let result: (serde_json::Value,) = sqlx::query_as(
        "SELECT attributes->'cpu' FROM products_jsonb WHERE id = 1"
    )
    .fetch_one(pool)
    .await?;
    println!("    attributes->'cpu' (JSON値): {}", result.0);

    // ->> : テキストとして取得
    let result: (Option<String>,) = sqlx::query_as(
        "SELECT attributes->>'memory' FROM products_jsonb WHERE id = 1"
    )
    .fetch_one(pool)
    .await?;
    println!("    attributes->>'memory' (テキスト): {:?}", result.0);

    // #> : ネストしたパスでJSON取得
    sqlx::query(
        r#"UPDATE products_jsonb SET attributes = attributes || '{"specs": {"cores": 8, "threads": 16}}' WHERE id = 1"#
    )
    .execute(pool)
    .await?;

    let result: (serde_json::Value,) = sqlx::query_as(
        "SELECT attributes#>'{specs,cores}' FROM products_jsonb WHERE id = 1"
    )
    .fetch_one(pool)
    .await?;
    println!("    attributes#>'{{specs,cores}}': {}", result.0);

    // @> : 含まれているか
    let result: (bool,) = sqlx::query_as(
        r#"SELECT attributes @> '{"cpu": "Intel i7"}' FROM products_jsonb WHERE id = 1"#
    )
    .fetch_one(pool)
    .await?;
    println!("    attributes @> '{{\"cpu\": \"Intel i7\"}}': {}", result.0);

    // ? : キーが存在するか
    let result: (bool,) = sqlx::query_as(
        "SELECT attributes ? 'gpu' FROM products_jsonb WHERE id = 1"
    )
    .fetch_one(pool)
    .await?;
    println!("    attributes ? 'gpu': {}", result.0);

    // JSONB更新操作
    println!("\n  JSONB更新操作:");

    // jsonb_set: 値の更新/追加
    sqlx::query(
        "UPDATE products_jsonb SET attributes = jsonb_set(attributes, '{gpu}', '\"RTX 4090\"') WHERE id = 1"
    )
    .execute(pool)
    .await?;
    println!("    jsonb_set で gpu を追加");

    // || : マージ
    sqlx::query(
        r#"UPDATE products_jsonb SET attributes = attributes || '{"warranty": "2 years"}' WHERE id = 1"#
    )
    .execute(pool)
    .await?;
    println!("    || で warranty をマージ");

    // - : キーの削除
    sqlx::query(
        "UPDATE products_jsonb SET attributes = attributes - 'warranty' WHERE id = 1"
    )
    .execute(pool)
    .await?;
    println!("    - で warranty を削除");

    // 最終状態確認
    let result: (serde_json::Value,) = sqlx::query_as(
        "SELECT attributes FROM products_jsonb WHERE id = 1"
    )
    .fetch_one(pool)
    .await?;
    println!("\n  最終状態: {}", serde_json::to_string_pretty(&result.0)?);

    // Rustでの型安全なJSONB操作
    println!("\n  Rustでの型安全なJSONB操作:");
    println!("    1. serde_json::Value で汎用的に扱う");
    println!("    2. 独自の struct を定義して from_value() でパース");
    println!("    3. sqlx::FromRow で直接マッピング（要カスタム実装）");

    #[derive(Debug, Serialize, Deserialize)]
    struct FullProductAttrs {
        cpu: Option<String>,
        memory: Option<String>,
        storage: Option<String>,
        gpu: Option<String>,
        specs: Option<Specs>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Specs {
        cores: Option<i32>,
        threads: Option<i32>,
    }

    let attrs: FullProductAttrs = serde_json::from_value(result.0)?;
    println!("    パース結果: {:?}", attrs);
    println!();

    Ok(())
}
