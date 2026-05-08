//! 2024 edition で安定化した async closure (`async || { ... }`) の例。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `AsyncFnMut` で受ける (キャプチャを mut で動かすので `AsyncFn` ではなく)
//! - エラー型は `thiserror` で命名し、`&'static str` の濫用を避ける
//! - 書籍9章のリトライ/サーキットブレーカーで多用される `Fn() -> impl Future` 境界を
//!   `AsyncFnMut` に置き換える
//! - 試行回数情報をエラーに残し、観測性を上げる

#[derive(Debug, thiserror::Error)]
#[error("operation failed after {attempts} attempts: {last}")]
struct RetryError {
    attempts: u32,
    last: String,
}

async fn retry<F, T>(max_attempts: u32, mut op: F) -> Result<T, RetryError>
where
    F: AsyncFnMut() -> Result<T, String>,
{
    let mut attempts = 0u32;
    loop {
        attempts += 1;
        match op().await {
            Ok(v) => return Ok(v),
            Err(last) if attempts >= max_attempts => {
                return Err(RetryError { attempts, last });
            }
            Err(_) => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let mut counter = 0u32;
    // async closure は環境を `&mut` で借りられるのが利点。
    // 2021 edition の `|| async move { ... }` 形式では難しかった。
    let result = retry(5, async || {
        counter += 1;
        if counter < 3 {
            Err(format!("not yet (attempt {counter})"))
        } else {
            Ok::<&str, String>("done")
        }
    })
    .await;
    println!("retry result: {result:?}, attempts: {counter}");
}
