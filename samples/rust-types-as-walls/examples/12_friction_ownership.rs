//! 摩擦1: 所有権と状態遷移の相性。
//! 状態遷移を「所有権ごと消費」で書くと、同じ値を参照で複数箇所から見ている場面で困る。
//! 回避策: Clone を挟む、Arc で共有する、借用ベースに切り替える。
//!
//! スライド「摩擦1：所有権と状態遷移は、常に同居できるわけではない」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone)]
struct UnvalidatedOrder {
    id: String,
    items: Vec<String>,
}

struct ValidatedOrder {
    id: String,
    items: Vec<String>,
}

#[derive(Debug, Error)]
enum OrderError {
    #[error("注文が空です")]
    Empty,
}

fn receive() -> UnvalidatedOrder {
    UnvalidatedOrder {
        id: "order_1".into(),
        items: vec!["book".into()],
    }
}

fn log_for_audit(o: &UnvalidatedOrder) {
    println!("監査ログ: order_id={}", o.id);
}

fn send_to_metrics(o: &UnvalidatedOrder) {
    println!("メトリクス送信: item_count={}", o.items.len());
}

fn validate(o: UnvalidatedOrder) -> Result<ValidatedOrder, OrderError> {
    if o.items.is_empty() {
        return Err(OrderError::Empty);
    }
    Ok(ValidatedOrder {
        id: o.id,
        items: o.items,
    })
}

fn show_validated(o: &ValidatedOrder) {
    println!("検証済み: order_id={} / item_count={}", o.id, o.items.len());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // パターンA: 借用で参照だけ渡す（ログ、メトリクスは参照でOK）
    let order = receive();
    log_for_audit(&order);
    send_to_metrics(&order);
    let valid = validate(order)?;
    show_validated(&valid);
    // ここで order は消費された。これ以上使えない。

    // パターンB: 複数の状態遷移や非同期処理で共有したい場合は Arc
    let shared = Arc::new(receive());
    let shared2 = Arc::clone(&shared);
    log_for_audit(&shared);
    send_to_metrics(&shared2);
    // Arc の中身を取り出して validate したい場合は (&*shared).clone() で値の複製が必要

    // パターンC: Clone を挟む（コストはかかるが素直）
    let original = receive();
    let copy = original.clone();
    let cloned_valid = validate(original)?;
    show_validated(&cloned_valid);
    log_for_audit(&copy);

    Ok(())
}
