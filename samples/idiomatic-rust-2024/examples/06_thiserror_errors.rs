//! ch4 / 4.5 Error handling。
//!
//! 2024 edition で適用したベストプラクティス:
//! - ライブラリは `thiserror` で型付きエラーを返す。アプリは `anyhow` を被せる
//! - `#[from]` で I/O エラーを透過変換
//! - `#[non_exhaustive]` で将来 variant を追加する余地を残す
//! - `unwrap_used = deny` を満たすため `?` で伝搬

use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LoadError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(#[from] ParseIntError),
    #[error("empty input")]
    Empty,
}

fn parse_first_number(text: &str) -> Result<i64, LoadError> {
    let line = text.lines().next().ok_or(LoadError::Empty)?;
    let n = line.trim().parse::<i64>()?; // ParseIntError -> LoadError 自動変換
    Ok(n)
}

fn main() -> Result<(), LoadError> {
    let n = parse_first_number("  42\nrest")?;
    println!("first = {n}");

    // Empty 例
    match parse_first_number("") {
        Ok(_) => println!("unexpected"),
        Err(e) => println!("expected error: {e}"),
    }

    // Parse 例
    match parse_first_number("not-a-number") {
        Ok(_) => println!("unexpected"),
        Err(e) => println!("expected error: {e}"),
    }
    Ok(())
}
