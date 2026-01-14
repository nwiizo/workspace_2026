//! パフォーマンス最適化 - アンチパターン検証コード（詳細版）
//!
//! このコードは以下のアンチパターンと解決策を実演します:
//! 1. N+1クエリ問題
//!    → JOIN / IN句+HashMap / array_agg / json_agg / DataLoader
//! 2. インデックスショットガン
//!    → EXPLAIN ANALYZE / 複合インデックス / 部分インデックス / カバリングインデックス
//! 3. スパゲッティクエリ
//!    → tokio::try_join! / CTE(WITH句) / マテリアライズドビュー
//! 4. アンビギュアスグループ
//!    → ROW_NUMBER() / DISTINCT ON / LATERAL JOIN / FILTER句
//! 5. ランダムセレクション
//!    → TABLESAMPLE BERNOULLI vs SYSTEM / オフセット方式 / キャッシュ方式
//! 6. 接続プール設計
//!    → PgPoolOptions / モニタリング

use anyhow::Result;
use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

const DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/antipattern";

#[tokio::main]
async fn main() -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(2)
        .connect(DATABASE_URL)
        .await?;

    println!("=== パフォーマンス最適化 デモ ===\n");

    setup_tables(&pool).await?;
    insert_sample_data(&pool).await?;

    demo_n_plus_1(&pool).await?;
    demo_json_agg(&pool).await?;
    demo_dataloader(&pool).await?;
    demo_index_shotgun(&pool).await?;
    demo_index_advanced(&pool).await?;
    demo_spaghetti_query(&pool).await?;
    demo_cte_examples(&pool).await?;
    demo_ambiguous_group(&pool).await?;
    demo_lateral_join(&pool).await?;
    demo_random_selection(&pool).await?;
    demo_random_cache(&pool).await?;
    demo_connection_pool(&pool).await?;
    demo_n_plus_one_detection(&pool).await?;
    demo_rust_detection_methods(&pool).await?;

    cleanup_tables(&pool).await?;

    Ok(())
}

