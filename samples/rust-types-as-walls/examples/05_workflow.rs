//! ワークフロー全体を型で貫く: 注文受付から支払い完了までのパイプラインを
//! 状態型で表現する。関数シグネチャが仕様書になる。
//!
//! スライド「ワークフロー全体を型で貫く」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use tap::prelude::*;
use thiserror::Error;

// --- ドメイン値 ---

type Money = u64;

#[derive(Debug, Clone)]
struct RawItem {
    sku: String,
    qty: i32,
}

#[derive(Debug, Clone)]
struct Item {
    sku: String,
    qty: u32,
    price: Money,
}

#[derive(Debug)]
struct PaymentId(String);

#[derive(Debug)]
struct Card {
    number: String,
}

// --- 状態ごとの型 ---

struct UnvalidatedOrder {
    items: Vec<RawItem>,
}

struct ValidatedOrder {
    items: Vec<Item>,
}

struct PricedOrder {
    items: Vec<Item>,
    subtotal: Money,
}

struct PaidOrder {
    items: Vec<Item>,
    subtotal: Money,
    payment: PaymentId,
}

// --- エラー ---

#[derive(Debug, Error)]
enum OrderError {
    #[error("注文が空です")]
    Empty,
    #[error("数量が不正です: sku={0}")]
    InvalidQuantity(String),
}

#[derive(Debug, Error)]
enum PaymentError {
    #[error("カード情報が不正です")]
    InvalidCard,
}

#[derive(Debug, Error)]
enum WorkflowError {
    #[error(transparent)]
    Order(#[from] OrderError),
    #[error(transparent)]
    Payment(#[from] PaymentError),
}

// --- 各ステップ ---

fn validate(o: UnvalidatedOrder) -> Result<ValidatedOrder, OrderError> {
    if o.items.is_empty() {
        return Err(OrderError::Empty);
    }

    let items = o
        .items
        .into_iter()
        .map(|raw_item| {
            let RawItem { sku, qty } = raw_item;
            if qty <= 0 {
                return Err(OrderError::InvalidQuantity(sku));
            }
            let quantity =
                u32::try_from(qty).map_err(|_| OrderError::InvalidQuantity(sku.clone()))?;
            Ok(Item {
                sku,
                qty: quantity,
                price: 1_000, // デモ用
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ValidatedOrder { items })
}

fn price(o: ValidatedOrder) -> PricedOrder {
    let subtotal = o
        .items
        .iter()
        .map(|item| item.price * u64::from(item.qty))
        .sum();
    PricedOrder {
        items: o.items,
        subtotal,
    }
}

fn charge(o: PricedOrder, card: &Card) -> Result<PaidOrder, PaymentError> {
    // デモ: カード番号が空なら失敗
    if card.number.is_empty() {
        return Err(PaymentError::InvalidCard);
    }
    Ok(PaidOrder {
        items: o.items,
        subtotal: o.subtotal,
        payment: PaymentId("pay_demo_123".into()),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw = UnvalidatedOrder {
        items: vec![RawItem {
            sku: "BOOK-001".into(),
            qty: 2,
        }],
    };
    let card = Card {
        number: "4242424242424242".into(),
    };
    let paid = raw
        .pipe(validate)
        .map(price)
        .map_err(WorkflowError::from)
        .and_then(|priced_order| charge(priced_order, &card).map_err(WorkflowError::from))?;

    let lines = paid
        .items
        .iter()
        .map(|item| {
            format!(
                "{} x{} = {}円",
                item.sku,
                item.qty,
                item.price * u64::from(item.qty)
            )
        })
        .collect::<Vec<_>>();
    println!("明細: {}", lines.join(", "));
    println!(
        "注文完了: 合計 {}円 / 決済ID {}",
        paid.subtotal, paid.payment.0
    );

    // 次の行のコメントを外すとコンパイルエラー:
    //   `price` は ValidatedOrder を期待するが、UnvalidatedOrder を渡している
    // let _ = price(UnvalidatedOrder { items: vec![] });

    Ok(())
}
