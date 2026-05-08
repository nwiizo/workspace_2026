//! 1章: `tokio::join!` で並行 HTTP リクエストを発射する例。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `reqwest = "0.12"` / `tokio = "1.40"` に更新
//! - panic ではなく `Result` で main から抜ける
//! - `Duration` 系の出力に `Instant::elapsed()` を使う (書籍と同じ)
//! - `unwrap_used = deny` を満たすためエラーは `?` で伝搬

use std::time::Instant;

#[derive(Debug, thiserror::Error)]
enum DemoError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}

#[tokio::main]
async fn main() -> Result<(), DemoError> {
    let url = "https://jsonplaceholder.typicode.com/posts/1";
    let start = Instant::now();

    // tokio::join! は4つのfutureを同時に進める。
    // 個別にawaitすると逐次実行になる点は2021と同じ。
    let (a, b, c, d) = tokio::join!(
        reqwest::get(url),
        reqwest::get(url),
        reqwest::get(url),
        reqwest::get(url),
    );
    let _ = (a?, b?, c?, d?);

    println!("4 requests took {} ms", start.elapsed().as_millis());
    Ok(())
}
