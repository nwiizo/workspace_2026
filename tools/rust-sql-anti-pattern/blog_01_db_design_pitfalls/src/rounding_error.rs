//! アンチパターン4: ラウンディングエラー（FLOATの落とし穴）
//!
//! ## 問題の本質
//! - IEEE 754浮動小数点は2進数表現のため、多くの10進数を正確に表現できない
//! - 0.1, 0.2, 0.3 などの一般的な値が正確に格納できない
//! - 累積誤差、比較の不安定さ、丸め方法の違い
//!
//! ## 影響を受けるドメイン（金融だけではない）
//! - 金額・会計: 1円の誤差も許されない
//! - 税率・割引率: 正確な計算が必要
//! - 統計・分析: 累積誤差が結果に影響
//! - センサーデータ: 集計時に誤差が蓄積
//! - 座標データ: 精度要件に応じた型選択が必要
//!
//! ## PostgreSQL数値型
//! - FLOAT/REAL/DOUBLE PRECISION: IEEE 754浮動小数点（近似値、高速）
//! - NUMERIC/DECIMAL: 任意精度10進数（正確、やや遅い）
//! - BIGINT: 整数（正確、最速）
//!
//! ## 回避策
//! - 正確さが必要 → NUMERIC/DECIMAL + rust_decimal
//! - 近似値で十分 → DOUBLE PRECISION + f64
//! - 固定小数点 → BIGINT（セント単位など）

use anyhow::Result;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;

/// 問題のデモ: IEEE 754の基本的な問題
pub async fn demo_problem(pool: &PgPool) -> Result<()> {
    println!("--- 4. ラウンディングエラー（問題：IEEE 754） ---");

    // Rust側での問題
    println!("  Rust (f64) での問題:");
    let a = 0.1_f64;
    let b = 0.2_f64;
    let c = a + b;
    println!("    0.1 + 0.2 = {}", c);
    println!("    0.1 + 0.2 == 0.3 ? {}", c == 0.3);

    // PostgreSQL側での問題
    println!("\n  PostgreSQL での問題:");
    let row: (bool,) = sqlx::query_as("SELECT 0.1::float8 + 0.2::float8 = 0.3::float8")
        .fetch_one(pool)
        .await?;
    println!("    FLOAT: 0.1 + 0.2 = 0.3 ? {}", row.0);

    let row: (bool,) = sqlx::query_as("SELECT 0.1::numeric + 0.2::numeric = 0.3::numeric")
        .fetch_one(pool)
        .await?;
    println!("    NUMERIC: 0.1 + 0.2 = 0.3 ? {}", row.0);

    // 具体的な計算での問題
    println!("\n  計算での誤差:");
    sqlx::query("INSERT INTO products_bad (name, price) VALUES ($1, $2)")
        .bind("商品A")
        .bind(19.99_f64)
        .execute(pool)
        .await?;

    let row: (f64,) = sqlx::query_as("SELECT price * 3 FROM products_bad WHERE id = 1")
        .fetch_one(pool)
        .await?;
    println!("    FLOAT: 19.99 * 3 = {}", row.0);

    let row: (bool,) = sqlx::query_as("SELECT (price * 3) = 59.97 FROM products_bad WHERE id = 1")
        .fetch_one(pool)
        .await?;
    println!("    比較: 59.97 と等しい? {}", row.0);

    // DECIMAL で解決
    sqlx::query("INSERT INTO products_good (name, price) VALUES ($1, $2)")
        .bind("商品A")
        .bind(Decimal::from_str("19.99")?)
        .execute(pool)
        .await?;

    let row: (Decimal, Decimal) =
        sqlx::query_as("SELECT price * 3, price_with_tax FROM products_good WHERE id = 1")
            .fetch_one(pool)
            .await?;
    println!("\n  DECIMAL で解決:");
    println!("    19.99 * 3 = {}", row.0);
    println!("    税込価格(生成列): {}", row.1);
    println!();

    Ok(())
}

