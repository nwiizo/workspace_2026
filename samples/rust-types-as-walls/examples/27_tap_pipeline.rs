//! `tap::Pipe` / `tap::Tap` で左から右へ流れるパイプラインを書く。
//! `05_workflow.rs` と同じ発想を、`pipe(validate).and_then(price).and_then(charge)`
//! の形に寄せて試す。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use tap::prelude::*;
use thiserror::Error;

#[derive(Debug)]
struct DraftOrder {
    email: String,
    skus: Vec<String>,
}

#[derive(Debug)]
struct ValidatedOrder {
    email: String,
    skus: Vec<String>,
}

#[derive(Debug)]
struct PricedOrder {
    email: String,
    lines: Vec<(String, u64)>,
    total: u64,
}

#[derive(Debug)]
struct PaidOrder {
    email: String,
    total: u64,
    payment_reference: String,
}

#[derive(Debug, Error)]
enum CheckoutError {
    #[error("email に @ がありません")]
    InvalidEmail,
    #[error("注文が空です")]
    EmptyOrder,
    #[error("SKU が未知です: {0}")]
    UnknownSku(String),
    #[error("カード番号が空です")]
    InvalidCard,
}

fn validate(order: DraftOrder) -> Result<ValidatedOrder, CheckoutError> {
    if !order.email.contains('@') {
        return Err(CheckoutError::InvalidEmail);
    }
    if order.skus.is_empty() {
        return Err(CheckoutError::EmptyOrder);
    }

    Ok(ValidatedOrder {
        email: order.email,
        skus: order.skus,
    })
}

fn lookup_price(sku: &str) -> Result<u64, CheckoutError> {
    match sku {
        "BOOK-001" => Ok(1_500),
        "PEN-001" => Ok(300),
        "BAG-001" => Ok(4_800),
        other => Err(CheckoutError::UnknownSku(other.to_owned())),
    }
}

fn price(order: ValidatedOrder) -> Result<PricedOrder, CheckoutError> {
    let priced_lines = order
        .skus
        .into_iter()
        .map(|sku| lookup_price(&sku).map(|unit_price| (sku, unit_price)))
        .collect::<Result<Vec<_>, _>>()?;
    let total = priced_lines.iter().map(|(_, unit_price)| *unit_price).sum();

    Ok(PricedOrder {
        email: order.email,
        lines: priced_lines,
        total,
    })
}

fn charge(order: PricedOrder, card_number: &str) -> Result<PaidOrder, CheckoutError> {
    if card_number.trim().is_empty() {
        return Err(CheckoutError::InvalidCard);
    }

    Ok(PaidOrder {
        email: order.email,
        total: order.total,
        payment_reference: format!("pay_{}", order.lines.len()),
    })
}

fn checkout_nested(order: DraftOrder, card_number: &str) -> Result<PaidOrder, CheckoutError> {
    let validated_order = validate(order)?;
    let priced_order = price(validated_order)?;
    charge(priced_order, card_number)
}

fn checkout_piped(order: DraftOrder, card_number: &str) -> Result<PaidOrder, CheckoutError> {
    order
        .pipe(validate)
        .tap_err(|error| println!("validate failed: {error}"))
        .and_then(price)
        .tap_ok(|priced_order| println!("priced total={}円", priced_order.total))
        .and_then(|priced_order| charge(priced_order, card_number))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nested = checkout_nested(
        DraftOrder {
            email: "buyer@example.com".to_owned(),
            skus: vec!["BOOK-001".to_owned(), "PEN-001".to_owned()],
        },
        "4242424242424242",
    )?;
    let piped = checkout_piped(
        DraftOrder {
            email: "buyer@example.com".to_owned(),
            skus: vec!["BOOK-001".to_owned(), "PEN-001".to_owned()],
        },
        "4242424242424242",
    )?;

    println!(
        "nested: email={} total={} payment={}",
        nested.email, nested.total, nested.payment_reference
    );
    println!(
        "piped: email={} total={} payment={}",
        piped.email, piped.total, piped.payment_reference
    );

    assert_eq!(nested.total, piped.total);
    assert_eq!(nested.payment_reference, piped.payment_reference);

    Ok(())
}
