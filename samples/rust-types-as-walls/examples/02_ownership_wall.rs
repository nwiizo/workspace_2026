//! 所有権という壁: 値を関数に渡すと所有権が移動し、呼び出し元では使えなくなる。
//! これを使うと「検証前の注文」を検証後に再利用できなくする、という保証を型で表現できる。
//!
//! スライド「所有権という壁」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use thiserror::Error;

#[derive(Debug, Clone)]
struct Item {
    #[allow(dead_code)]
    sku: String,
}

struct UnvalidatedOrder {
    items: Vec<Item>,
}

struct ValidatedOrder {
    items: Vec<Item>,
}

#[derive(Debug, Error)]
enum OrderError {
    #[error("注文が空です")]
    Empty,
}

fn validate(o: UnvalidatedOrder) -> Result<ValidatedOrder, OrderError> {
    if o.items.is_empty() {
        return Err(OrderError::Empty);
    }
    Ok(ValidatedOrder { items: o.items })
}

fn calculate_total(o: &ValidatedOrder) -> usize {
    o.items.len()
}

fn main() -> Result<(), OrderError> {
    let raw = UnvalidatedOrder {
        items: vec![Item { sku: "A".into() }, Item { sku: "B".into() }],
    };

    let valid = validate(raw)?;
    // ^ ここで `raw` の所有権は `validate` に移動した。

    println!("合計点数: {}", calculate_total(&valid));

    // 次の行のコメントを外すと、コンパイルエラーになる:
    //   error[E0382]: use of moved value: `raw`
    // let again = validate(raw)?;

    Ok(())
}
