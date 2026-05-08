//! 摩擦2: enumのバリアント間でフィールドが重複する。
//! Rustには「共通フィールド」を enum に直接書く機能がない。
//! 選択肢: 素直に繰り返す / 状態だけを enum にして struct に埋め込む。
//!
//! スライド「摩擦2：enumのバリアント間で、フィールドが重複する」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use chrono::Utc;

// 選択肢A: 素直に繰り返す。可読性は高いが DRY ではない。
mod variant_a {
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

    pub fn email_of(user: &User) -> &str {
        match user {
            User::Unverified { email } | User::Verified { email, .. } => email,
        }
    }

    pub fn verified_at(user: &User) -> Option<DateTime<Utc>> {
        match user {
            User::Unverified { .. } => None,
            User::Verified { verified_at, .. } => Some(*verified_at),
        }
    }
}

// 選択肢B: 状態だけを enum にする。共通フィールドが1箇所に集まる。
// トレードオフ: 「検証済みの場合のみ verified_at を取れる」という
// 型レベルの保証はやや弱くなる（UserState から取り出すときに match が必要）。
mod variant_b {
    use chrono::{DateTime, Utc};

    pub struct User {
        pub email: String,
        pub state: UserState,
    }

    pub enum UserState {
        Unverified,
        Verified(DateTime<Utc>),
    }

    pub fn email_of(user: &User) -> &str {
        &user.email
    }

    pub fn verified_at(user: &User) -> Option<DateTime<Utc>> {
        match user.state {
            UserState::Unverified => None,
            UserState::Verified(t) => Some(t),
        }
    }
}

fn main() {
    let a = variant_a::User::Verified {
        email: "a@example.com".into(),
        verified_at: Utc::now(),
    };
    let a2 = variant_a::User::Unverified {
        email: "pending-a@example.com".into(),
    };
    println!("A: email = {}", variant_a::email_of(&a));
    println!("A: verified_at = {:?}", variant_a::verified_at(&a));
    println!("A2: email = {}", variant_a::email_of(&a2));

    let b = variant_b::User {
        email: "b@example.com".into(),
        state: variant_b::UserState::Verified(Utc::now()),
    };
    let b2 = variant_b::User {
        email: "pending-b@example.com".into(),
        state: variant_b::UserState::Unverified,
    };
    println!("B: email = {}", variant_b::email_of(&b));
    println!("B: verified_at = {:?}", variant_b::verified_at(&b));
    println!("B2: email = {}", variant_b::email_of(&b2));
    println!("B2: verified_at = {:?}", variant_b::verified_at(&b2));
}
