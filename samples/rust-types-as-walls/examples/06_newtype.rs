//! パターン2: newtypeで取り違えを防ぐ。
//! 同じ `u64` でも「顧客ID」と「注文ID」を別の型にすれば、
//! 引数の順序ミスがコンパイル時に検出される。
//!
//! スライド「パターン2：newtypeで取り違えを防ぐ」「単一フィールドの構造体で型を別にする」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

// Before: u64 が配られているコード
#[allow(dead_code)]
mod before {
    fn charge(_customer_id: u64, _order_id: u64) {
        // 何もしない
    }

    pub fn demo() {
        let customer: u64 = 1001;
        let order: u64 = 5678;

        // 引数の順序を間違えてもコンパイラは何も言わない。
        charge(order, customer);
    }
}

// After: newtypeで別の型にする。
mod after {
    #[derive(Debug, Clone, Copy)]
    pub struct CustomerId(pub u64);

    #[derive(Debug, Clone, Copy)]
    pub struct OrderId(pub u64);

    pub fn charge(customer: CustomerId, order: OrderId) {
        println!("顧客 {} の注文 {} に課金します", customer.0, order.0);
    }
}

fn main() {
    before::demo();

    let customer = after::CustomerId(1001);
    let order = after::OrderId(5678);

    // 正しい順序
    after::charge(customer, order);

    // 次の行のコメントを外すとコンパイルエラー:
    //   error[E0308]: mismatched types
    //   expected `CustomerId`, found `OrderId`
    // after::charge(order, customer);
}
