//! ch8 / 8.3 Procedural macros の入口として、まずは declarative macro。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `macro_rules!` の `expr` matcher は 2024 edition で意味が拡張された。
//!   従来の `expr` は `expr_2021` に renamed され、新しい `expr` は `let` や `_` も許容する
//! - 既存マクロが `:expr` で受けていた箇所は edition 上げ時に挙動が変わる可能性あり
//! - `cargo fix --edition` が `expr` を `expr_2021` に書き換えてくれる
//!
//! 例: 「ラベル付きで時間を測る」マクロ。

macro_rules! timed {
    // expr matcher。2024 edition では新しい expr 規則。
    ($label:literal, $body:expr) => {{
        let start = std::time::Instant::now();
        let v = $body;
        println!("[{}] {} us", $label, start.elapsed().as_micros());
        v
    }};
}

fn fib(n: u32) -> u64 {
    if n < 2 {
        return u64::from(n);
    }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    let n = timed!("fib(20)", fib(20));
    println!("fib(20) = {n}");

    // expr 拡張のおかげで以下のような let-block も渡せる
    let s = timed!("compute", {
        let x = 1 + 2;
        x.to_string()
    });
    println!("s = {s}");
}
