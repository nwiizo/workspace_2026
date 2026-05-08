//! 2章: 手書き `Future` で `Pin` / `Context` / `Waker` を体感する。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `poll` 内の `std::thread::sleep` を撤去 (ランタイムスレッドをブロックする悪手)
//! - `JoinHandle` を `unwrap` せず `Result` でハンドリング
//! - `std::pin::pin!` マクロでスタック上に Future を pin (heap 不要)
//! - `cx.waker().wake_by_ref()` のビジー再ポーリング自体は edition 不変
//!
//! 書籍では各 future を `tokio::spawn` しているが、ここでは
//! 「同タスク内で複数 future を pin して進める」 std スタイルも併記する。

use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

#[derive(Debug)]
struct CounterFuture {
    count: u32,
    target: u32,
}

impl Future for CounterFuture {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.count += 1;
        println!("polling: {}", self.count);
        if self.count < self.target {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(self.count)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), tokio::task::JoinError> {
    // パターン1: tokio::spawn — 別タスクとしてランタイムに乗せる
    let one = tokio::spawn(CounterFuture {
        count: 0,
        target: 3,
    });
    let two = tokio::spawn(CounterFuture {
        count: 0,
        target: 3,
    });
    let (a, b) = tokio::join!(one, two);
    println!("spawn results: {} {}", a?, b?);

    // パターン2: pin! でスタックに置いて手元で await
    // Box::pin より軽く、`!Unpin` な future でも安全に await できる。
    let fut = pin!(CounterFuture {
        count: 0,
        target: 3
    });
    let n = fut.await;
    println!("pin! result: {n}");

    Ok(())
}
