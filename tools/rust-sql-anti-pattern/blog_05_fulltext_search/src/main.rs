//! PostgreSQL全文検索 - アンチパターン検証コード（詳細版）
//!
//! このコードは以下のパターンと解決策を実演します:
//! 1. プアマンズサーチエンジン（LIKEの限界）
//!    → B-treeインデックスの制約 / 前方一致のみ
//! 2. tsvector/tsquery による全文検索
//!    → 重み付け(A,B,C,D) / トリガー自動更新 / ts_stat
//! 3. pg_trgm によるあいまい検索
//!    → similarity / word_similarity / GIN vs GiST
//! 4. 日本語検索の課題
//!    → pg_bigm / pgroonga / 外部エンジン連携
//! 5. 検索パフォーマンス比較
//!    → EXPLAIN ANALYZE / バッファ使用量

use anyhow::Result;
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

    println!("=== PostgreSQL全文検索 デモ ===\n");

    setup_tables(&pool).await?;
    insert_sample_data(&pool).await?;

    demo_like_limitations(&pool).await?;
    demo_fulltext_search(&pool).await?;
    demo_fulltext_advanced(&pool).await?;
    demo_trigram_search(&pool).await?;
    demo_trigram_advanced(&pool).await?;
    demo_japanese_search(&pool).await?;
    demo_performance_comparison(&pool).await?;

    cleanup_tables(&pool).await?;

    Ok(())
}

