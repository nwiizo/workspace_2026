//! RFC 3498: lifetime capture rules 2024。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `-> impl Future + '_` の手書きは原則不要 (in-scope の generic / lifetime を
//!   自動キャプチャ)
//! - 逆に「捕捉したくない」場合のみ `+ use<>` で明示 opt-out できる
//! - `manual_async_fn` は `fn -> impl Future` 形式を保つ意図がある場合のみ allow
//!
//! 書籍 4章 (hyper 統合) や 10章 (std で async サーバー) で頻出する
//! 「Future を返す関数」 API の註釈を簡潔にできる。

use std::future::Future;

// 2021 edition だと「`s` のライフタイムが返り値に乗らない」と怒られて
// `-> impl Future<Output = usize> + '_` と書く必要があった。
// 2024 edition なら下記のままで通る。
// (clippy はこれを `async fn` に書き直せと言うが、ここは「fn -> impl Future」
// 形式の自動キャプチャを示すデモなので allow する。)
#[expect(
    clippy::manual_async_fn,
    reason = "edition 2024 のキャプチャ挙動を示すデモ"
)]
fn count_chars(s: &str) -> impl Future<Output = usize> {
    async move { s.chars().count() }
}

// 一方で `+ use<>` を使えば「明示的に何もキャプチャしない」も書ける。
// 'static な値しか触らないことをコンパイラと読者に伝えたいときに有効。
#[expect(clippy::manual_async_fn, reason = "use<> opt-out を見せるためのデモ")]
fn const_len() -> impl Future<Output = usize> + use<> {
    async { "edition2024".len() }
}

#[tokio::main]
async fn main() {
    let s = String::from("async-rust-2024");
    let n = count_chars(&s).await;
    let m = const_len().await;
    println!("len = {n}, const_len = {m}");
}
