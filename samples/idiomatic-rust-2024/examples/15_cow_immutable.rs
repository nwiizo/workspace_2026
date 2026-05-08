//! ch9 / 9.7 Using Cow for immutability。
//!
//! 2024 edition で適用したベストプラクティス:
//! - 「変更が必要なときだけ allocate」を `Cow<'_, T>` で表現
//! - 入力が触らずに済む場合は借用のまま返し、書き換えが必要なときだけ `to_mut`
//! - 戻り値型 `Cow<'a, str>` は呼び出し側で自然に扱える

use std::borrow::Cow;

/// 入力が大文字を含むならそのまま返す。含まないなら大文字化した新しい String を返す。
fn ensure_uppercase(input: &str) -> Cow<'_, str> {
    if input.chars().any(char::is_uppercase) {
        Cow::Borrowed(input)
    } else {
        Cow::Owned(input.to_uppercase())
    }
}

/// 設定の上書き。default のままなら借用、上書きが必要なら所有を作る。
fn resolve_greeting<'a>(default: &'a str, override_with: Option<&str>) -> Cow<'a, str> {
    override_with.map_or(Cow::Borrowed(default), |s| Cow::Owned(s.to_owned()))
}

fn main() {
    let a = ensure_uppercase("Hello");
    let b = ensure_uppercase("hello");
    println!("a = {a} (borrowed? {})", matches!(a, Cow::Borrowed(_)));
    println!("b = {b} (borrowed? {})", matches!(b, Cow::Borrowed(_)));

    let g1 = resolve_greeting("hi", None);
    let g2 = resolve_greeting("hi", Some("yo"));
    println!("g1 = {g1} (borrowed? {})", matches!(g1, Cow::Borrowed(_)));
    println!("g2 = {g2} (borrowed? {})", matches!(g2, Cow::Borrowed(_)));
}