async fn setup_tables(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS authors (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS articles (
            id SERIAL PRIMARY KEY,
            author_id INTEGER REFERENCES authors(id),
            title TEXT NOT NULL,
            view_count INTEGER DEFAULT 0,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // インデックスなし版
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS articles_no_index (
            id SERIAL PRIMARY KEY,
            author_id INTEGER,
            title TEXT NOT NULL,
            status TEXT DEFAULT 'draft',
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // コメントテーブル（json_aggデモ用）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS comments (
            id SERIAL PRIMARY KEY,
            article_id INTEGER REFERENCES articles(id),
            body TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ランダム選択用のテーブル
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS large_table (
            id SERIAL PRIMARY KEY,
            data TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_sample_data(pool: &PgPool) -> Result<()> {
    // 著者データ
    for i in 1..=10 {
        sqlx::query("INSERT INTO authors (name) VALUES ($1)")
            .bind(format!("Author {}", i))
            .execute(pool)
            .await?;
    }

    // 記事データ
    for author_id in 1..=10 {
        for j in 1..=5 {
            sqlx::query("INSERT INTO articles (author_id, title, view_count) VALUES ($1, $2, $3)")
                .bind(author_id)
                .bind(format!("Article {} by Author {}", j, author_id))
                .bind(j * 100)
                .execute(pool)
                .await?;
        }
    }

    // インデックスなしテーブルにもデータ
    for i in 1..=1000 {
        sqlx::query("INSERT INTO articles_no_index (author_id, title, status) VALUES ($1, $2, $3)")
            .bind(i % 10 + 1)
            .bind(format!("Article {}", i))
            .bind(if i % 3 == 0 { "published" } else { "draft" })
            .execute(pool)
            .await?;
    }

    // コメントデータ
    for article_id in 1..=50 {
        for j in 1..=3 {
            sqlx::query("INSERT INTO comments (article_id, body) VALUES ($1, $2)")
                .bind(article_id)
                .bind(format!("Comment {} on article {}", j, article_id))
                .execute(pool)
                .await?;
        }
    }

    // ランダム選択用の大量データ
    sqlx::query(
        r#"
        INSERT INTO large_table (data)
        SELECT 'data_' || generate_series
        FROM generate_series(1, 10000)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn cleanup_tables(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS comments CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS large_table CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS articles_no_index CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS articles CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS authors CASCADE")
        .execute(pool)
        .await?;
    Ok(())
}

/// 1. N+1クエリ問題
async fn demo_n_plus_1(pool: &PgPool) -> Result<()> {
    println!("--- 1. N+1クエリ問題 ---");

    // アンチパターン: 記事ごとに著者を取得
    let start = Instant::now();
    let articles: Vec<(i32, String, i32)> =
        sqlx::query_as("SELECT id, title, author_id FROM articles")
            .fetch_all(pool)
            .await?;

    let mut query_count = 1;
    for (_, _, author_id) in &articles {
        let _author: (String,) = sqlx::query_as("SELECT name FROM authors WHERE id = $1")
            .bind(author_id)
            .fetch_one(pool)
            .await?;
        query_count += 1;
    }
    println!(
        "  N+1パターン: {} クエリ, {:?}",
        query_count,
        start.elapsed()
    );

    // 解決策1: JOINで一括取得
    let start = Instant::now();
    let _results: Vec<(i32, String, String)> = sqlx::query_as(
        r#"
        SELECT a.id, a.title, au.name
        FROM articles a
        JOIN authors au ON a.author_id = au.id
        "#,
    )
    .fetch_all(pool)
    .await?;
    println!("  JOIN: 1 クエリ, {:?}", start.elapsed());

    // 解決策2: IN句 + HashMap
    let start = Instant::now();
    let articles: Vec<(i32, String, i32)> =
        sqlx::query_as("SELECT id, title, author_id FROM articles")
            .fetch_all(pool)
            .await?;

    let author_ids: Vec<i32> = articles.iter().map(|(_, _, id)| *id).collect();
    let authors: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM authors WHERE id = ANY($1)")
            .bind(&author_ids)
            .fetch_all(pool)
            .await?;

    let author_map: HashMap<i32, String> = authors.into_iter().collect();
    for (_, title, author_id) in &articles {
        let _author_name = author_map.get(author_id);
        let _ = (title, _author_name); // 使用
    }
    println!("  IN句+HashMap: 2 クエリ, {:?}\n", start.elapsed());

    Ok(())
}

/// 2. インデックスショットガン
async fn demo_index_shotgun(pool: &PgPool) -> Result<()> {
    println!("--- 2. インデックスショットガン ---");

    // インデックスなしでの検索
    let start = Instant::now();
    let _: Vec<(i32,)> = sqlx::query_as(
        "SELECT id FROM articles_no_index WHERE author_id = 5 AND status = 'published'",
    )
    .fetch_all(pool)
    .await?;
    println!("  インデックスなし: {:?}", start.elapsed());

    // EXPLAIN結果の確認
    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT id FROM articles_no_index WHERE author_id = 5 AND status = 'published'",
    )
    .fetch_all(pool)
    .await?;
    println!("  EXPLAIN:");
    for (line,) in &explain {
        println!("    {}", line);
    }

    // 複合インデックスを追加
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_author_status ON articles_no_index (author_id, status)")
        .execute(pool)
        .await?;

    let start = Instant::now();
    let _: Vec<(i32,)> = sqlx::query_as(
        "SELECT id FROM articles_no_index WHERE author_id = 5 AND status = 'published'",
    )
    .fetch_all(pool)
    .await?;
    println!("  複合インデックスあり: {:?}", start.elapsed());

    // インデックス後のEXPLAIN
    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT id FROM articles_no_index WHERE author_id = 5 AND status = 'published'",
    )
    .fetch_all(pool)
    .await?;
    println!("  EXPLAIN (インデックス後):");
    for (line,) in &explain {
        println!("    {}", line);
    }
    println!();

    Ok(())
}

/// 3. スパゲッティクエリ
async fn demo_spaghetti_query(pool: &PgPool) -> Result<()> {
    println!("--- 3. スパゲッティクエリ ---");

    // 複数の統計を並列で取得
    let start = Instant::now();
    let (total, avg_views, top_author) = tokio::try_join!(
        async {
            let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM articles")
                .fetch_one(pool)
                .await?;
            Ok::<_, sqlx::Error>(result.0)
        },
        async {
            let result: (Option<f64>,) =
                sqlx::query_as("SELECT AVG(view_count)::float8 FROM articles")
                    .fetch_one(pool)
                    .await?;
            Ok::<_, sqlx::Error>(result.0.unwrap_or(0.0))
        },
        async {
            let result: (String, i64) = sqlx::query_as(
                r#"
                SELECT au.name, COUNT(*) as cnt
                FROM articles a
                JOIN authors au ON a.author_id = au.id
                GROUP BY au.name
                ORDER BY cnt DESC
                LIMIT 1
                "#,
            )
            .fetch_one(pool)
            .await?;
            Ok::<_, sqlx::Error>(result)
        }
    )?;

    println!("  並列クエリ結果:");
    println!("    総記事数: {}", total);
    println!("    平均閲覧数: {:.2}", avg_views);
    println!("    最多投稿著者: {} ({} 記事)", top_author.0, top_author.1);
    println!("  実行時間: {:?}\n", start.elapsed());

    Ok(())
}

/// 4. アンビギュアスグループ
async fn demo_ambiguous_group(pool: &PgPool) -> Result<()> {
    println!("--- 4. アンビギュアスグループ ---");

    // 各著者の最新記事を取得

    // 解決策1: ウィンドウ関数 ROW_NUMBER()
    let results: Vec<(String, String, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT name, title, created_at
        FROM (
            SELECT au.name, a.title, a.created_at,
                   ROW_NUMBER() OVER (PARTITION BY a.author_id ORDER BY a.created_at DESC) as rn
            FROM articles a
            JOIN authors au ON a.author_id = au.id
        ) sub
        WHERE rn = 1
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("  ROW_NUMBER()による各著者の最新記事:");
    for (name, title, created_at) in results.iter().take(3) {
        println!("    {} - {} ({})", name, title, created_at);
    }

    // 解決策2: DISTINCT ON（PostgreSQL固有）
    let results: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT DISTINCT ON (a.author_id) au.name, a.title
        FROM articles a
        JOIN authors au ON a.author_id = au.id
        ORDER BY a.author_id, a.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("  DISTINCT ONによる各著者の最新記事:");
    for (name, title) in results.iter().take(3) {
        println!("    {} - {}", name, title);
    }
    println!();

    Ok(())
}

/// 5. ランダムセレクション
async fn demo_random_selection(pool: &PgPool) -> Result<()> {
    println!("--- 5. ランダムセレクション ---");

    // アンチパターン: ORDER BY RANDOM()
    let start = Instant::now();
    let _: Vec<(i32,)> = sqlx::query_as("SELECT id FROM large_table ORDER BY RANDOM() LIMIT 5")
        .fetch_all(pool)
        .await?;
    println!("  ORDER BY RANDOM(): {:?}", start.elapsed());

    // 解決策1: オフセット方式
    let start = Instant::now();
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM large_table")
        .fetch_one(pool)
        .await?;
    let offset = rand::random::<u64>() % (count.0 as u64);
    let _: Vec<(i32,)> = sqlx::query_as("SELECT id FROM large_table LIMIT 5 OFFSET $1")
        .bind(offset as i64)
        .fetch_all(pool)
        .await?;
    println!("  オフセット方式: {:?}", start.elapsed());

    // 解決策2: ID範囲からランダム選択
    let start = Instant::now();
    let (min_id, max_id): (i32, i32) = sqlx::query_as("SELECT MIN(id), MAX(id) FROM large_table")
        .fetch_one(pool)
        .await?;

    let mut random_ids: Vec<i32> = Vec::new();
    for _ in 0..5 {
        let random_id = min_id + (rand::random::<u32>() % ((max_id - min_id + 1) as u32)) as i32;
        random_ids.push(random_id);
    }

    let _: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, data FROM large_table WHERE id = ANY($1)")
            .bind(&random_ids)
            .fetch_all(pool)
            .await?;
    println!("  ID範囲方式: {:?}", start.elapsed());

    // 解決策3: TABLESAMPLE SYSTEM（高速だが偏りあり）
    let start = Instant::now();
    let results: Vec<(i32,)> =
        sqlx::query_as("SELECT id FROM large_table TABLESAMPLE SYSTEM(1) LIMIT 5")
            .fetch_all(pool)
            .await?;
    println!(
        "  TABLESAMPLE SYSTEM: {:?} ({} 件)",
        start.elapsed(),
        results.len()
    );

    // 解決策4: TABLESAMPLE BERNOULLI（均一だが遅い）
    let start = Instant::now();
    let results: Vec<(i32,)> =
        sqlx::query_as("SELECT id FROM large_table TABLESAMPLE BERNOULLI(0.1) LIMIT 5")
            .fetch_all(pool)
            .await?;
    println!(
        "  TABLESAMPLE BERNOULLI: {:?} ({} 件)",
        start.elapsed(),
        results.len()
    );

    // SYSTEM vs BERNOULLI の違い
    println!("\n  TABLESAMPLE の違い:");
    println!("    SYSTEM: ページ単位でサンプリング → 高速だが偏りあり");
    println!("    BERNOULLI: 行単位でサンプリング → 均一だが遅い");
    println!("    注意: どちらも行数が保証されない（LIMITと併用推奨）");
    println!();

    Ok(())
}

/// 2b. 高度なインデックス: 部分インデックス / カバリングインデックス / EXPLAIN ANALYZE
async fn demo_index_advanced(pool: &PgPool) -> Result<()> {
    println!("--- 2b. 高度なインデックス設計 ---");

    // 部分インデックス（Partial Index）
    println!("  部分インデックス（条件付きインデックス）:");
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_articles_published
         ON articles_no_index (author_id)
         WHERE status = 'published'",
    )
    .execute(pool)
    .await?;

    // 部分インデックスの効果確認
    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN ANALYZE SELECT id FROM articles_no_index
         WHERE author_id = 5 AND status = 'published'",
    )
    .fetch_all(pool)
    .await?;
    println!("    EXPLAIN ANALYZE (部分インデックス使用):");
    for (line,) in explain.iter().take(3) {
        println!("      {}", line);
    }

    // 式インデックス（Expression Index）
    println!("\n  式インデックス:");
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_articles_title_lower
         ON articles_no_index (lower(title))",
    )
    .execute(pool)
    .await?;

    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT id FROM articles_no_index WHERE lower(title) = 'article 50'",
    )
    .fetch_all(pool)
    .await?;
    println!("    EXPLAIN (式インデックス):");
    for (line,) in &explain {
        println!("      {}", line);
    }

    // カバリングインデックス（INCLUDE）
    println!("\n  カバリングインデックス（PostgreSQL 11+）:");
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_articles_covering
         ON articles_no_index (author_id)
         INCLUDE (title, status)",
    )
    .execute(pool)
    .await?;

    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT author_id, title, status FROM articles_no_index WHERE author_id = 5",
    )
    .fetch_all(pool)
    .await?;
    println!("    EXPLAIN (Index Only Scanが可能に):");
    for (line,) in &explain {
        println!("      {}", line);
    }

    // インデックス設計指針
    println!("\n  インデックス設計指針:");
    println!("    1. WHERE句で頻繁に使われる列 → B-treeインデックス");
    println!("    2. 範囲検索が多い列 → 複合インデックスの先頭に");
    println!("    3. 一部のデータのみ検索 → 部分インデックス");
    println!("    4. 関数適用 → 式インデックス");
    println!("    5. SELECT列もカバー → INCLUDEでIndex Only Scan");
    println!("    6. 配列/JSONB → GINインデックス");
    println!();

    Ok(())
}

