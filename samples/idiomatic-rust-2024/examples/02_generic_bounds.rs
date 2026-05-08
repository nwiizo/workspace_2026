//! ch2 / 2.1 Generics + 2.2 Traits。
//!
//! 2024 edition で適用したベストプラクティス:
//! - 戻り値の `impl Trait` は in-scope generic / lifetime を自動キャプチャ
//! - `where` 節を使い、関数シグネチャが横に伸びるのを防ぐ
//! - `Display` ではなく `std::fmt::Write` で alloc-free に組み立てる手も知っておく

use std::fmt::Display;

// 古典的な関数。`+ Display` は trait bound、where 節に出すと読みやすい。
fn label<T>(prefix: &str, value: T) -> String
where
    T: Display,
{
    format!("{prefix}: {value}")
}

// `impl Trait` を引数で使うと「ジェネリックの構文糖」として等価。
// 2024 edition でも意味は変わらない。
fn label_impl(prefix: &str, value: impl Display) -> String {
    format!("{prefix}: {value}")
}

// 戻り値の `impl Trait` でクロージャを返す。
// 2024 edition では `+ '_` を書かずとも `prefix` のライフタイムが自動キャプチャされる。
fn make_labeler(prefix: &str) -> impl Fn(u32) -> String {
    move |n| format!("{prefix}: {n}")
}

fn main() {
    println!("{}", label("count", 7));
    println!("{}", label_impl("count", 7));
    let l = make_labeler("count");
    println!("{}", l(7));
}
