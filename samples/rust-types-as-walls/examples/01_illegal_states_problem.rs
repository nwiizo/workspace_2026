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
/// `bool` 2つ・3状態・`Option` 1つで、表現できる状態は 2 * 2 * 3 * 2 = 24通り。
/// ドメイン上正しいのはそのうち数通りで、残りは「書けてしまう不正」だ。
#[allow(dead_code)]
#[derive(Debug)]
struct Order {
    is_paid: bool,
    payment_id: Option<PaymentId>,
    status: OrderStatus,
    verified_at: Option<DateTime<Utc>>,
}

/// 「実行時に守る」アプローチ。フィールド間の整合性をここでチェックする。
/// このやり方は3つの弱点を抱える: 呼び忘れる経路があれば素通りし、フィールドが
/// 増えるたびに条件が膨らみ、矛盾が分かるのは書いた瞬間ではなく実行時だ。
fn assert_consistent(order: &Order) -> Result<(), &'static str> {
    if order.is_paid && order.payment_id.is_none() {
        return Err("支払い済みなのに決済IDがない");
    }
    if matches!(order.status, OrderStatus::Verified) && order.verified_at.is_none() {
        return Err("認証済みなのに認証日時がない");
    }
    // 状態が増えるたびに、ここに条件が増えていく。
    Ok(())
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

    // 実行時バリデーションを「呼べば」捕まえられる。だが呼び忘れる経路があれば素通りする。
    match assert_consistent(&bad_order) {
        Err(reason) => println!("実行時チェックで初めて検出: {reason}"),
        Ok(()) => unreachable!(),
    }

    // 実運用では、このチェックを呼び忘れた経路を通ったレコードが静かにDBに残り、
    // 数ヶ月後に返金処理や再認証ポリシーで突然例外を投げる。
    // 後続のサンプル（04, 08）では、この矛盾を enum と状態型で「書けなく」していく。
}