/// 3b. CTE（WITH句）による段階的クエリ構築
async fn demo_cte_examples(pool: &PgPool) -> Result<()> {
    println!("--- 3b. CTE（WITH句）による可読性向上 ---");

    // シンプルなCTE
    println!("  CTE基本形:");
    let result: Vec<(String, i64, i64)> = sqlx::query_as(
        r#"
        WITH author_stats AS (
            SELECT author_id, COUNT(*) as article_count, SUM(view_count) as total_views
            FROM articles
            GROUP BY author_id
        )
        SELECT au.name, s.article_count, s.total_views
        FROM author_stats s
        JOIN authors au ON au.id = s.author_id
        ORDER BY s.total_views DESC
        LIMIT 3
        "#,
    )
    .fetch_all(pool)
    .await?;

    for (name, count, views) in &result {
        println!("    {} - {} 記事, {} 閲覧", name, count, views);
    }

    // 複数CTEの連結
    println!("\n  複数CTEの連結:");
    let result: Vec<(String, f64, f64)> = sqlx::query_as(
        r#"
        WITH
        article_stats AS (
            SELECT author_id, AVG(view_count)::float8 as avg_views
            FROM articles
            GROUP BY author_id
        ),
        global_avg AS (
            SELECT AVG(view_count)::float8 as avg FROM articles
        )
        SELECT au.name, s.avg_views, g.avg
        FROM article_stats s
        JOIN authors au ON au.id = s.author_id
        CROSS JOIN global_avg g
        WHERE s.avg_views > g.avg
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("    平均以上の閲覧数を持つ著者:");
    for (name, author_avg, global_avg) in &result {
        println!(
            "      {} - 平均 {:.1} (全体平均: {:.1})",
            name, author_avg, global_avg
        );
    }

    // マテリアライズドビューの概念
    println!("\n  マテリアライズドビュー（事前計算結果の保存）:");
    println!("    CREATE MATERIALIZED VIEW author_summary AS ...");
    println!("    REFRESH MATERIALIZED VIEW author_summary;");
    println!("    → 集計が重い場合は定期的にREFRESHして高速化");
    println!();

    Ok(())
}

/// 4b. LATERAL JOINとFILTER句
async fn demo_lateral_join(pool: &PgPool) -> Result<()> {
    println!("--- 4b. LATERAL JOINとFILTER句 ---");

    // LATERAL JOIN: 各著者の最新3記事を取得
    println!("  LATERAL JOIN（各著者の最新3記事）:");
    let results: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT au.name, a.title
        FROM authors au
        CROSS JOIN LATERAL (
            SELECT title
            FROM articles
            WHERE author_id = au.id
            ORDER BY created_at DESC
            LIMIT 3
        ) a
        ORDER BY au.name
        LIMIT 9
        "#,
    )
    .fetch_all(pool)
    .await?;

    for (name, title) in &results {
        println!("    {} - {}", name, title);
    }

    // FILTER句による条件付き集約
    println!("\n  FILTER句（条件付き集約）:");
    let result: Vec<(i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE status = 'published') as published,
            COUNT(*) FILTER (WHERE status = 'draft') as draft
        FROM articles_no_index
        "#,
    )
    .fetch_one(pool)
    .await
    .map(|r| vec![r])?;

    for (total, published, draft) in &result {
        println!(
            "    全記事: {}, 公開: {}, 下書き: {}",
            total, published, draft
        );
    }

    // CASE式との比較
    println!("\n  FILTER vs CASE:");
    println!("    FILTER: COUNT(*) FILTER (WHERE condition)");
    println!("    CASE:   SUM(CASE WHEN condition THEN 1 ELSE 0 END)");
    println!("    → FILTERの方が意図が明確で読みやすい（PostgreSQL 9.4+）");
    println!();

    Ok(())
}

