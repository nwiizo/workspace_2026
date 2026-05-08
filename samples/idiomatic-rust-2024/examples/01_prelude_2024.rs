//! ch1 / Rust 2024 prelude 追加: `Future` と `IntoFuture` が prelude に入った。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `use std::future::Future;` を書かなくてよい (prelude にある)
//! - 同じ名前の trait が user code にあると衝突する。`cargo fix --edition` 時に
//!   `rust_2024_prelude_collisions` が自動で完全修飾に書き換える
//! - `IntoFuture` 実装で「`.await` で値に変換できる型」を作るのが idiomatic

use std::pin::Pin;

#[derive(Debug)]
struct Greeting(String);

// `IntoFuture` 実装: Greeting を `.await` 可能にする。
// prelude に入ったので `use std::future::IntoFuture` 不要。
impl IntoFuture for Greeting {
    type Output = String;
    type IntoFuture = Pin<Box<dyn Future<Output = String> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { format!("hello, {}", self.0) })
    }
}

// 戻り値型として `impl Future` を書くときも `use Future` 不要。
#[expect(
    clippy::manual_async_fn,
    reason = "fn -> impl Future 形式のままでデモしたい"
)]
fn compute() -> impl Future<Output = u32> {
    async { 42 }
}

#[expect(
    clippy::print_stdout,
    reason = "examples では stdout を許す (lib では deny 推奨)"
)]
fn main() {
    // 簡易 block_on を std だけで書く代わりに、ここでは pollster 風に手書きせず、
    // futures_lite を使わないために単純に `tokio::runtime::Runtime` を作るか、
    // テストで verify するに留める。ここでは実行は省き、型チェックのみ。
    let _greeting = Greeting("world".into());
    let _f = compute();
    println!("compiled (prelude additions verified at type level)");
}