/// 回避策1: 累積誤差のデモと対策
pub async fn demo_cumulative_error(pool: &PgPool) -> Result<()> {
    println!("--- 4b. ラウンディングエラー（回避策：DECIMAL） ---");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tx_float (id SERIAL PRIMARY KEY, amount FLOAT NOT NULL)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS tx_decimal (id SERIAL PRIMARY KEY, amount DECIMAL(10,2) NOT NULL)")
        .execute(pool).await?;

    println!("  0.01を1000回加算:");

    for _ in 0..1000 {
        sqlx::query("INSERT INTO tx_float (amount) VALUES (0.01)")
            .execute(pool)
            .await?;
        sqlx::query("INSERT INTO tx_decimal (amount) VALUES (0.01)")
            .execute(pool)
            .await?;
    }

    let float_sum: (f64,) = sqlx::query_as("SELECT SUM(amount) FROM tx_float")
        .fetch_one(pool)
        .await?;
    let decimal_sum: (Decimal,) = sqlx::query_as("SELECT SUM(amount) FROM tx_decimal")
        .fetch_one(pool)
        .await?;

    println!(
        "    FLOAT:   {} (誤差: {})",
        float_sum.0,
        (float_sum.0 - 10.0).abs()
    );
    println!("    DECIMAL: {} (誤差: 0)", decimal_sum.0);

    println!("\n  消費税計算（10%）:");
    let price_float = 1234.56_f64;
    let total_float = price_float * 1.10;
    println!("    FLOAT: {}", total_float);

    let price_decimal = Decimal::from_str("1234.56")?;
    let total_decimal = (price_decimal * Decimal::from_str("1.10")?).round_dp(2);
    println!("    DECIMAL: {}", total_decimal);

    sqlx::query("DROP TABLE IF EXISTS tx_float, tx_decimal CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}

/// 回避策2: セント単位格納
pub async fn demo_cents_storage(pool: &PgPool) -> Result<()> {
    println!("--- 4c. ラウンディングエラー（回避策：セント単位） ---");

    sqlx::query("CREATE TABLE IF NOT EXISTS products_cents (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name TEXT NOT NULL, price_cents INTEGER NOT NULL)")
        .execute(pool).await?;

    sqlx::query("INSERT INTO products_cents (name, price_cents) VALUES ($1, $2), ($3, $4)")
        .bind("商品A")
        .bind(1999_i32) // $19.99
        .bind("商品B")
        .bind(2500_i32) // $25.00
        .execute(pool)
        .await?;

    fn format_price(cents: i32) -> String {
        format!("${}.{:02}", cents / 100, cents % 100)
    }

    let products: Vec<(String, i32)> =
        sqlx::query_as("SELECT name, price_cents FROM products_cents")
            .fetch_all(pool)
            .await?;

    println!("  セント単位格納:");
    for (name, cents) in &products {
        println!("    {}: {} → {}", name, cents, format_price(*cents));
    }

    let total: (i64,) = sqlx::query_as("SELECT SUM(price_cents) FROM products_cents")
        .fetch_one(pool)
        .await?;
    println!("  合計: {}", format_price(total.0 as i32));

    println!("\n  rust_decimal メソッド:");
    println!("    Decimal::new(1999, 2) = {}", Decimal::new(1999, 2));
    println!(
        "    round_dp(2): {}",
        Decimal::from_str("19.999")?.round_dp(2)
    );
    println!("    ZERO: {}, ONE: {}", Decimal::ZERO, Decimal::ONE);

    // 丸め戦略のデモ
    println!("\n  丸め戦略（RoundingStrategy）:");
    let value = Decimal::from_str("2.5")?;
    println!("    値: {}", value);
    println!(
        "    MidpointNearestEven (デフォルト): {}",
        value.round_dp(0)
    );
    println!(
        "    MidpointAwayFromZero (金融向け): {}",
        value.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
    );
    println!(
        "    ToZero (切り捨て): {}",
        value.round_dp_with_strategy(0, RoundingStrategy::ToZero)
    );

    // 消費税計算例
    println!("\n  消費税計算（丸め戦略指定）:");
    let price = Decimal::from_str("1234")?;
    let tax_rate = Decimal::from_str("0.10")?;
    let tax = price * tax_rate;
    let tax_rounded = tax.round_dp_with_strategy(0, RoundingStrategy::ToZero);
    println!(
        "    価格: {}, 税額: {} → {} (切り捨て)",
        price, tax, tax_rounded
    );
    println!("    合計: {}", price + tax_rounded);

    sqlx::query("DROP TABLE IF EXISTS products_cents CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}

/// 回避策3: FLOATが許容される場面のデモ
pub async fn demo_float_acceptable_cases(pool: &PgPool) -> Result<()> {
    println!("--- 4d. ラウンディングエラー（FLOATが許容される場面） ---");

    // センサーデータのデモ
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sensor_readings (
            id SERIAL PRIMARY KEY,
            temperature DOUBLE PRECISION NOT NULL,
            humidity DOUBLE PRECISION NOT NULL,
            recorded_at TIMESTAMPTZ DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    // 座標データのデモ
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS locations (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            latitude DOUBLE PRECISION NOT NULL,
            longitude DOUBLE PRECISION NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    println!("  センサーデータ（DOUBLE PRECISION）:");
    println!("    理由: センサー自体の測定誤差 > FLOAT誤差");

    // センサーデータを挿入
    for i in 0..5 {
        let temp = 23.5 + (i as f64) * 0.1;
        let humidity = 65.0 + (i as f64) * 0.5;
        sqlx::query("INSERT INTO sensor_readings (temperature, humidity) VALUES ($1, $2)")
            .bind(temp)
            .bind(humidity)
            .execute(pool)
            .await?;
    }

    let avg: (f64, f64) =
        sqlx::query_as("SELECT AVG(temperature), AVG(humidity) FROM sensor_readings")
            .fetch_one(pool)
            .await?;
    println!("    平均温度: {:.2}°C, 平均湿度: {:.1}%", avg.0, avg.1);

    println!("\n  座標データ（DOUBLE PRECISION）:");
    println!("    理由: 15桁精度で十分（1cm精度に必要なのは7桁）");

    // 座標データを挿入（東京駅）
    sqlx::query("INSERT INTO locations (name, latitude, longitude) VALUES ($1, $2, $3)")
        .bind("東京駅")
        .bind(35.681236_f64)
        .bind(139.767125_f64)
        .execute(pool)
        .await?;

    let loc: (String, f64, f64) =
        sqlx::query_as("SELECT name, latitude, longitude FROM locations WHERE id = 1")
            .fetch_one(pool)
            .await?;
    println!("    {}: ({}, {})", loc.0, loc.1, loc.2);

    println!("\n  座標精度の目安:");
    println!("    ┌──────────────┬────────────┐");
    println!("    │ 小数点以下   │ 精度       │");
    println!("    ├──────────────┼────────────┤");
    println!("    │ 2桁          │ 約1.1km    │");
    println!("    │ 4桁          │ 約11m      │");
    println!("    │ 6桁          │ 約11cm     │");
    println!("    │ 8桁          │ 約1.1mm    │");
    println!("    └──────────────┴────────────┘");

    // 丸め方法の違いをデモ
    println!("\n  PostgreSQLの丸め方法の違い:");
    let rows: Vec<(f64, i64, i64)> = sqlx::query_as(
        "SELECT x::float8, round(x::numeric)::bigint, round(x::double precision)::bigint
         FROM (VALUES (-2.5), (-1.5), (1.5), (2.5)) AS t(x)",
    )
    .fetch_all(pool)
    .await?;

    println!("    ┌───────┬─────────┬───────┐");
    println!("    │ 値    │ NUMERIC │ FLOAT │");
    println!("    ├───────┼─────────┼───────┤");
    for (x, numeric_round, float_round) in &rows {
        println!(
            "    │ {:>5.1} │ {:>7} │ {:>5} │",
            x, numeric_round, float_round
        );
    }
    println!("    └───────┴─────────┴───────┘");
    println!("    NUMERIC: ゼロから遠い方へ");
    println!("    FLOAT: 最近偶数へ (Banker's Rounding)");

    sqlx::query("DROP TABLE IF EXISTS sensor_readings, locations CASCADE")
        .execute(pool)
        .await?;
    println!();

    Ok(())
}
