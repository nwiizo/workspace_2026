//! 現象の提示: 型が「正しい形」を規定していないため、不正な状態が作れてしまう。
//!
//! スライド「不正な状態は、なぜ生まれるのか」「何が『不正』なのか」
//! 「なぜ、型はこれを止められなかったのか」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use chrono::{DateTime, Utc};

#[allow(dead_code)]
#[derive(Debug)]
struct PaymentId(String);

#[allow(dead_code)]
#[derive(Debug)]
enum OrderStatus {
    Pending,
    Verified,
    Shipped,
}

/// よく見かける「フラットなstruct」。
/// 個々のフィールドは単独では合法だが、組み合わせで矛盾を許してしまう。
#[allow(dead_code)]
#[derive(Debug)]
struct Order {
    is_paid: bool,
    payment_id: Option<PaymentId>,
    status: OrderStatus,
    verified_at: Option<DateTime<Utc>>,
}

fn main() {
    // 型の上では合法。でもドメインの目で見ると完全に矛盾しているレコード。
    let bad_order = Order {
        is_paid: true,                 // 支払い済み
        payment_id: None,              // でも決済IDがない
        status: OrderStatus::Verified, // 認証済み
        verified_at: None,             // でも認証日時がない
    };

    // コンパイラは何も言わない。
    // このコードは警告もエラーも出ずに通る。
    println!("不正なのにコンパイルが通ってしまう: {bad_order:?}");

    // 実運用では、こうしたレコードが静かにDBに残り、
    // 数ヶ月後に返金処理や再認証ポリシーで突然例外を投げる。
}
