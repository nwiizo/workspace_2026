//! イミュータビリティという壁: 変数も参照も、デフォルトで書き換え不可。
//! 関数型と同じく「入力を受けて新しい値を作る」発想が自然になる。
//!
//! スライド「イミュータビリティという壁」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

#[derive(Debug, Clone)]
struct Item {
    sku: String,
    price: u64,
}

#[derive(Debug, Clone)]
struct PricedOrder {
    items: Vec<Item>,
    subtotal: u64,
}

/// 関数型的な書き方: 新しい値を返す。元の値は不変。
fn apply_discount_pure(o: PricedOrder, rate_basis_points: u64) -> PricedOrder {
    let discounted_total = o
        .items
        .iter()
        .map(|item| item.price)
        .sum::<u64>()
        .saturating_mul(10_000 - rate_basis_points)
        .div_euclid(10_000);
    PricedOrder {
        items: o.items,
        subtotal: discounted_total,
    }
}

/// OOP 的な書き方: 既存の値を書き換える。`&mut self` が必須。
impl PricedOrder {
    fn apply_discount_inplace(&mut self, rate_basis_points: u64) {
        self.subtotal = self
            .subtotal
            .saturating_mul(10_000 - rate_basis_points)
            .div_euclid(10_000);
    }
}

fn main() {
    let order = PricedOrder {
        items: vec![Item {
            sku: "A".into(),
            price: 1_000,
        }],
        subtotal: 1_000,
    };

    // 変数は不変。次の行のコメントを外すとコンパイルエラーになる:
    //   error[E0596]: cannot borrow `order.items` as mutable
    // order.items.push(Item { sku: "B".into(), price: 500 });

    let skus = order
        .items
        .iter()
        .map(|item| item.sku.as_str())
        .collect::<Vec<_>>();
    println!("対象SKU: {skus:?}");

    // 関数型的な書き方: 新しい値を受け取る
    let discounted = apply_discount_pure(order.clone(), 1_000);
    println!(
        "純粋版: 元={} 割引後={}",
        order.subtotal, discounted.subtotal
    );

    // OOP 的な書き方: mut 明示が必要
    let mut mutable_order = order;
    mutable_order.apply_discount_inplace(1_000);
    println!("可変版: {}", mutable_order.subtotal);
}
