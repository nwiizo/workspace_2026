//! パターン4: Make Illegal States Unrepresentable (MIU)。
//! フラグ + Option の組み合わせ爆発を、enumで根本から解決する。
//! 型が「コンパイル時のユニットテスト」として働く。
//!
//! スライド「パターン4：フラグとOptionの組み合わせ爆発」
//! 「不正な状態を表現不可能にする」「型は、コンパイル時のユニットテストになる」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use chrono::{DateTime, Utc};

// Before: struct + Option。4通りの状態のうち 2通りは不正。
#[allow(dead_code)]
mod before {
    use chrono::{DateTime, Utc};

    pub struct User {
        pub email: String,
        pub is_verified: bool,
        pub verified_at: Option<DateTime<Utc>>,
    }

    pub fn demo() {
        // 以下はすべて型の上では合法だが、ドメインの目では不正:
        let _bad_1 = User {
            email: "a@b.com".into(),
            is_verified: true, // 認証済みのはずなのに
            verified_at: None, // 認証日時がない
        };
        let _bad_2 = User {
            email: "a@b.com".into(),
            is_verified: false,            // 未認証のはずなのに
            verified_at: Some(Utc::now()), // 認証日時がある
        };
    }
}

// After: enumで「同時に存在する」のではなく「どちらか一方」を表現する。
mod after {
    use chrono::{DateTime, Utc};

    pub enum User {
        Unverified {
            email: String,
        },
        Verified {
            email: String,
            verified_at: DateTime<Utc>,
        },
    }

    pub fn send_receipt(user: &User) {
        match user {
            User::Verified { email, verified_at } => {
                println!("{email} へ領収書送信 (認証日: {verified_at})");
            }
            User::Unverified { email } => {
                println!("{email} は未認証のため送信しない");
            }
        }
    }

    // コンパイル時のユニットテスト:
    // 「認証済みなのに verified_at が null」のケースは、そもそも書けない。
    // → テストを書く必要がない。型がテスト代わりになっている。
}

fn main() {
    before::demo();

    let verified = after::User::Verified {
        email: "verified@example.com".into(),
        verified_at: Utc::now(),
    };
    let unverified = after::User::Unverified {
        email: "pending@example.com".into(),
    };

    after::send_receipt(&verified);
    after::send_receipt(&unverified);

    // 次の行のコメントを外すとコンパイルエラー:
    //   エラー: Verified バリアントには verified_at が必須
    //   あるいは、Unverified バリアントには verified_at フィールドが存在しない
    // let bad = after::User::Verified { email: "x@y.com".into() };
    // let bad = after::User::Unverified { email: "x@y.com".into(), verified_at: Utc::now() };

    // match の網羅性もコンパイラが強制する:
    // 次の行のコメントを外すと「パターンが網羅されていない」エラーになる
    // match &verified {
    //     after::User::Verified { .. } => {}
    //     // Unverified を書き忘れている
    // }
    let _ = DateTime::<Utc>::default;
}
