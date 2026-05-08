//! ch6 / Library design (6.8 Don't break the user's code)。
//!
//! 2024 edition で適用したベストプラクティス:
//! - 公開 enum / struct には `#[non_exhaustive]` を付ける
//! - 利用者は `_` アームを書かざるをえなくなり、新 variant 追加で破壊的変更にならない
//! - ただし内部 (この crate 内) では `_` を強制されない。crate 外からの match のみ制約
//! - struct で使うと「新フィールド追加に強い」になる代わりに `..Default::default()` 等で
//!   構築する手間を要求できる

#[derive(Debug)]
#[non_exhaustive]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    // 将来 `Trace` や `Fatal` を追加しても、利用側は `_ => ...` を必ず書いているので
    // SemVer 上の破壊的変更にならない (minor で追加可能)。
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ClientConfig {
    pub timeout_ms: u64,
    pub retry: u32,
}

const fn label(level: &LogLevel) -> &'static str {
    // この crate 内なので `#[non_exhaustive]` は match 網羅性に効かない。
    // 全 variant を書ける。crate 外から match するときは `_` アームが必須になる。
    match level {
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    }
}

fn main() {
    let cfg = ClientConfig {
        timeout_ms: 1000,
        ..Default::default()
    };
    println!("{cfg:?}");
    for level in [LogLevel::Info, LogLevel::Warn, LogLevel::Error] {
        println!("{}", label(&level));
    }
}
