//! 設計を進化させるときも、型を作る。
//! 要件が増えたとき、既存の型にフィールドを足すのではなく、新しい型を作る。
//! コンパイラが依存箇所を全部追跡してくれる。
//!
//! スライド「設計を進化させるときも、型を作る」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

type Money = u64;

#[derive(Debug, Clone)]
struct Item {
    sku: String,
    price: Money,
}

#[derive(Debug, Clone)]
struct Address {
    zip: String,
}

#[derive(Debug, Clone)]
struct ShippingInfo {
    cost: Money,
    address: Address,
}

// Before: ValidatedOrder に Option フィールドを追加していく道。
// 状態が増えるごとに Option が増え、どの組み合わせが正しいか型からは読み取れなくなる。
mod before {
    use super::{Address, Item, Money};

    // 比較用に「Option が増えていく形」だけを置いている。
    #[allow(dead_code)]
    struct ValidatedOrder {
        items: Vec<Item>,
        shipping_cost: Option<Money>,      // 計算前は None
        shipping_address: Option<Address>, // 計算前は None
    }
}

// After: 状態ごとに新しい型を作る。
// `PricedOrderWithShipping` は `ShippingInfo` を必ず持つ。Option にしない。
#[derive(Debug)]
struct PricedOrder {
    items: Vec<Item>,
    subtotal: Money,
}

#[derive(Debug)]
struct PricedOrderWithShipping {
    items: Vec<Item>,
    subtotal: Money,
    shipping: ShippingInfo,
}

fn add_shipping(o: PricedOrder, shipping: ShippingInfo) -> PricedOrderWithShipping {
    PricedOrderWithShipping {
        items: o.items,
        subtotal: o.subtotal,
        shipping,
    }
}

fn items_subtotal(items: &[Item]) -> Money {
    items.iter().map(|item| item.price).sum()
}

fn total_with_shipping(o: &PricedOrderWithShipping) -> Money {
    items_subtotal(&o.items) + o.shipping.cost
}

fn describe_shipping(o: &PricedOrderWithShipping) -> String {
    let items = o
        .items
        .iter()
        .map(|item| format!("{}:{}円", item.sku, item.price))
        .collect::<Vec<_>>();
    format!(
        "宛先 {} / 明細 {}",
        o.shipping.address.zip,
        items.join(", ")
    )
}

fn main() {
    let priced = PricedOrder {
        items: vec![Item {
            sku: "A".into(),
            price: 1_000,
        }],
        subtotal: 1_000,
    };

    let with_ship = add_shipping(
        priced,
        ShippingInfo {
            cost: 500,
            address: Address {
                zip: "100-0001".into(),
            },
        },
    );

    println!("小計: {}", with_ship.subtotal);
    println!("配送料込みの合計: {}", total_with_shipping(&with_ship));
    println!("{}", describe_shipping(&with_ship));

    // 次の行のコメントを外すとコンパイルエラー:
    //   `total_with_shipping` は `&PricedOrderWithShipping` を要求する。
    //   配送情報が未確定の `PricedOrder` では呼べない。
    // let priced2 = PricedOrder { items: vec![], subtotal: 0 };
    // let _ = total_with_shipping(&priced2);
}
