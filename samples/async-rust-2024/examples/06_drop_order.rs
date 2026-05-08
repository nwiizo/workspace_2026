//! 2024 edition の tail expression drop order 変更を体感する。
//!
//! 2021 edition では関数末尾の式の一時変数は、ローカル変数より「後」に drop された。
//! 2024 edition では「先」に drop される。
//! これは `MutexGuard` を抱えたまま `.await` する原書 11章のサンプル
//! (deadlock テスト) の挙動に直接効く。
//!
//! 2024 edition で適用したベストプラクティス:
//! - そもそもガードを `.await` を跨いで保持しない (`await_holding_lock = deny`)
//! - ガードは「明示的に scope で切る」または `drop(guard)` で意図を読者に伝える
//! - tokio の async-aware Mutex は cancel safety にコストがあるので、
//!   短いクリティカルセクションには `std::sync::Mutex` を使い、ロックは scope で閉じる

use std::sync::Mutex;

#[derive(Debug)]
struct Tag(&'static str);

impl Drop for Tag {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Tag {
    fn value(&self) -> u32 {
        println!("call value() on {}", self.0);
        42
    }
}

fn demo_tail_drop() -> u32 {
    let _local = Tag("local");
    // 末尾式 `Tag("temp").value()` を見る:
    // - `Tag("temp")` は呼び出し中だけ生きる一時値
    // - `.value()` 呼び出しが終わったあと、その一時値が drop される
    //   そのタイミングが edition で逆転する
    //
    // 2024 edition: drop temp -> drop local (末尾式の一時値が local より先に死ぬ)
    // 2021 edition: drop local -> drop temp (末尾式の一時値が local より後に死ぬ)
    Tag("temp").value()
}

async fn lock_then_await() {
    // ベストプラクティス: std::sync::Mutex のガードはスコープで閉じてから .await。
    // `await_holding_lock = deny` を満たし、deadlock リスクも消える。
    let m = Mutex::new(0u32);
    let snapshot = {
        // poison は本サンプルでは起こらないが、現実コードでは
        // `match` で扱うか `parking_lot::Mutex` など poison しない実装を選ぶ。
        let mut g = match m.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *g += 1;
        *g
        // ← ここで g が drop され、ロックが解放される
    };
    tokio::task::yield_now().await; // ロックを持ち越さずに await
    println!("snapshot after lock scope: {snapshot}");
}

#[tokio::main]
async fn main() {
    let n = demo_tail_drop();
    println!("returned {n}");
    lock_then_await().await;
}
