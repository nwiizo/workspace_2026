//! 11章: 時間を制御した async テストの 2024 edition 流。
//!
//! ここで適用したベストプラクティス:
//! - `tokio::test(start_paused = true)` で時間を「進めない限り進まない」状態に固定
//! - `tokio::time::advance(Duration)` で時間を任意に進める
//! - これにより「時間に依存するロジック」を ms 待たずに検証できる
//! - `await_holding_lock = deny` を満たすため、ロックは scope で閉じてから await
//!
//! 本ファイルは下記を併置する。実運用では `cargo test` で回す形が本筋。
//!
//! - `fn main()` で `cargo run --example 10_paused_time_test` できる手動デモ
//! - `#[cfg(test)]` 内に `#[tokio::test(start_paused = true)]` のテスト本体

use std::time::Duration;
use tokio::time::{Instant, advance, sleep};

async fn fires_after(d: Duration) -> Instant {
    sleep(d).await;
    Instant::now()
}

#[tokio::main(flavor = "current_thread", start_paused = true)]
async fn main() {
    let start = Instant::now();
    let task = tokio::spawn(fires_after(Duration::from_secs(2)));

    // タスクがタイマー登録を済ませるまで一回だけ譲る。
    tokio::task::yield_now().await;
    advance(Duration::from_secs(2)).await;

    #[expect(
        clippy::expect_used,
        reason = "spawn 内 panic はテスト失敗として顕在化させたい"
    )]
    let fired_at = task.await.expect("spawned task panicked");
    let virtual_elapsed = fired_at.duration_since(start);
    println!("virtual elapsed = {} ms", virtual_elapsed.as_millis());
    // 期待: ~2000 ms (仮想時間)、実時間は ~0 ms
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo test --example 10_paused_time_test` で動く本物のテスト。
    /// `start_paused = true` で時計を停めた状態から開始し、
    /// `advance` で進めた時間ぶんだけ Future が進む。
    #[tokio::test(start_paused = true)]
    async fn fires_after_two_seconds_in_virtual_time() {
        let start = Instant::now();
        let task = tokio::spawn(fires_after(Duration::from_secs(2)));
        tokio::task::yield_now().await;
        advance(Duration::from_secs(2)).await;

        let fired_at = task.await.expect("spawned task panicked");
        let elapsed = fired_at.duration_since(start);
        assert!(
            (1990..=2010).contains(&u128::from(elapsed.as_millis())),
            "expected ~2000ms, got {}ms",
            elapsed.as_millis()
        );
    }
}
