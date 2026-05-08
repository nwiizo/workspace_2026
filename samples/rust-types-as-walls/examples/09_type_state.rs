//! Rust固有のパターン: Type State + PhantomData。
//! 1つの構造体に対して型パラメータで状態を切り替える。
//! 実行時コストはゼロ（PhantomData は 0バイト）。
//!
//! スライド「Type State パターン：状態を型パラメータに乗せる」
//! 「Type State パターンのコード例」「Type Stateの効果と限界」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use std::marker::PhantomData;
use thiserror::Error;

// --- 状態マーカー型（値は作らない） ---

struct Unvalidated;
struct Validated;

// --- 本体 ---

#[derive(Debug, Clone)]
struct Item {
    price: u64,
}

struct Order<State> {
    items: Vec<Item>,
    _state: PhantomData<State>,
}

#[derive(Debug, Error)]
enum OrderError {
    #[error("注文が空です")]
    Empty,
}

impl Order<Unvalidated> {
    fn new(items: Vec<Item>) -> Self {
        Order {
            items,
            _state: PhantomData,
        }
    }

    fn validate(self) -> Result<Order<Validated>, OrderError> {
        if self.items.is_empty() {
            return Err(OrderError::Empty);
        }
        Ok(Order {
            items: self.items,
            _state: PhantomData,
        })
    }
}

impl Order<Validated> {
    fn total(&self) -> u64 {
        self.items.iter().map(|i| i.price).sum()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let unvalidated = Order::<Unvalidated>::new(vec![Item { price: 100 }, Item { price: 200 }]);

    // 次の行のコメントを外すとコンパイルエラー:
    //   `total` は Order<Validated> にしか実装されていない
    // let _ = unvalidated.total();

    let validated = unvalidated.validate()?;
    println!("検証後の合計: {}", validated.total());

    // PhantomData は 0バイトであることを確認
    println!(
        "Order<Validated> のサイズ: {} bytes (Vec<Item> と同じ = PhantomDataは0バイト)",
        std::mem::size_of::<Order<Validated>>()
    );
    println!(
        "Vec<Item> のサイズ: {} bytes",
        std::mem::size_of::<Vec<Item>>()
    );

    Ok(())
}
