//! ch10 / 10.3 Using `unwrap()` — antipattern としての `unwrap` を 2024 で書き直す。
//!
//! 2024 edition で適用したベストプラクティス:
//! - lib では `unwrap()` を `unwrap_used = deny` で禁止
//! - bin / examples / tests では `expect("理由")` を許容するが、`reason` を書く
//! - `Option` / `Result` は `?` / `let-else` / `ok_or` / `unwrap_or` で素直にハンドルする
//! - 「絶対 `None` ではない」と分かっている所は `.expect(static-msg)` で文書化

use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
enum ConfigError {
    #[error("missing key: {0}")]
    Missing(&'static str),
    #[error("invalid number: {0}")]
    InvalidNumber(String),
}

fn read_port(map: &HashMap<&str, String>) -> Result<u16, ConfigError> {
    // antipattern: map.get("port").unwrap().parse().unwrap()
    // idiomatic 2024:
    let raw = map.get("port").ok_or(ConfigError::Missing("port"))?;
    let port: u16 = raw
        .parse()
        .map_err(|_| ConfigError::InvalidNumber(raw.clone()))?;
    Ok(port)
}

fn first_word_or<'a>(s: &'a str, fallback: &'a str) -> &'a str {
    // antipattern: s.split_whitespace().next().unwrap()
    // idiomatic: unwrap_or
    s.split_whitespace().next().unwrap_or(fallback)
}

fn main() -> Result<(), ConfigError> {
    let mut map = HashMap::new();
    map.insert("port", "8080".to_string());
    println!("port = {}", read_port(&map)?);
    println!("first = {}", first_word_or("  hello world", "(none)"));
    println!("first = {}", first_word_or("   ", "(none)"));
    Ok(())
}
