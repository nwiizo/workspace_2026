//! 3章: 自作 async タスクキューの最小版を 2024 edition で。
//!
//! ここで適用したベストプラクティス:
//! - `LazyLock` でキューを 1 度だけ初期化 (1.80+ で stable)
//! - `let _ = catch_unwind(...)` で worker を防御
//! - `unwrap()` を `expect("queue closed")` に変えて停止理由を可視化
//! - **複数 worker** を立て、スレッド ID を出力して並行実行を可視化する

use std::future::Future;
use std::panic::catch_unwind;
use std::sync::LazyLock;
use std::thread;

use async_task::{Runnable, Task};
use futures_lite::future as flite;

const WORKER_COUNT: usize = 4;

static QUEUE: LazyLock<flume::Sender<Runnable>> = LazyLock::new(|| {
    let (tx, rx) = flume::unbounded::<Runnable>();
    for i in 0..WORKER_COUNT {
        let rx = rx.clone();
        #[expect(
            clippy::expect_used,
            reason = "worker thread の起動失敗は致命的なので panic でよい"
        )]
        thread::Builder::new()
            .name(format!("async-queue-worker-{i}"))
            .spawn(move || {
                while let Ok(runnable) = rx.recv() {
                    // future が panic しても worker は止めない。
                    let _ = catch_unwind(|| runnable.run());
                }
            })
            .expect("failed to spawn worker thread");
    }
    tx
});

fn spawn_task<F, T>(future: F) -> Task<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let schedule = |runnable| {
        #[expect(
            clippy::expect_used,
            reason = "queue クローズはランタイム停止と等価"
        )]
        QUEUE.send(runnable).expect("worker queue closed");
    };
    let (runnable, task) = async_task::spawn(future, schedule);
    runnable.schedule();
    task
}

async fn step(label: u32) -> u32 {
    let name = thread::current().name().unwrap_or("?").to_owned();
    println!("[{name}] step {label} begin");
    flite::yield_now().await;
    println!("[{name}] step {label} end");
    label
}

fn main() {
    // 8 個のタスクを 4 ワーカーで並行に進める。
    // スレッド ID の出力が交互に出れば並行動作が観察できる。
    let handles: Vec<_> = (0..8).map(|i| spawn_task(step(i))).collect();
    let total: u32 = flite::block_on(async {
        let mut sum = 0;
        for h in handles {
            sum += h.await;
        }
        sum
    });
    println!("total = {total}");
}
