//! 5章 Coroutines を 2024 edition で書くときの注意点。
//!
//! 2024 edition での制約:
//! - `Coroutine` トレイト本体 (`std::ops::Coroutine`) は依然 nightly。
//! - `gen` は予約語。書籍が頻繁に使う変数名 `let mut gen = ...` はそのままだとエラー。
//! - 同期 `gen { yield x; }` ブロックは nightly で stable 化作業中だが、
//!   stable 1.95 では使えない。stable での代替は手書き `Iterator`。
//!
//! 2024 edition で適用したベストプラクティス:
//! - 「resume できる状態機械」の本質は `Iterator` で表現する
//! - I/O 失敗は `?` で伝搬し、`unwrap()` を使わない
//! - reserved keyword 衝突を避けるため `coro` などにリネーム

use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines};

#[derive(Debug)]
struct LineNumbers {
    lines: Lines<BufReader<File>>,
}

impl LineNumbers {
    fn new(path: &str) -> io::Result<Self> {
        Ok(Self {
            lines: BufReader::new(File::open(path)?).lines(),
        })
    }
}

impl Iterator for LineNumbers {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        // パース失敗は素直に None で終了とする (書籍の Coroutine::Complete に対応)
        self.lines.next()?.ok()?.parse().ok()
    }
}

fn main() -> io::Result<()> {
    let path = "/tmp/async-rust-2024-data.txt";
    std::fs::write(path, "1\n2\n3\n4\n5\n")?;

    // NOTE: 原書は `let mut gen = ReadCoroutine::new(...)`
    // 2024 edition では `gen` が予約語なのでリネーム必須。
    let coro = LineNumbers::new(path)?;
    for n in coro {
        println!("{n}");
    }
    Ok(())
}
