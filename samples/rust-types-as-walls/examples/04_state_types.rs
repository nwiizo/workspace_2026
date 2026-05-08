//! パターン1: 状態ごとに型を分ける。
//! 「検証前の注文」と「検証済みの注文」を別の型にして、検証をスキップしたコードを
//! コンパイル時に弾く。
//!
//! スライド「パターン1：状態ごとに型を分ける」「状態が違うなら、型を分ける」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use thiserror::Error;

#[derive(Debug, Clone)]
struct Item {
    price: u64,
}

// Before: 検証済みかどうかをフラグで持つ（実行時にしかチェックできない）。
#[allow(dead_code)]
mod before {
    use super::Item;

    struct Order {
        validated: bool,
        items: Vec<Item>,
    }

    fn calculate_total(order: &Order) -> u64 {
        // validated == false でも呼べてしまう。コンパイラは止めない。
        order.items.iter().map(|i| i.price).sum()
    }

    pub fn demo() {
        let order = Order {
            validated: false,
            items: vec![Item { price: 100 }],
        };
        // 検証していない注文にも `calculate_total` が呼べてしまう。
        let _ = calculate_total(&order);
    }
}

// After: 状態ごとに別の型にする。
mod after {
    use super::Item;
    use thiserror::Error;

    pub struct UnvalidatedOrder {
        pub items: Vec<Item>,
    }

    pub struct ValidatedOrder {
        items: Vec<Item>,
    }

    #[derive(Debug, Error)]
    pub enum OrderError {
        #[error("注文が空です")]
        Empty,
    }

    pub fn validate(o: UnvalidatedOrder) -> Result<ValidatedOrder, OrderError> {
        if o.items.is_empty() {
            return Err(OrderError::Empty);
        }
        Ok(ValidatedOrder { items: o.items })
    }

    pub fn calculate_total(o: &ValidatedOrder) -> u64 {
        o.items.iter().map(|i| i.price).sum()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    before::demo();

    let raw = after::UnvalidatedOrder {
        items: vec![Item { price: 100 }, Item { price: 200 }],
    };

    let valid = after::validate(raw)?;
    println!("検証後の合計: {}", after::calculate_total(&valid));

    // 次の行のコメントを外すとコンパイルエラー:
    //   error[E0308]: mismatched types
    //   expected `&ValidatedOrder`, found `&UnvalidatedOrder`
    // let _ = after::calculate_total(&after::UnvalidatedOrder { items: vec![] });

    Ok(())
}

// このブロックは型の区別がコンパイル時に効いていることを示すための説明。
#[derive(Debug, Error)]
enum _DummyError {
    #[error("未使用")]
    _Unused,
}
