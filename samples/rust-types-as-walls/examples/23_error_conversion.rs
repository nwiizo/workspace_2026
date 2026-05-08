//! `thiserror` の `#[from]` を使って、ドメインエラーを階層化しながら `?` で流す。
//! 「どの層で失敗したか」を失わずに、呼び出し側のコード量を増やさない。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use rust_types_as_walls::idiomatic_email::{Email, EmailError};
use thiserror::Error;

#[derive(Debug, Error)]
enum DomainError {
    #[error("数量は 1 以上でなければなりません")]
    InvalidQuantity,
    #[error("SKU が見つかりません: {0}")]
    UnknownSku(String),
    #[error(transparent)]
    InvalidEmail(#[from] EmailError),
}

#[derive(Debug, Error)]
enum PaymentError {
    #[error("カード番号が空です")]
    MissingCardNumber,
    #[error("高額決済のため承認待ちです")]
    RequiresManualReview,
}

#[derive(Debug, Error)]
enum CheckoutError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Payment(#[from] PaymentError),
}

#[derive(Debug, Error)]
enum ApiError {
    #[error(transparent)]
    Checkout(#[from] CheckoutError),
    #[error("JSON の読み取りに失敗しました: {0}")]
    Json(#[from] serde_json::Error),
}

fn lookup_price(sku: &str) -> Result<u64, DomainError> {
    match sku {
        "BOOK-001" => Ok(1_500),
        "PEN-001" => Ok(300),
        _ => Err(DomainError::UnknownSku(sku.to_owned())),
    }
}

fn checkout(email: &str, sku: &str, qty: i32, card_number: &str) -> Result<u64, CheckoutError> {
    let _email = Email::try_from(email).map_err(DomainError::from)?;
    if qty <= 0 {
        return Err(DomainError::InvalidQuantity.into());
    }

    let price = lookup_price(sku)?;
    let quantity = u64::try_from(qty).map_err(|_| DomainError::InvalidQuantity)?;
    let total = price * quantity;

    if card_number.trim().is_empty() {
        return Err(PaymentError::MissingCardNumber.into());
    }
    if total >= 10_000 {
        return Err(PaymentError::RequiresManualReview.into());
    }

    Ok(total)
}

fn handle_json_request(body: &str) -> Result<u64, ApiError> {
    #[derive(serde::Deserialize)]
    struct Request {
        email: String,
        sku: String,
        qty: i32,
        card_number: String,
    }

    let request: Request = serde_json::from_str(body)?;
    Ok(checkout(
        &request.email,
        &request.sku,
        request.qty,
        &request.card_number,
    )?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let total = handle_json_request(
        r#"{
            "email": "buyer@example.com",
            "sku": "BOOK-001",
            "qty": 2,
            "card_number": "4242424242424242"
        }"#,
    )?;
    println!("checkout total={total}");

    match handle_json_request(
        r#"{
            "email": "buyer@example.com",
            "sku": "UNKNOWN",
            "qty": 1,
            "card_number": "4242424242424242"
        }"#,
    ) {
        Err(error) => println!("domain failure keeps its cause: {error}"),
        Ok(_) => unreachable!(),
    }

    match handle_json_request(
        r#"{
            "email": "buyer@example.com",
            "sku": "BOOK-001",
            "qty": 8,
            "card_number": "4242424242424242"
        }"#,
    ) {
        Err(error) => println!("payment failure also bubbles up: {error}"),
        Ok(_) => unreachable!(),
    }

    Ok(())
}
