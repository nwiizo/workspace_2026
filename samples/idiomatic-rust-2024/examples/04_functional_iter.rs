//! ch3 / 3.2 Functional Rust。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `Iterator` チェーンで中間 `Vec` を作らない
//! - `try_fold` / `try_for_each` で「失敗したら短絡」を素直に書く
//! - 外部依存が増えるが、複雑な操作は `itertools` のほうが読みやすい
//!   (`chunk_by` / `tuple_windows` など)

use itertools::Itertools;

#[derive(Debug, thiserror::Error)]
enum ParseError {
    #[error("not a number: {0}")]
    NotANumber(String),
}

fn sum_lines(input: &str) -> Result<i64, ParseError> {
    // try_fold: 失敗したら即終了して Result を返す。
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .try_fold(0i64, |acc, line| {
            line.parse::<i64>()
                .map(|n| acc + n)
                .map_err(|_| ParseError::NotANumber(line.into()))
        })
}

fn pairs_sum(input: &[i64]) -> Vec<i64> {
    // tuple_windows で隣接ペアの和を取る。手書きインデックス操作不要。
    input
        .iter()
        .copied()
        .tuple_windows()
        .map(|(a, b)| a + b)
        .collect()
}

fn main() -> Result<(), ParseError> {
    let total = sum_lines("1\n2\n3\n  \n4")?;
    println!("sum = {total}");

    let pairs = pairs_sum(&[1, 2, 3, 4, 5]);
    println!("pair sums = {pairs:?}");
    Ok(())
}
