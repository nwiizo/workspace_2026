//! 境界では、データは必ず生。
//! HTTPリクエストなど外部から入ってきた値は、型を貼る前の生データ。
//! 境界のただ1か所で parse を通し、以降は型付きの安全な値だけが流れる設計。
//!
//! スライド「境界では、データは必ず生」に対応。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use serde::Deserialize;
use thiserror::Error;

// --- 境界の外: 生データ（HTTPリクエストなど） ---

#[derive(Debug, Deserialize)]
struct CreateOrderRequest {
    customer_id: u64,
    items: Vec<ItemInput>,
    email: String,
}

#[derive(Debug, Deserialize)]
struct ItemInput {
    sku: String,
    qty: i32,
}

// --- 境界の内: 型で守られたドメインモデル ---

#[derive(Debug, Clone, Copy)]
struct CustomerId(u64);

impl CustomerId {
    fn new(n: u64) -> Result<Self, ApiError> {
        if n == 0 {
            return Err(ApiError::InvalidCustomerId);
        }
        Ok(CustomerId(n))
    }

    fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
struct Email(String);

impl Email {
    fn new(s: &str) -> Result<Self, ApiError> {
        if !s.contains('@') {
            return Err(ApiError::InvalidEmail);
        }
        Ok(Email(s.to_owned()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
struct Item {
    sku: String,
    qty: u32,
}

impl Item {
    fn sku(&self) -> &str {
        &self.sku
    }

    fn qty(&self) -> u32 {
        self.qty
    }
}

impl TryFrom<ItemInput> for Item {
    type Error = ApiError;

    fn try_from(input: ItemInput) -> Result<Self, Self::Error> {
        let ItemInput { sku, qty } = input;
        if qty <= 0 {
            return Err(ApiError::InvalidQuantity(sku));
        }
        let quantity = u32::try_from(qty).map_err(|_| ApiError::InvalidQuantity(sku.clone()))?;
        Ok(Item { sku, qty: quantity })
    }
}

#[derive(Debug)]
struct ValidatedOrder {
    customer: CustomerId,
    email: Email,
    items: Vec<Item>,
}

impl ValidatedOrder {
    fn new(customer: CustomerId, email: Email, items: Vec<Item>) -> Result<Self, ApiError> {
        if items.is_empty() {
            return Err(ApiError::EmptyOrder);
        }
        Ok(ValidatedOrder {
            customer,
            email,
            items,
        })
    }

    fn summary(&self) -> String {
        let items = self
            .items
            .iter()
            .map(|item| format!("{} x{}", item.sku(), item.qty()))
            .collect::<Vec<_>>();
        format!(
            "customer_id={} email={} items=[{}]",
            self.customer.value(),
            self.email.as_str(),
            items.join(", ")
        )
    }
}

#[derive(Debug, Error)]
enum ApiError {
    #[error("customer_id が不正です")]
    InvalidCustomerId,
    #[error("email が不正です")]
    InvalidEmail,
    #[error("数量が不正です: sku={0}")]
    InvalidQuantity(String),
    #[error("注文が空です")]
    EmptyOrder,
    #[error("JSONのパースに失敗: {0}")]
    Json(#[from] serde_json::Error),
}

// --- 境界: 生データを型付きドメインに変換するたった1か所 ---

fn create_order(req: CreateOrderRequest) -> Result<ValidatedOrder, ApiError> {
    let customer = CustomerId::new(req.customer_id)?;
    let email = Email::new(&req.email)?;
    let items = req
        .items
        .into_iter()
        .map(Item::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    ValidatedOrder::new(customer, email, items)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // シナリオ1: 正常なリクエスト
    let good_json = r#"{
        "customer_id": 1001,
        "email": "user@example.com",
        "items": [{"sku": "BOOK-001", "qty": 2}]
    }"#;
    let good_request: CreateOrderRequest = serde_json::from_str(good_json)?;
    let order = create_order(good_request)?;
    println!("受け付け成功: {}", order.summary());

    // シナリオ2: 不正なメールアドレス
    let bad_json = r#"{
        "customer_id": 1001,
        "email": "invalid-email",
        "items": [{"sku": "BOOK-001", "qty": 2}]
    }"#;
    let bad_request: CreateOrderRequest = serde_json::from_str(bad_json)?;
    match create_order(bad_request) {
        Err(e) => println!("境界で弾かれた: {e}"),
        Ok(_) => unreachable!(),
    }

    // シナリオ3: 数量が不正
    let bad_qty_json = r#"{
        "customer_id": 1001,
        "email": "user@example.com",
        "items": [{"sku": "BOOK-001", "qty": -1}]
    }"#;
    let bad_qty_request: CreateOrderRequest = serde_json::from_str(bad_qty_json)?;
    match create_order(bad_qty_request) {
        Err(e) => println!("境界で弾かれた: {e}"),
        Ok(_) => unreachable!(),
    }

    Ok(())
}
