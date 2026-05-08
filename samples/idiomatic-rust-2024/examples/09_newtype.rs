//! ch5 / 5.7 Newtype pattern。
//!
//! 2024 edition で適用したベストプラクティス:
//! - 単純な意味ラベル (`UserId`, `OrderId`, `Email`) は newtype で型エラーに昇格
//! - `From` / `AsRef` を実装して人間 ergonomics を確保
//! - `#[derive(...)]` を最小限。`Display` は意図して書く (`Debug` とは違う出力にしたいので)
//! - 内部表現は `pub(crate)` に絞り、外には不変アクセサだけ晒す

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(String);

impl UserId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "user:{}", self.0)
    }
}

impl From<&str> for UserId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrderId(String);

impl OrderId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

fn lookup_user(id: &UserId) -> String {
    format!("looking up {id}")
}

fn main() {
    let u: UserId = "alice".into();
    let o = OrderId::new("o-1");
    println!("{}", lookup_user(&u));
    println!("{o:?}");

    // 別 newtype は型レベルで混ぜられない:
    // lookup_user(&o); // compile error
}