async fn setup_tables(pool: &PgPool) -> Result<()> {
    // pg_trgm拡張を有効化（あいまい検索用）
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_trgm")
        .execute(pool)
        .await?;

    // 記事テーブル
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS articles_search (
            id SERIAL PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            search_vector TSVECTOR
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 日本語テスト用テーブル
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS articles_japanese (
            id SERIAL PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 大量データ用テーブル
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS articles_large (
            id SERIAL PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            search_vector TSVECTOR
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_sample_data(pool: &PgPool) -> Result<()> {
    // 英語記事サンプル
    let articles = [
        ("Introduction to Rust Programming", "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety."),
        ("PostgreSQL Full-Text Search", "PostgreSQL provides powerful full-text search capabilities including tsvector, tsquery, and ranking functions."),
        ("Building Web Applications", "Learn how to build web applications using Rust with frameworks like Actix-web and Axum."),
        ("Database Performance Optimization", "Optimize your database queries using proper indexing, query analysis, and connection pooling."),
        ("Rust and PostgreSQL Integration", "This article covers how to use Rust with PostgreSQL using the sqlx library for type-safe queries."),
    ];

    for (title, body) in &articles {
        sqlx::query(
            r#"
            INSERT INTO articles_search (title, body, search_vector)
            VALUES ($1, $2, to_tsvector('english', $1 || ' ' || $2))
            "#,
        )
        .bind(title)
        .bind(body)
        .execute(pool)
        .await?;
    }

    // 日本語記事サンプル
    let japanese_articles = [
        ("Rustプログラミング入門", "Rustは安全性とパフォーマンスを両立したシステムプログラミング言語です。"),
        ("PostgreSQLの全文検索機能", "PostgreSQLには強力な全文検索機能が備わっています。tsvectorとtsqueryを使用します。"),
        ("Webアプリケーション開発", "RustでWebアプリケーションを開発する方法を解説します。"),
    ];

    for (title, body) in &japanese_articles {
        sqlx::query("INSERT INTO articles_japanese (title, body) VALUES ($1, $2)")
            .bind(title)
            .bind(body)
            .execute(pool)
            .await?;
    }

    // 大量データ挿入
    sqlx::query(
        r#"
        INSERT INTO articles_large (title, body, search_vector)
        SELECT
            'Article ' || i || ': ' || CASE WHEN i % 5 = 0 THEN 'Rust' WHEN i % 3 = 0 THEN 'PostgreSQL' ELSE 'Programming' END,
            'This is the body of article ' || i || '. It contains various keywords like database, performance, optimization, and security.',
            to_tsvector('english', 'Article ' || i || ' body content database performance optimization security')
        FROM generate_series(1, 10000) AS i
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn cleanup_tables(pool: &PgPool) -> Result<()> {
    sqlx::query("DROP TABLE IF EXISTS articles_large CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS articles_japanese CASCADE")
        .execute(pool)
        .await?;
    sqlx::query("DROP TABLE IF EXISTS articles_search CASCADE")
        .execute(pool)
        .await?;
    Ok(())
}

/// 1. プアマンズサーチエンジン: LIKEの限界
async fn demo_like_limitations(pool: &PgPool) -> Result<()> {
    println!("--- 1. LIKEの限界 ---");

    // 前方一致（インデックス使用可能）
    let results: Vec<(String,)> =
        sqlx::query_as("SELECT title FROM articles_search WHERE title LIKE 'Rust%'")
            .fetch_all(pool)
            .await?;
    println!("  前方一致 'Rust%': {} 件", results.len());

    // 中間一致（インデックス使用不可）
    let results: Vec<(String,)> =
        sqlx::query_as("SELECT title FROM articles_search WHERE title LIKE '%Rust%'")
            .fetch_all(pool)
            .await?;
    println!("  中間一致 '%Rust%': {} 件", results.len());

    // 大文字小文字の問題
    let results: Vec<(String,)> =
        sqlx::query_as("SELECT title FROM articles_search WHERE title LIKE '%rust%'")
            .fetch_all(pool)
            .await?;
    println!("  小文字 '%rust%': {} 件 (大文字と一致しない)", results.len());

    // ILIKE（大文字小文字無視）
    let results: Vec<(String,)> =
        sqlx::query_as("SELECT title FROM articles_search WHERE title ILIKE '%rust%'")
            .fetch_all(pool)
            .await?;
    println!("  ILIKE '%rust%': {} 件", results.len());

    // 問題: ランキングができない
    println!("\n  LIKEの問題点:");
    println!("    - 中間一致はインデックスが効かない");
    println!("    - 大文字小文字の区別");
    println!("    - 関連度によるランキング不可");
    println!("    - 単語境界を考慮しない");

    println!();
    Ok(())
}

/// 2. tsvector/tsquery による全文検索
async fn demo_fulltext_search(pool: &PgPool) -> Result<()> {
    println!("--- 2. 全文検索 (tsvector/tsquery) ---");

    // GINインデックスの作成
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_search ON articles_search USING GIN(search_vector)")
        .execute(pool)
        .await?;

    // 基本的な検索
    let results: Vec<(String, f32)> = sqlx::query_as(
        r#"
        SELECT title, ts_rank(search_vector, query) as rank
        FROM articles_search, plainto_tsquery('english', 'rust programming') query
        WHERE search_vector @@ query
        ORDER BY rank DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("  'rust programming' 検索:");
    for (title, rank) in &results {
        println!("    [{:.4}] {}", rank, title);
    }

    // AND検索
    let results: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT title FROM articles_search
        WHERE search_vector @@ to_tsquery('english', 'rust & postgresql')
        "#,
    )
    .fetch_all(pool)
    .await?;
    println!("\n  'rust & postgresql' (AND検索): {} 件", results.len());

    // OR検索
    let results: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT title FROM articles_search
        WHERE search_vector @@ to_tsquery('english', 'rust | postgresql')
        "#,
    )
    .fetch_all(pool)
    .await?;
    println!("  'rust | postgresql' (OR検索): {} 件", results.len());

    // フレーズ検索
    let results: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT title FROM articles_search
        WHERE search_vector @@ phraseto_tsquery('english', 'full text search')
        "#,
    )
    .fetch_all(pool)
    .await?;
    println!("  'full text search' (フレーズ): {} 件", results.len());

    // ハイライト表示
    let results: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT title,
               ts_headline('english', body, plainto_tsquery('english', 'rust'),
                          'StartSel=<<, StopSel=>>, MaxWords=20')
        FROM articles_search
        WHERE search_vector @@ plainto_tsquery('english', 'rust')
        LIMIT 2
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\n  ハイライト表示:");
    for (title, headline) in &results {
        println!("    {}", title);
        println!("      {}", headline);
    }

    println!();
    Ok(())
}

/// 3. pg_trgm によるあいまい検索
async fn demo_trigram_search(pool: &PgPool) -> Result<()> {
    println!("--- 3. あいまい検索 (pg_trgm) ---");

    // トライグラムインデックスの作成
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_articles_title_trgm ON articles_search USING GIN(title gin_trgm_ops)",
    )
    .execute(pool)
    .await?;

    // 類似度検索
    let results: Vec<(String, f32)> = sqlx::query_as(
        r#"
        SELECT title, similarity(title, 'Rust Programing') as sim
        FROM articles_search
        WHERE similarity(title, 'Rust Programing') > 0.3
        ORDER BY sim DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("  'Rust Programing' (タイプミス) 類似度検索:");
    for (title, sim) in &results {
        println!("    [{:.2}] {}", sim, title);
    }

    // % 演算子による検索
    let results: Vec<(String,)> = sqlx::query_as(
        "SELECT title FROM articles_search WHERE title % 'Postgrseql'",
    )
    .fetch_all(pool)
    .await?;
    println!("\n  'Postgrseql' (タイプミス) %演算子: {} 件", results.len());

    // ILIKE の高速化
    println!("\n  pg_trgmによるILIKE高速化:");
    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT title FROM articles_search WHERE title ILIKE '%program%'",
    )
    .fetch_all(pool)
    .await?;
    for (line,) in &explain {
        println!("    {}", line);
    }

    println!();
    Ok(())
}

/// 4. 日本語検索の課題
async fn demo_japanese_search(pool: &PgPool) -> Result<()> {
    println!("--- 4. 日本語検索 ---");

    // デフォルトの問題
    println!("  デフォルトパーサーの問題:");
    let result: (String,) = sqlx::query_as(
        "SELECT to_tsvector('simple', 'Rustプログラミング入門')::text",
    )
    .fetch_one(pool)
    .await?;
    println!("    'simple': {}", result.0);

    // トライグラムによる日本語検索
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_japanese_title_trgm ON articles_japanese USING GIN(title gin_trgm_ops)",
    )
    .execute(pool)
    .await?;

    let results: Vec<(String, f32)> = sqlx::query_as(
        r#"
        SELECT title, similarity(title, 'プログラミング') as sim
        FROM articles_japanese
        WHERE title % 'プログラミング'
        ORDER BY sim DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    println!("\n  pg_trgmによる日本語検索 'プログラミング':");
    for (title, sim) in &results {
        println!("    [{:.2}] {}", sim, title);
    }

    // LIKEでの日本語検索
    let results: Vec<(String,)> = sqlx::query_as(
        "SELECT title FROM articles_japanese WHERE title LIKE '%Rust%'",
    )
    .fetch_all(pool)
    .await?;
    println!("\n  LIKE '%Rust%': {} 件", results.len());

    println!("\n  日本語検索の選択肢:");
    println!("    - pg_trgm: 汎用的、インデックス可能");
    println!("    - pg_bigm: 2-gramベース、日本語に適している");
    println!("    - 外部形態素解析: MeCab連携で高精度");
    println!("    - Meilisearch等: 外部検索エンジン連携");

    println!();
    Ok(())
}

/// 5. 検索パフォーマンス比較
async fn demo_performance_comparison(pool: &PgPool) -> Result<()> {
    println!("--- 5. パフォーマンス比較 ---");

    // インデックスなしLIKE
    let start = Instant::now();
    let _: Vec<(i32,)> =
        sqlx::query_as("SELECT id FROM articles_large WHERE body LIKE '%database%'")
            .fetch_all(pool)
            .await?;
    println!("  LIKE (インデックスなし): {:?}", start.elapsed());

    // GINインデックス作成
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_large_search ON articles_large USING GIN(search_vector)")
        .execute(pool)
        .await?;

    // 全文検索（GINインデックス使用）
    let start = Instant::now();
    let _: Vec<(i32,)> = sqlx::query_as(
        "SELECT id FROM articles_large WHERE search_vector @@ plainto_tsquery('english', 'database')",
    )
    .fetch_all(pool)
    .await?;
    println!("  全文検索 (GINインデックス): {:?}", start.elapsed());

    // EXPLAIN比較
    println!("\n  EXPLAIN (LIKE):");
    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT id FROM articles_large WHERE body LIKE '%database%'",
    )
    .fetch_all(pool)
    .await?;
    for (line,) in &explain {
        println!("    {}", line);
    }

    println!("\n  EXPLAIN (全文検索):");
    let explain: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN SELECT id FROM articles_large WHERE search_vector @@ plainto_tsquery('english', 'database')",
    )
    .fetch_all(pool)
    .await?;
    for (line,) in &explain {
        println!("    {}", line);
    }

    println!("\n  まとめ:");
    println!("    - 小規模データ: LIKEでも十分");
    println!("    - 中規模データ: pg_trgm + GINインデックス");
    println!("    - 大規模/高機能: tsvector + GIN または外部エンジン");

    println!();
    Ok(())
}

/// 2b. 全文検索の高度な機能
async fn demo_fulltext_advanced(pool: &PgPool) -> Result<()> {
    println!("--- 2b. 全文検索（高度な機能） ---");

    // 重み付き検索用テーブル
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS articles_weighted (
            id SERIAL PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            search_vector TSVECTOR
        )"
    )
    .execute(pool)
    .await?;

    // 重み付きtsvectorを作成（A: title, B: 未使用, C: 未使用, D: body）
    println!("  重み付き検索（A > B > C > D）:");
    sqlx::query(
        r#"
        INSERT INTO articles_weighted (title, body, search_vector)
        VALUES (
            'Rust Programming Guide',
            'This guide covers Rust programming basics and advanced topics.',
            setweight(to_tsvector('english', 'Rust Programming Guide'), 'A') ||
            setweight(to_tsvector('english', 'This guide covers Rust programming basics and advanced topics.'), 'D')
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO articles_weighted (title, body, search_vector)
        VALUES (
            'Web Development Tips',
            'Learn Rust for web development with various frameworks.',
            setweight(to_tsvector('english', 'Web Development Tips'), 'A') ||
            setweight(to_tsvector('english', 'Learn Rust for web development with various frameworks.'), 'D')
        )
        "#
    )
    .execute(pool)
    .await?;

    // 重み付きランキング
    let results: Vec<(String, f32)> = sqlx::query_as(
        r#"
        SELECT title, ts_rank(search_vector, query) as rank
        FROM articles_weighted, plainto_tsquery('english', 'rust') query
        WHERE search_vector @@ query
        ORDER BY rank DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    for (title, rank) in &results {
        println!("    [{:.4}] {}", rank, title);
    }
    println!("    → タイトルに 'Rust' がある方がランクが高い");

    // トリガーによる自動更新
    println!("\n  トリガーによる自動更新:");
    println!(r#"
    CREATE FUNCTION update_search_vector() RETURNS TRIGGER AS $$
    BEGIN
        NEW.search_vector :=
            setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
            setweight(to_tsvector('english', COALESCE(NEW.body, '')), 'D');
        RETURN NEW;
    END;
    $$ LANGUAGE plpgsql;

    CREATE TRIGGER tsvector_update BEFORE INSERT OR UPDATE
    ON articles FOR EACH ROW EXECUTE FUNCTION update_search_vector();
    "#);

    // ts_stat: 辞書の統計
    println!("  ts_stat（辞書統計）:");
    let results: Vec<(String, i32, i32)> = sqlx::query_as(
        "SELECT word, ndoc, nentry FROM ts_stat('SELECT search_vector FROM articles_search')
         ORDER BY nentry DESC LIMIT 5"
    )
    .fetch_all(pool)
    .await?;

    println!("    よく出現する単語:");
    for (word, ndoc, nentry) in &results {
        println!("      {} - {} 件 ({} 回)", word, ndoc, nentry);
    }

    // ts_debug: トークン化のデバッグ
    println!("\n  ts_debug（トークン化確認）:");
    let results: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT alias, description, token, lexemes::text FROM ts_debug('english', 'Rust is a systems programming language')"
    )
    .fetch_all(pool)
    .await?;

    for (alias, _desc, token, lexemes) in results.iter().take(3) {
        println!("    {} '{}' → {:?}", alias, token, lexemes);
    }

    // クリーンアップ
    sqlx::query("DROP TABLE IF EXISTS articles_weighted CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}

/// 3b. pg_trgm の高度な機能
async fn demo_trigram_advanced(pool: &PgPool) -> Result<()> {
    println!("--- 3b. pg_trgm（高度な機能） ---");

    // word_similarity: 単語単位での類似度
    println!("  word_similarity vs similarity:");
    let result: (f32, f32) = sqlx::query_as(
        "SELECT
            similarity('PostgreSQL Full-Text Search', 'PostgreSQL'),
            word_similarity('PostgreSQL Full-Text Search', 'PostgreSQL')"
    )
    .fetch_one(pool)
    .await?;
    println!("    similarity:      {:.3}", result.0);
    println!("    word_similarity: {:.3}", result.1);
    println!("    → word_similarity は単語境界を考慮");

    // strict_word_similarity: より厳密な単語類似度
    let result: (f32,) = sqlx::query_as(
        "SELECT strict_word_similarity('PostgreSQL', 'PostgreSQL Full-Text Search')"
    )
    .fetch_one(pool)
    .await?;
    println!("    strict_word_similarity: {:.3}", result.0);

    // GIN vs GiST インデックス
    println!("\n  GIN vs GiST インデックス:");
    println!("    ┌────────────┬─────────────────────┬─────────────────────┐");
    println!("    │ 観点       │ GIN                 │ GiST                │");
    println!("    ├────────────┼─────────────────────┼─────────────────────┤");
    println!("    │ 構築速度   │ △ 遅い             │ ◎ 速い             │");
    println!("    │ 検索速度   │ ◎ 高速             │ ○ 中程度           │");
    println!("    │ 更新速度   │ △ 遅い             │ ○ 中程度           │");
    println!("    │ サイズ     │ △ 大きい           │ ○ コンパクト       │");
    println!("    │ 使用場面   │ 読み取り多い       │ 更新が多い         │");
    println!("    └────────────┴─────────────────────┴─────────────────────┘");

    // 類似度しきい値の設定
    println!("\n  類似度しきい値:");
    let result: (f32,) = sqlx::query_as("SELECT show_trgm_wordlimit()")
        .fetch_one(pool)
        .await
        .unwrap_or((0.3,));
    println!("    デフォルト: 0.3");
    println!("    SET pg_trgm.similarity_threshold = 0.5;  -- より厳密に");

    // トライグラムの表示
    println!("\n  トライグラムの確認:");
    let result: (Vec<String>,) = sqlx::query_as(
        "SELECT show_trgm('Rust')::text[]"
    )
    .fetch_one(pool)
    .await?;
    println!("    'Rust' のトライグラム: {:?}", result.0);

    println!();
    Ok(())
}