/// 1b. json_aggによる複雑なネスト構造の取得
async fn demo_json_agg(pool: &PgPool) -> Result<()> {
    println!("--- 1b. json_aggによるネスト構造の取得 ---");

    // コメントをJSON配列として取得
    #[derive(Debug, Serialize, Deserialize)]
    struct CommentJson {
        id: i32,
        body: String,
    }

    let results: Vec<(i32, String, serde_json::Value)> = sqlx::query_as(
        r#"
        SELECT
            a.id,
            a.title,
            COALESCE(
                json_agg(
                    json_build_object(
                        'id', c.id,
                        'body', c.body
                    )
                    ORDER BY c.created_at DESC
                ) FILTER (WHERE c.id IS NOT NULL),
                '[]'
            ) as comments
        FROM articles a
        LEFT JOIN comments c ON a.id = c.article_id
        GROUP BY a.id, a.title
        ORDER BY a.id
        LIMIT 5
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("  記事とコメント（json_agg使用）:");
    for (id, title, comments) in &results {
        let comments_parsed: Vec<CommentJson> =
            serde_json::from_value(comments.clone()).unwrap_or_default();
        println!(
            "    記事 {} ({}): {} コメント",
            id,
            title,
            comments_parsed.len()
        );
        for comment in comments_parsed.iter().take(2) {
            println!("      - {}", comment.body);
        }
    }

    // jsonb_agg vs json_agg の違い
    println!("\n  json_agg vs jsonb_agg:");
    println!("    json_agg:  テキスト格納、順序保持");
    println!("    jsonb_agg: バイナリ格納、重複キー削除、インデックス可能");
    println!("    → 単純な集約にはjson_agg、検索が必要ならjsonb_agg");
    println!();

    Ok(())
}

/// 1c. DataLoaderパターン
async fn demo_dataloader(pool: &PgPool) -> Result<()> {
    println!("--- 1c. DataLoaderパターン ---");

    // シンプルなDataLoader実装
    struct AuthorLoader {
        pool: PgPool,
        cache: RwLock<HashMap<i32, String>>,
    }

    impl AuthorLoader {
        fn new(pool: PgPool) -> Self {
            Self {
                pool,
                cache: RwLock::new(HashMap::new()),
            }
        }

        async fn load_many(&self, ids: &[i32]) -> Result<HashMap<i32, String>, sqlx::Error> {
            let cache = self.cache.read().await;

            // キャッシュにないIDを特定
            let missing: Vec<i32> = ids
                .iter()
                .filter(|id| !cache.contains_key(id))
                .copied()
                .collect();

            drop(cache); // ロックを解放

            if !missing.is_empty() {
                // 一括でDBから取得
                let authors: Vec<(i32, String)> =
                    sqlx::query_as("SELECT id, name FROM authors WHERE id = ANY($1)")
                        .bind(&missing)
                        .fetch_all(&self.pool)
                        .await?;

                // キャッシュに追加
                let mut cache = self.cache.write().await;
                for (id, name) in authors {
                    cache.insert(id, name);
                }
            }

            // 結果を構築
            let cache = self.cache.read().await;
            let result = ids
                .iter()
                .filter_map(|id| cache.get(id).map(|name| (*id, name.clone())))
                .collect();

            Ok(result)
        }
    }

    let loader = AuthorLoader::new(pool.clone());

    // 使用例: 記事を取得して著者をDataLoaderで解決
    let start = Instant::now();
    let articles: Vec<(i32, String, i32)> =
        sqlx::query_as("SELECT id, title, author_id FROM articles LIMIT 20")
            .fetch_all(pool)
            .await?;

    let author_ids: Vec<i32> = articles.iter().map(|(_, _, id)| *id).collect();
    let authors = loader.load_many(&author_ids).await?;

    println!("  DataLoader結果（2クエリで完了）:");
    for (id, title, author_id) in articles.iter().take(5) {
        let author_name = authors
            .get(author_id)
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        println!("    記事 {}: {} by {}", id, title, author_name);
    }
    println!("  実行時間: {:?}", start.elapsed());

    // 2回目のロード（キャッシュヒット）
    let start = Instant::now();
    let _ = loader.load_many(&author_ids).await?;
    println!("  キャッシュヒット時: {:?}\n", start.elapsed());

    Ok(())
}

/// 5b. キャッシュ方式のランダム選択
async fn demo_random_cache(pool: &PgPool) -> Result<()> {
    println!("--- 5b. キャッシュ方式のランダム選択 ---");

    // ランダム選択用キャッシュ
    struct RandomCache {
        ids: Arc<RwLock<Vec<i32>>>,
    }

    impl RandomCache {
        fn new() -> Self {
            Self {
                ids: Arc::new(RwLock::new(Vec::new())),
            }
        }

        async fn refresh(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
            let ids: Vec<(i32,)> = sqlx::query_as("SELECT id FROM large_table")
                .fetch_all(pool)
                .await?;

            let mut cache = self.ids.write().await;
            *cache = ids.into_iter().map(|(id,)| id).collect();
            Ok(())
        }

        async fn get_random(&self, count: usize) -> Vec<i32> {
            let ids = self.ids.read().await;
            let mut rng = rand::thread_rng();
            ids.choose_multiple(&mut rng, count.min(ids.len()))
                .copied()
                .collect()
        }
    }

    let cache = RandomCache::new();

    // キャッシュを初期化
    let start = Instant::now();
    cache.refresh(pool).await?;
    println!("  キャッシュ初期化: {:?}", start.elapsed());

    // ランダム取得（O(1)）
    let start = Instant::now();
    let random_ids = cache.get_random(5).await;
    println!("  ランダム取得（キャッシュ）: {:?}", start.elapsed());
    println!("    取得したID: {:?}", random_ids);

    // 複数回取得してもO(1)
    let start = Instant::now();
    for _ in 0..10 {
        let _ = cache.get_random(5).await;
    }
    println!("  10回ランダム取得: {:?}\n", start.elapsed());

    Ok(())
}

/// 6. 接続プールモニタリング
async fn demo_connection_pool(pool: &PgPool) -> Result<()> {
    println!("--- 6. 接続プールモニタリング ---");

    // プールの状態を確認
    let size = pool.size();
    let idle = pool.num_idle();
    let max_connections = pool.options().get_max_connections();

    println!("  プール状態:");
    println!("    現在の接続数: {}", size);
    println!("    アイドル接続: {}", idle);
    println!("    アクティブ接続: {}", size.saturating_sub(idle as u32));
    println!("    最大接続数: {}", max_connections);

    // 使用率の計算
    let active = size.saturating_sub(idle as u32);
    let usage = if max_connections > 0 {
        (active as f64 / max_connections as f64) * 100.0
    } else {
        0.0
    };
    println!("    使用率: {:.1}%", usage);

    // 接続を使用してみる
    println!("\n  接続の借用と解放:");
    {
        let start = Instant::now();
        let _conn = pool.acquire().await?;
        println!("    接続取得: {:?}", start.elapsed());
        println!("    接続中のサイズ: {}", pool.size());
        // _conn がスコープを抜けると自動的に返却される
    }
    println!("    解放後のアイドル: {}", pool.num_idle());

    // 健全性チェックの例
    println!("\n  健全性チェック:");
    if idle == 0 && size >= max_connections {
        println!("    ⚠️  警告: 接続プールが枯渇しています");
    } else if usage > 80.0 {
        println!("    ⚠️  注意: 接続プールの使用率が高いです");
    } else {
        println!("    ✓ 正常: 接続プールは健全です");
    }

    println!();
    Ok(())
}

/// 7. N+1問題の検出デモ
async fn demo_n_plus_one_detection(pool: &PgPool) -> Result<()> {
    println!("--- 7. N+1問題の検出 ---");

    // シンプルなクエリカウンター
    #[derive(Clone)]
    struct QueryCounter {
        count: Arc<AtomicUsize>,
        patterns: Arc<RwLock<HashMap<String, usize>>>,
    }

    impl QueryCounter {
        fn new() -> Self {
            Self {
                count: Arc::new(AtomicUsize::new(0)),
                patterns: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        fn increment(&self) {
            self.count.fetch_add(1, Ordering::Relaxed);
        }

        async fn record_pattern(&self, pattern: &str) {
            self.increment();
            let mut patterns = self.patterns.write().await;
            *patterns.entry(pattern.to_string()).or_insert(0) += 1;
        }

        fn get(&self) -> usize {
            self.count.load(Ordering::Relaxed)
        }

        fn reset(&self) {
            self.count.store(0, Ordering::Relaxed);
        }

        async fn report(&self) {
            let patterns = self.patterns.read().await;
            println!("    総クエリ数: {}", self.get());
            println!("    パターン別:");
            let mut sorted: Vec<_> = patterns.iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(a.1));
            for (pattern, count) in sorted.iter().take(5) {
                let warning = if **count > 5 {
                    " ⚠️ N+1の可能性"
                } else {
                    ""
                };
                println!("      {} 回: {}{}", count, pattern, warning);
            }
        }
    }

    let counter = QueryCounter::new();

    // N+1パターンのシミュレーション
    println!("\n  N+1パターン（アンチパターン）:");
    counter.reset();

    let articles: Vec<(i32, String, i32)> =
        sqlx::query_as("SELECT id, title, author_id FROM articles LIMIT 10")
            .fetch_all(pool)
            .await?;
    counter.record_pattern("SELECT articles").await;

    for (_, _, author_id) in &articles {
        let _author: (String,) = sqlx::query_as("SELECT name FROM authors WHERE id = $1")
            .bind(author_id)
            .fetch_one(pool)
            .await?;
        counter.record_pattern("SELECT author by id").await;
    }

    counter.report().await;
    println!("    → 11クエリ発行（1 + 10）");

    // 改善パターン
    println!("\n  改善パターン（JOINで一括取得）:");
    counter.reset();
    {
        let mut patterns = counter.patterns.write().await;
        patterns.clear();
    }

    let _results: Vec<(i32, String, String)> = sqlx::query_as(
        r#"
        SELECT a.id, a.title, au.name
        FROM articles a
        JOIN authors au ON a.author_id = au.id
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await?;
    counter.record_pattern("SELECT articles JOIN authors").await;

    counter.report().await;
    println!("    → 1クエリで完了");

    // PostgreSQLでの検出方法
    println!("\n  PostgreSQLでの検出:");

    // pg_stat_statementsが利用可能かチェック
    let ext_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements')",
    )
    .fetch_one(pool)
    .await?;

    if ext_exists.0 {
        println!("    pg_stat_statements: 有効");

        // 頻繁に実行されるクエリを検出
        let frequent_queries: Vec<(i64, String)> = sqlx::query_as(
            r#"
            SELECT calls, LEFT(query, 60) as query_preview
            FROM pg_stat_statements
            WHERE query NOT LIKE '%pg_stat%'
              AND calls > 1
            ORDER BY calls DESC
            LIMIT 5
            "#,
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        println!("    頻繁なクエリ:");
        for (calls, query) in &frequent_queries {
            let warning = if *calls > 10 { " ⚠️" } else { "" };
            println!("      {} 回: {}...{}", calls, query, warning);
        }
    } else {
        println!("    pg_stat_statements: 未インストール");
        println!("    インストール方法:");
        println!("      CREATE EXTENSION pg_stat_statements;");
        println!("      (postgresql.confでshared_preload_librariesの設定も必要)");
    }

    // 検出のベストプラクティス
    println!("\n  N+1検出のベストプラクティス:");
    println!("    1. 開発時: クエリカウンターで閾値（10クエリ/リクエスト）を監視");
    println!("    2. テスト時: assert_max_queriesでN+1を自動検出");
    println!("    3. コードレビュー: ループ内の.awaitをチェック");
    println!("    4. 本番環境: pg_stat_statementsで大量実行クエリを特定");
    println!();

    Ok(())
}

/// 8. Rust言語側でのN+1検出方法
async fn demo_rust_detection_methods(pool: &PgPool) -> Result<()> {
    println!("--- 8. Rust言語側でのN+1検出方法 ---");

    // ========================================
    // 方法1: tracingを使ったクエリログ
    // ========================================
    println!("\n  【方法1】tracingによるクエリログ:");
    println!("    sqlxはtracingクレートと統合されており、");
    println!("    RUST_LOG=sqlx=debug で全クエリをログ出力できる");
    println!();
    println!("    設定例:");
    println!("    ```rust");
    println!("    tracing_subscriber::fmt()");
    println!("        .with_env_filter(\"sqlx=debug\")");
    println!("        .init();");
    println!("    ```");

    // ========================================
    // 方法2: カスタムクエリラッパー
    // ========================================
    println!("\n  【方法2】カスタムクエリラッパー:");

    /// リクエストスコープのクエリトラッカー
    #[derive(Clone)]
    struct QueryTracker {
        queries: Arc<RwLock<Vec<QueryRecord>>>,
        threshold: usize,
    }

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct QueryRecord {
        sql_preview: String,
        duration_us: u64,
    }

    impl QueryTracker {
        fn new(threshold: usize) -> Self {
            Self {
                queries: Arc::new(RwLock::new(Vec::new())),
                threshold,
            }
        }

        async fn record(&self, sql: &str, duration: std::time::Duration) {
            let record = QueryRecord {
                sql_preview: sql.chars().take(50).collect(),
                duration_us: duration.as_micros() as u64,
            };
            self.queries.write().await.push(record);
        }

        async fn analyze(&self) -> N1Analysis {
            let queries = self.queries.read().await;
            let total = queries.len();

            // 同じSQLパターンの出現回数をカウント
            let mut pattern_counts: HashMap<String, usize> = HashMap::new();
            for q in queries.iter() {
                // パラメータを正規化
                let normalized = q
                    .sql_preview
                    .replace(char::is_numeric, "N")
                    .replace("'", "");
                *pattern_counts.entry(normalized).or_insert(0) += 1;
            }

            // 5回以上同じパターンがあればN+1の疑い
            let suspicious: Vec<_> = pattern_counts
                .into_iter()
                .filter(|(_, count)| *count >= 5)
                .collect();

            N1Analysis {
                total_queries: total,
                threshold_exceeded: total > self.threshold,
                suspicious_patterns: suspicious,
            }
        }

        #[allow(dead_code)]
        async fn reset(&self) {
            self.queries.write().await.clear();
        }
    }

    #[derive(Debug)]
    struct N1Analysis {
        total_queries: usize,
        threshold_exceeded: bool,
        suspicious_patterns: Vec<(String, usize)>,
    }

    let tracker = QueryTracker::new(10);

    // N+1パターンをシミュレート
    let start = Instant::now();
    let articles: Vec<(i32, i32)> = sqlx::query_as("SELECT id, author_id FROM articles LIMIT 10")
        .fetch_all(pool)
        .await?;
    tracker
        .record("SELECT id, author_id FROM articles", start.elapsed())
        .await;

    for (_, author_id) in &articles {
        let start = Instant::now();
        let _: Option<(String,)> = sqlx::query_as("SELECT name FROM authors WHERE id = $1")
            .bind(author_id)
            .fetch_optional(pool)
            .await?;
        tracker
            .record(
                &format!("SELECT name FROM authors WHERE id = {}", author_id),
                start.elapsed(),
            )
            .await;
    }

    let analysis = tracker.analyze().await;
    println!("    分析結果:");
    println!("      総クエリ数: {}", analysis.total_queries);
    println!(
        "      閾値超過: {} (閾値: 10)",
        if analysis.threshold_exceeded {
            "⚠️ Yes"
        } else {
            "No"
        }
    );
    if !analysis.suspicious_patterns.is_empty() {
        println!("      疑わしいパターン:");
        for (pattern, count) in &analysis.suspicious_patterns {
            println!(
                "        {} 回: {}...",
                count,
                &pattern[..pattern.len().min(40)]
            );
        }
    }

    // ========================================
    // 方法3: テスト用アサーション
    // ========================================
    println!("\n  【方法3】テスト用アサーション:");

    /// テスト用: クエリ数をアサートするガード
    #[allow(dead_code)]
    struct QueryGuard {
        tracker: QueryTracker,
        context: String,
        max_queries: usize,
    }

    #[allow(dead_code)]
    impl QueryGuard {
        fn new(context: &str, max_queries: usize) -> Self {
            Self {
                tracker: QueryTracker::new(max_queries),
                context: context.to_string(),
                max_queries,
            }
        }

        async fn check(&self) -> std::result::Result<(), String> {
            let analysis = self.tracker.analyze().await;
            if analysis.total_queries > self.max_queries {
                Err(format!(
                    "N+1 detected in '{}': {} queries (max: {})\nSuspicious: {:?}",
                    self.context,
                    analysis.total_queries,
                    self.max_queries,
                    analysis.suspicious_patterns
                ))
            } else {
                Ok(())
            }
        }
    }

    println!("    使用例:");
    println!("    ```rust");
    println!("    #[tokio::test]");
    println!("    async fn test_no_n_plus_one() {{");
    println!("        let guard = QueryGuard::new(\"get_posts_with_authors\", 2);");
    println!("        ");
    println!("        // テスト対象の関数を実行");
    println!("        let result = get_posts_with_authors(&pool).await;");
    println!("        ");
    println!("        // N+1チェック");
    println!("        guard.check().await.expect(\"N+1 detected!\");");
    println!("    }}");
    println!("    ```");

    // ========================================
    // 方法4: コンパイル時の警告（パターンマッチ）
    // ========================================
    println!("\n  【方法4】コードパターンによる静的検出:");
    println!("    以下のパターンを見つけたらN+1の可能性:");
    println!();
    println!("    ❌ 危険なパターン:");
    println!("    ```rust");
    println!("    for item in items {{");
    println!("        let x = sqlx::query!(...).fetch_one(pool).await?;  // ループ内await");
    println!("    }}");
    println!("    ```");
    println!();
    println!("    ✅ 安全なパターン:");
    println!("    ```rust");
    println!("    let ids: Vec<_> = items.iter().map(|i| i.id).collect();");
    println!("    let results = sqlx::query!(\"... WHERE id = ANY($1)\", &ids)");
    println!("        .fetch_all(pool).await?;");
    println!("    ```");

    // ========================================
    // 方法5: Dieselとの比較
    // ========================================
    println!("\n  【方法5】Dieselとの比較:");
    println!("    Dieselには関連データのEager Loading機能がある:");
    println!();
    println!("    ```rust");
    println!("    // Diesel: belonging_to で関連データを一括取得");
    println!("    let posts = posts::table.load::<Post>(&conn)?;");
    println!("    let comments = Comment::belonging_to(&posts)");
    println!("        .load::<Comment>(&conn)?");
    println!("        .grouped_by(&posts);");
    println!("    ```");
    println!();
    println!("    sqlxには同等の機能がないため、");
    println!("    手動でIN句やJOINを使う必要がある。");

    // ========================================
    // 方法6: ミドルウェアでの自動検出（Webフレームワーク）
    // ========================================
    println!("\n  【方法6】Webフレームワークミドルウェア:");
    println!("    Axum/Actix-webでリクエストごとにクエリ数を監視:");
    println!();
    println!("    ```rust");
    println!("    async fn n1_detection_middleware<B>(");
    println!("        State(tracker): State<QueryTracker>,");
    println!("        request: Request<B>,");
    println!("        next: Next<B>,");
    println!("    ) -> Response {{");
    println!("        tracker.reset().await;");
    println!("        let response = next.run(request).await;");
    println!("        ");
    println!("        let analysis = tracker.analyze().await;");
    println!("        if analysis.threshold_exceeded {{");
    println!("            tracing::warn!(");
    println!("                queries = analysis.total_queries,");
    println!("                \"Potential N+1 detected\"");
    println!("            );");
    println!("        }}");
    println!("        response");
    println!("    }}");
    println!("    ```");

    // ========================================
    // 推奨アプローチのまとめ
    // ========================================
    println!("\n  【推奨アプローチ】");
    println!("    ┌─────────────────────────────────────────────────────┐");
    println!("    │ 開発段階        │ 推奨手法                         │");
    println!("    ├─────────────────────────────────────────────────────┤");
    println!("    │ コーディング中   │ RUST_LOG=sqlx=debug でログ確認   │");
    println!("    │ コードレビュー   │ ループ内 .await パターンを検索   │");
    println!("    │ ユニットテスト   │ QueryGuard でクエリ数アサート    │");
    println!("    │ 統合テスト       │ pg_stat_statements で検証        │");
    println!("    │ 開発環境         │ ミドルウェアで閾値監視           │");
    println!("    │ 本番環境         │ pg_stat_statements + アラート    │");
    println!("    └─────────────────────────────────────────────────────┘");
    println!();

    Ok(())
}
