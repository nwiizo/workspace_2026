//! 境界: DBスキーマとADTの往復。
//! DBのフラット形式（フラグ + NULL）と、ドメイン側のADT（enum）の間で
//! 双方向の変換を型安全に書く。
//!
//! スライド「DBスキーマとADTの往復」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use chrono::{DateTime, Utc};
use thiserror::Error;

// --- ドメイン側の型: ADT ---

#[derive(Debug, Clone)]
enum User {
    Unverified {
        email: String,
    },
    Verified {
        email: String,
        verified_at: DateTime<Utc>,
    },
}

// --- DB側の型: フラットなレコード ---

#[derive(Debug, Clone)]
struct UserRow {
    email: String,
    is_verified: bool,
    verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Error)]
enum DbError {
    #[error("DBの不整合: is_verified=true なのに verified_at が NULL")]
    VerifiedWithoutTimestamp,
    #[error("DBの不整合: is_verified=false なのに verified_at に値がある")]
    UnverifiedWithTimestamp,
}

// --- 書き込み: ADT → フラット ---

fn to_row(user: &User) -> UserRow {
    match user {
        User::Unverified { email } => UserRow {
            email: email.clone(),
            is_verified: false,
            verified_at: None,
        },
        User::Verified { email, verified_at } => UserRow {
            email: email.clone(),
            is_verified: true,
            verified_at: Some(*verified_at),
        },
    }
}

// --- 読み込み: フラット → ADT (失敗しうる) ---

fn from_row(row: UserRow) -> Result<User, DbError> {
    match (row.is_verified, row.verified_at) {
        (false, None) => Ok(User::Unverified { email: row.email }),
        (true, Some(at)) => Ok(User::Verified {
            email: row.email,
            verified_at: at,
        }),
        (true, None) => Err(DbError::VerifiedWithoutTimestamp),
        (false, Some(_)) => Err(DbError::UnverifiedWithTimestamp),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let verified = User::Verified {
        email: "v@example.com".into(),
        verified_at: Utc::now(),
    };
    let unverified = User::Unverified {
        email: "u@example.com".into(),
    };

    // ADT → DB (必ず成功)
    let row_v = to_row(&verified);
    let row_u = to_row(&unverified);
    println!("DB書き込み行: {row_v:?}");
    println!("DB書き込み行: {row_u:?}");

    // DB → ADT (不整合なら失敗)
    let verified_user = from_row(row_v)?;
    println!("DBから復元: {verified_user:?}");

    let unverified_user = from_row(row_u)?;
    println!("DBから復元: {unverified_user:?}");

    // 不整合行は Err
    let broken = UserRow {
        email: "broken@example.com".into(),
        is_verified: true,
        verified_at: None,
    };
    match from_row(broken) {
        Err(e) => println!("不整合を検出: {e}"),
        Ok(_) => unreachable!(),
    }

    Ok(())
}
