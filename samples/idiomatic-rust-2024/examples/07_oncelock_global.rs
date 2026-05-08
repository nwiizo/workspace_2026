//! ch4 / 4.6 Global state。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `OnceLock<T>` (1.70+) は thread-safe な遅延初期化。`lazy_static!` は不要
//! - 同期初期化なら `LazyLock` (1.80+) のほうがクロージャ直書きで簡潔
//! - グローバル可変状態は基本避ける。設定値や `Regex` のような不変キャッシュに限定する

use std::sync::{LazyLock, OnceLock};

#[derive(Debug)]
pub struct Config {
    pub greeting: String,
    pub max_retries: u32,
}

// LazyLock: 初回アクセス時にクロージャを実行。書き味が短い。
static DEFAULT_CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    greeting: "hello".into(),
    max_retries: 3,
});

// OnceLock: 「外から後で一度だけ」入れ込みたい場合。例: アプリ起動時の動的設定。
static RUNTIME_CONFIG: OnceLock<Config> = OnceLock::new();

pub fn install_runtime_config(c: Config) -> Result<(), Config> {
    RUNTIME_CONFIG.set(c)
}

pub fn current() -> &'static Config {
    RUNTIME_CONFIG.get().unwrap_or(&DEFAULT_CONFIG)
}

fn main() {
    println!("before install: {}", current().greeting);
    let _ = install_runtime_config(Config {
        greeting: "yo".into(),
        max_retries: 5,
    });
    println!("after  install: {}", current().greeting);
}
