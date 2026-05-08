//! ch3 / 3.1 Pattern matching。
//!
//! 2024 edition で適用したベストプラクティス:
//! - `let-else` (1.65+) で「失敗時は早期 return / break / continue」を意図直結で書く
//! - `if let` chain (1.88+, 2024 edition で実用フェーズ) で `&&` 連結が自然になる
//! - `match` の `_` ワイルドカードは「将来の variant 追加で気づきたい」場面では避け、
//!   `#[non_exhaustive]` と組み合わせて明示する

#[derive(Debug)]
struct Order {
    id: u32,
    customer: Option<String>,
    items: Vec<String>,
}

fn customer_label(order: &Order) -> Option<String> {
    // let-else: customer が None ならその時点で None を返す。
    let Some(name) = &order.customer else {
        return None;
    };
    Some(format!("#{}: {name}", order.id))
}

fn first_premium_item(order: &Order) -> Option<&str> {
    // if let chain: 「最初の要素があり、かつ premium- で始まる」を一つの if に。
    // 2024 edition / 1.88+ で stable。
    if let Some(first) = order.items.first()
        && let Some(rest) = first.strip_prefix("premium-")
    {
        return Some(rest);
    }
    None
}

fn main() {
    let order = Order {
        id: 7,
        customer: Some("alice".into()),
        items: vec!["premium-coffee".into(), "tea".into()],
    };
    println!("label: {:?}", customer_label(&order));
    println!("premium: {:?}", first_premium_item(&order));

    let anon = Order {
        id: 8,
        customer: None,
        items: vec![],
    };
    println!("label (anon): {:?}", customer_label(&anon));
    println!("premium (anon): {:?}", first_premium_item(&anon));
}
