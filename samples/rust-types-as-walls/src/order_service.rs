//! 型パターンを統合したミニ注文サービス。

use std::num::NonZeroU64;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use derive_more::{AsRef, Display, From};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};
use thiserror::Error;

use crate::customer_id::{CustomerId, CustomerIdError};
use crate::idiomatic_email::{Email, EmailError};

pub async fn sqlite_memory_pool() -> Result<SqlitePool, DbError> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .map_err(DbError::from)
}

pub async fn app() -> Result<Router, ApiError> {
    let pool = sqlite_memory_pool().await?;
    app_with_pool(pool).await
}

pub async fn app_with_pool(pool: SqlitePool) -> Result<Router, ApiError> {
    let repository = OrderRepository::new(pool);
    repository.init().await?;

    let state = AppState {
        repository,
        next_id: Arc::new(AtomicU64::new(1)),
    };

    Ok(Router::new()
        .route("/orders", post(create_order))
        .route("/orders/{id}", get(get_order))
        .route("/orders/{id}/ship", post(ship_order))
        .with_state(state))
}

#[derive(Clone)]
struct AppState {
    repository: OrderRepository,
    next_id: Arc<AtomicU64>,
}

impl AppState {
    fn allocate_order_id(&self) -> Result<OrderId, DomainError> {
        let raw = self.next_id.fetch_add(1, Ordering::Relaxed);
        OrderId::try_from(raw).map_err(DomainError::from)
    }
}

#[derive(Debug, Deserialize)]
struct CreateOrderRequest {
    customer_id: u64,
    email: String,
    payment_method: String,
    items: Vec<CreateOrderItemRequest>,
}

#[derive(Debug, Deserialize)]
struct CreateOrderItemRequest {
    sku: String,
    quantity: i32,
}

#[derive(Debug, Serialize)]
struct OrderResponse {
    id: u64,
    customer_id: u64,
    email: String,
    status: String,
    payment_method: String,
    total_cents: Option<u64>,
    payment_reference: Option<String>,
    shipped_at: Option<String>,
    items: Vec<OrderItemResponse>,
}

#[derive(Debug, Serialize)]
struct OrderItemResponse {
    sku: String,
    quantity: u32,
    unit_price_cents: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error("リクエスト JSON が不正です: {0}")]
    InvalidJson(String),
}

impl From<JsonRejection> for ApiError {
    fn from(value: JsonRejection) -> Self {
        Self::InvalidJson(value.body_text())
    }
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Domain(DomainError::OrderNotReadyToShip | DomainError::AlreadyShipped) => {
                StatusCode::CONFLICT
            }
            Self::Domain(_) | Self::InvalidJson(_) => StatusCode::BAD_REQUEST,
            Self::Db(DbError::NotFound(_)) => StatusCode::NOT_FOUND,
            Self::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(ErrorResponse {
            error: self.to_string(),
        });
        (status, body).into_response()
    }
}

#[derive(Debug, Error)]
pub enum DomainError {
    #[error(transparent)]
    InvalidCustomerId(#[from] CustomerIdError),
    #[error(transparent)]
    InvalidEmail(#[from] EmailError),
    #[error(transparent)]
    InvalidOrderId(#[from] OrderIdError),
    #[error(transparent)]
    InvalidMoney(#[from] MoneyError),
    #[error("注文には 1 件以上の商品が必要です")]
    EmptyOrder,
    #[error("SKU は空にできません")]
    EmptySku,
    #[error("数量は 1 以上でなければなりません: sku={sku} qty={quantity}")]
    InvalidQuantity { sku: String, quantity: i32 },
    #[error("価格表に存在しない SKU です: {0}")]
    UnknownSku(String),
    #[error("payment_method が不正です: {0}")]
    InvalidPaymentMethod(String),
    #[error("支払い済み注文だけが出荷できます")]
    OrderNotReadyToShip,
    #[error("注文はすでに出荷済みです")]
    AlreadyShipped,
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("注文が見つかりません: {0}")]
    NotFound(OrderId),
    #[error("DB row の状態が不整合です: {0}")]
    RowInvariant(&'static str),
    #[error("未知の status です: {0}")]
    UnknownStatus(String),
    #[error("日時の復元に失敗しました: {0}")]
    InvalidTimestamp(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Display, AsRef)]
#[display("{_0}")]
pub struct OrderId(#[as_ref] NonZeroU64);

#[derive(Debug, Error)]
pub enum OrderIdError {
    #[error("order_id は 1 以上でなければなりません")]
    Zero,
    #[error("order_id は正の整数でなければなりません")]
    Parse,
    #[error("order_id が大きすぎます")]
    OutOfRange,
}

impl OrderId {
    fn get(self) -> u64 {
        self.0.get()
    }

    fn parse_text(value: &str) -> Result<Self, OrderIdError> {
        let raw = value.parse::<u64>().map_err(|_| OrderIdError::Parse)?;
        Self::try_from(raw)
    }
}

impl TryFrom<u64> for OrderId {
    type Error = OrderIdError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        NonZeroU64::new(value)
            .map(Self::from)
            .ok_or(OrderIdError::Zero)
    }
}

impl TryFrom<i64> for OrderId {
    type Error = OrderIdError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        let raw = u64::try_from(value).map_err(|_| OrderIdError::Parse)?;
        Self::try_from(raw)
    }
}

impl TryFrom<OrderId> for i64 {
    type Error = OrderIdError;

    fn try_from(value: OrderId) -> Result<Self, Self::Error> {
        i64::try_from(value.get()).map_err(|_| OrderIdError::OutOfRange)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(u64);

#[derive(Debug, Error)]
pub enum MoneyError {
    #[error("金額は 0 以上でなければなりません")]
    Negative,
    #[error("金額が大きすぎます")]
    OutOfRange,
    #[error("金額計算がオーバーフローしました")]
    Overflow,
}

impl Money {
    fn from_cents(cents: u64) -> Self {
        Self(cents)
    }

    fn cents(self) -> u64 {
        self.0
    }

    fn checked_mul(self, quantity: u32) -> Result<Self, MoneyError> {
        self.0
            .checked_mul(u64::from(quantity))
            .map(Self)
            .ok_or(MoneyError::Overflow)
    }

    fn checked_add(self, rhs: Self) -> Result<Self, MoneyError> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(MoneyError::Overflow)
    }
}

impl TryFrom<i64> for Money {
    type Error = MoneyError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        let cents = u64::try_from(value).map_err(|_| MoneyError::Negative)?;
        Ok(Self(cents))
    }
}

impl TryFrom<Money> for i64 {
    type Error = MoneyError;

    fn try_from(value: Money) -> Result<Self, Self::Error> {
        i64::try_from(value.cents()).map_err(|_| MoneyError::OutOfRange)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaymentMethod {
    Card,
    BankTransfer,
    Prepaid,
}

impl PaymentMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Card => "card",
            Self::BankTransfer => "bank_transfer",
            Self::Prepaid => "prepaid",
        }
    }
}

impl TryFrom<&str> for PaymentMethod {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim() {
            "card" => Ok(Self::Card),
            "bank_transfer" => Ok(Self::BankTransfer),
            "prepaid" => Ok(Self::Prepaid),
            other => Err(DomainError::InvalidPaymentMethod(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone)]
struct RawOrderLine {
    sku: String,
    quantity: i32,
}

#[derive(Debug, Clone)]
struct ValidatedOrderLine {
    sku: String,
    quantity: u32,
}

impl ValidatedOrderLine {
    fn new(sku: String, quantity: i32) -> Result<Self, DomainError> {
        let trimmed = sku.trim();
        if trimmed.is_empty() {
            return Err(DomainError::EmptySku);
        }

        let quantity_u32 = u32::try_from(quantity).map_err(|_| DomainError::InvalidQuantity {
            sku: trimmed.to_owned(),
            quantity,
        })?;
        if quantity_u32 == 0 {
            return Err(DomainError::InvalidQuantity {
                sku: trimmed.to_owned(),
                quantity,
            });
        }

        Ok(Self {
            sku: trimmed.to_owned(),
            quantity: quantity_u32,
        })
    }
}

#[derive(Debug, Clone)]
struct PricedOrderLine {
    sku: String,
    quantity: u32,
    unit_price: Money,
}

#[derive(Debug, Clone)]
struct PaymentReceipt {
    method: PaymentMethod,
    reference: String,
}

#[derive(Debug, Clone)]
pub struct Order<State> {
    id: OrderId,
    customer_id: CustomerId,
    email: Email,
    state: State,
}

pub type UnvalidatedOrder = Order<Unvalidated>;
pub type ValidatedOrder = Order<Validated>;
pub type PricedOrder = Order<Priced>;
pub type PaidOrder = Order<Paid>;
pub type ShippedOrder = Order<Shipped>;

#[derive(Debug, Clone)]
pub struct Unvalidated {
    items: Vec<RawOrderLine>,
    payment_method: PaymentMethod,
}

#[derive(Debug, Clone)]
pub struct Validated {
    items: Vec<ValidatedOrderLine>,
    payment_method: PaymentMethod,
}

#[derive(Debug, Clone)]
pub struct Priced {
    items: Vec<PricedOrderLine>,
    payment_method: PaymentMethod,
    total: Money,
}

#[derive(Debug, Clone)]
pub struct Paid {
    items: Vec<PricedOrderLine>,
    total: Money,
    payment: PaymentReceipt,
}

#[derive(Debug, Clone)]
pub struct Shipped {
    items: Vec<PricedOrderLine>,
    total: Money,
    payment: PaymentReceipt,
    shipped_at: DateTime<Utc>,
}

impl Order<Unvalidated> {
    fn from_request(id: OrderId, request: CreateOrderRequest) -> Result<Self, DomainError> {
        let customer_id = CustomerId::try_from(request.customer_id)?;
        let email = Email::try_from(request.email)?;
        let payment_method = PaymentMethod::try_from(request.payment_method.as_str())?;
        let items = request
            .items
            .into_iter()
            .map(|item| RawOrderLine {
                sku: item.sku,
                quantity: item.quantity,
            })
            .collect();

        Ok(Self {
            id,
            customer_id,
            email,
            state: Unvalidated {
                items,
                payment_method,
            },
        })
    }

    fn validate(self) -> Result<ValidatedOrder, DomainError> {
        if self.state.items.is_empty() {
            return Err(DomainError::EmptyOrder);
        }

        let items = self
            .state
            .items
            .into_iter()
            .map(|item| ValidatedOrderLine::new(item.sku, item.quantity))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Order {
            id: self.id,
            customer_id: self.customer_id,
            email: self.email,
            state: Validated {
                items,
                payment_method: self.state.payment_method,
            },
        })
    }
}

impl Order<Validated> {
    fn price(self) -> Result<PricedOrder, DomainError> {
        let mut total = Money::from_cents(0);
        let mut items = Vec::with_capacity(self.state.items.len());

        for item in self.state.items {
            let unit_price = catalog_price(&item.sku)?;
            total = total.checked_add(unit_price.checked_mul(item.quantity)?)?;
            items.push(PricedOrderLine {
                sku: item.sku,
                quantity: item.quantity,
                unit_price,
            });
        }

        Ok(Order {
            id: self.id,
            customer_id: self.customer_id,
            email: self.email,
            state: Priced {
                items,
                payment_method: self.state.payment_method,
                total,
            },
        })
    }
}

impl Order<Priced> {
    fn charge(self) -> PaidOrder {
        let reference = format!("pay_{}", self.id.get());

        Order {
            id: self.id,
            customer_id: self.customer_id,
            email: self.email,
            state: Paid {
                items: self.state.items,
                total: self.state.total,
                payment: PaymentReceipt {
                    method: self.state.payment_method,
                    reference,
                },
            },
        }
    }
}

impl Order<Paid> {
    fn ship(self, shipped_at: DateTime<Utc>) -> ShippedOrder {
        Order {
            id: self.id,
            customer_id: self.customer_id,
            email: self.email,
            state: Shipped {
                items: self.state.items,
                total: self.state.total,
                payment: self.state.payment,
                shipped_at,
            },
        }
    }
}

fn catalog_price(sku: &str) -> Result<Money, DomainError> {
    match sku {
        "BOOK-001" => Ok(Money::from_cents(1_500)),
        "PEN-001" => Ok(Money::from_cents(300)),
        "BAG-001" => Ok(Money::from_cents(4_800)),
        "STICKER-001" => Ok(Money::from_cents(200)),
        _ => Err(DomainError::UnknownSku(sku.to_owned())),
    }
}

#[derive(Debug, Clone)]
enum StoredOrder {
    Validated(ValidatedOrder),
    Priced(PricedOrder),
    Paid(PaidOrder),
    Shipped(ShippedOrder),
}

impl StoredOrder {
    fn to_response(&self) -> OrderResponse {
        match self {
            Self::Validated(order) => OrderResponse {
                id: order.id.get(),
                customer_id: order.customer_id.get(),
                email: order.email.as_str().to_owned(),
                status: "validated".to_owned(),
                payment_method: order.state.payment_method.as_str().to_owned(),
                total_cents: None,
                payment_reference: None,
                shipped_at: None,
                items: order
                    .state
                    .items
                    .iter()
                    .map(|item| OrderItemResponse {
                        sku: item.sku.clone(),
                        quantity: item.quantity,
                        unit_price_cents: None,
                    })
                    .collect(),
            },
            Self::Priced(order) => OrderResponse {
                id: order.id.get(),
                customer_id: order.customer_id.get(),
                email: order.email.as_str().to_owned(),
                status: "priced".to_owned(),
                payment_method: order.state.payment_method.as_str().to_owned(),
                total_cents: Some(order.state.total.cents()),
                payment_reference: None,
                shipped_at: None,
                items: order
                    .state
                    .items
                    .iter()
                    .map(|item| OrderItemResponse {
                        sku: item.sku.clone(),
                        quantity: item.quantity,
                        unit_price_cents: Some(item.unit_price.cents()),
                    })
                    .collect(),
            },
            Self::Paid(order) => OrderResponse {
                id: order.id.get(),
                customer_id: order.customer_id.get(),
                email: order.email.as_str().to_owned(),
                status: "paid".to_owned(),
                payment_method: order.state.payment.method.as_str().to_owned(),
                total_cents: Some(order.state.total.cents()),
                payment_reference: Some(order.state.payment.reference.clone()),
                shipped_at: None,
                items: order
                    .state
                    .items
                    .iter()
                    .map(|item| OrderItemResponse {
                        sku: item.sku.clone(),
                        quantity: item.quantity,
                        unit_price_cents: Some(item.unit_price.cents()),
                    })
                    .collect(),
            },
            Self::Shipped(order) => OrderResponse {
                id: order.id.get(),
                customer_id: order.customer_id.get(),
                email: order.email.as_str().to_owned(),
                status: "shipped".to_owned(),
                payment_method: order.state.payment.method.as_str().to_owned(),
                total_cents: Some(order.state.total.cents()),
                payment_reference: Some(order.state.payment.reference.clone()),
                shipped_at: Some(order.state.shipped_at.to_rfc3339()),
                items: order
                    .state
                    .items
                    .iter()
                    .map(|item| OrderItemResponse {
                        sku: item.sku.clone(),
                        quantity: item.quantity,
                        unit_price_cents: Some(item.unit_price.cents()),
                    })
                    .collect(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredLine {
    sku: String,
    quantity: u32,
    unit_price_cents: Option<u64>,
}

#[derive(Debug, Clone)]
struct OrderRow {
    id: i64,
    customer_id: i64,
    email: String,
    status: String,
    payment_method: String,
    items_json: String,
    total_cents: Option<i64>,
    payment_reference: Option<String>,
    shipped_at: Option<String>,
}

impl OrderRow {
    fn from_sql_row(row: SqliteRow) -> Result<Self, DbError> {
        Ok(Self {
            id: row.try_get("id")?,
            customer_id: row.try_get("customer_id")?,
            email: row.try_get("email")?,
            status: row.try_get("status")?,
            payment_method: row.try_get("payment_method")?,
            items_json: row.try_get("items_json")?,
            total_cents: row.try_get("total_cents")?,
            payment_reference: row.try_get("payment_reference")?,
            shipped_at: row.try_get("shipped_at")?,
        })
    }
}

impl TryFrom<&StoredOrder> for OrderRow {
    type Error = DbError;

    fn try_from(value: &StoredOrder) -> Result<Self, Self::Error> {
        match value {
            StoredOrder::Validated(order) => Ok(Self {
                id: i64::try_from(order.id).map_err(DomainError::from)?,
                customer_id: customer_id_to_i64(order.customer_id)?,
                email: order.email.as_str().to_owned(),
                status: "validated".to_owned(),
                payment_method: order.state.payment_method.as_str().to_owned(),
                items_json: serde_json::to_string(
                    &order
                        .state
                        .items
                        .iter()
                        .map(|item| StoredLine {
                            sku: item.sku.clone(),
                            quantity: item.quantity,
                            unit_price_cents: None,
                        })
                        .collect::<Vec<_>>(),
                )?,
                total_cents: None,
                payment_reference: None,
                shipped_at: None,
            }),
            StoredOrder::Priced(order) => Ok(Self {
                id: i64::try_from(order.id).map_err(DomainError::from)?,
                customer_id: customer_id_to_i64(order.customer_id)?,
                email: order.email.as_str().to_owned(),
                status: "priced".to_owned(),
                payment_method: order.state.payment_method.as_str().to_owned(),
                items_json: serde_json::to_string(
                    &order
                        .state
                        .items
                        .iter()
                        .map(|item| StoredLine {
                            sku: item.sku.clone(),
                            quantity: item.quantity,
                            unit_price_cents: Some(item.unit_price.cents()),
                        })
                        .collect::<Vec<_>>(),
                )?,
                total_cents: Some(i64::try_from(order.state.total).map_err(DomainError::from)?),
                payment_reference: None,
                shipped_at: None,
            }),
            StoredOrder::Paid(order) => Ok(Self {
                id: i64::try_from(order.id).map_err(DomainError::from)?,
                customer_id: customer_id_to_i64(order.customer_id)?,
                email: order.email.as_str().to_owned(),
                status: "paid".to_owned(),
                payment_method: order.state.payment.method.as_str().to_owned(),
                items_json: serde_json::to_string(
                    &order
                        .state
                        .items
                        .iter()
                        .map(|item| StoredLine {
                            sku: item.sku.clone(),
                            quantity: item.quantity,
                            unit_price_cents: Some(item.unit_price.cents()),
                        })
                        .collect::<Vec<_>>(),
                )?,
                total_cents: Some(i64::try_from(order.state.total).map_err(DomainError::from)?),
                payment_reference: Some(order.state.payment.reference.clone()),
                shipped_at: None,
            }),
            StoredOrder::Shipped(order) => Ok(Self {
                id: i64::try_from(order.id).map_err(DomainError::from)?,
                customer_id: customer_id_to_i64(order.customer_id)?,
                email: order.email.as_str().to_owned(),
                status: "shipped".to_owned(),
                payment_method: order.state.payment.method.as_str().to_owned(),
                items_json: serde_json::to_string(
                    &order
                        .state
                        .items
                        .iter()
                        .map(|item| StoredLine {
                            sku: item.sku.clone(),
                            quantity: item.quantity,
                            unit_price_cents: Some(item.unit_price.cents()),
                        })
                        .collect::<Vec<_>>(),
                )?,
                total_cents: Some(i64::try_from(order.state.total).map_err(DomainError::from)?),
                payment_reference: Some(order.state.payment.reference.clone()),
                shipped_at: Some(order.state.shipped_at.to_rfc3339()),
            }),
        }
    }
}

impl TryFrom<OrderRow> for StoredOrder {
    type Error = DbError;

    fn try_from(row: OrderRow) -> Result<Self, Self::Error> {
        let id = OrderId::try_from(row.id).map_err(DomainError::from)?;
        let customer_id = CustomerId::try_from(positive_u64(row.customer_id, "customer_id")?)
            .map_err(DomainError::from)?;
        let email = Email::try_from(row.email).map_err(DomainError::from)?;
        let payment_method = PaymentMethod::try_from(row.payment_method.as_str())?;
        let lines: Vec<StoredLine> = serde_json::from_str(&row.items_json)?;

        match row.status.as_str() {
            "validated" => {
                if row.total_cents.is_some()
                    || row.payment_reference.is_some()
                    || row.shipped_at.is_some()
                {
                    return Err(DbError::RowInvariant(
                        "validated row must not have total/payment/shipped columns",
                    ));
                }

                let items = lines
                    .into_iter()
                    .map(|line| {
                        if line.unit_price_cents.is_some() {
                            return Err(DbError::RowInvariant(
                                "validated items must not store unit_price_cents",
                            ));
                        }
                        let quantity = i32::try_from(line.quantity)
                            .map_err(|_| DbError::RowInvariant("quantity is too large"))?;
                        ValidatedOrderLine::new(line.sku, quantity).map_err(DbError::from)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(StoredOrder::Validated(Order {
                    id,
                    customer_id,
                    email,
                    state: Validated {
                        items,
                        payment_method,
                    },
                }))
            }
            "priced" => {
                let total_cents = row
                    .total_cents
                    .ok_or(DbError::RowInvariant("priced row must have total_cents"))?;
                if row.payment_reference.is_some() || row.shipped_at.is_some() {
                    return Err(DbError::RowInvariant(
                        "priced row must not have payment/shipped columns",
                    ));
                }

                let items = priced_items_from_lines(lines)?;
                Ok(StoredOrder::Priced(Order {
                    id,
                    customer_id,
                    email,
                    state: Priced {
                        items,
                        payment_method,
                        total: Money::try_from(total_cents).map_err(DomainError::from)?,
                    },
                }))
            }
            "paid" => {
                let total_cents = row
                    .total_cents
                    .ok_or(DbError::RowInvariant("paid row must have total_cents"))?;
                let payment_reference = row.payment_reference.ok_or(DbError::RowInvariant(
                    "paid row must have payment_reference",
                ))?;
                if row.shipped_at.is_some() {
                    return Err(DbError::RowInvariant("paid row must not have shipped_at"));
                }

                let items = priced_items_from_lines(lines)?;
                Ok(StoredOrder::Paid(Order {
                    id,
                    customer_id,
                    email,
                    state: Paid {
                        items,
                        total: Money::try_from(total_cents).map_err(DomainError::from)?,
                        payment: PaymentReceipt {
                            method: payment_method,
                            reference: payment_reference,
                        },
                    },
                }))
            }
            "shipped" => {
                let total_cents = row
                    .total_cents
                    .ok_or(DbError::RowInvariant("shipped row must have total_cents"))?;
                let payment_reference = row.payment_reference.ok_or(DbError::RowInvariant(
                    "shipped row must have payment_reference",
                ))?;
                let shipped_timestamp = row
                    .shipped_at
                    .ok_or(DbError::RowInvariant("shipped row must have shipped_at"))?;
                let shipped_at = DateTime::parse_from_rfc3339(&shipped_timestamp)
                    .map_err(|_| DbError::InvalidTimestamp(shipped_timestamp.clone()))?
                    .with_timezone(&Utc);

                let items = priced_items_from_lines(lines)?;
                Ok(StoredOrder::Shipped(Order {
                    id,
                    customer_id,
                    email,
                    state: Shipped {
                        items,
                        total: Money::try_from(total_cents).map_err(DomainError::from)?,
                        payment: PaymentReceipt {
                            method: payment_method,
                            reference: payment_reference,
                        },
                        shipped_at,
                    },
                }))
            }
            other => Err(DbError::UnknownStatus(other.to_owned())),
        }
    }
}

fn priced_items_from_lines(lines: Vec<StoredLine>) -> Result<Vec<PricedOrderLine>, DbError> {
    lines
        .into_iter()
        .map(|line| {
            let unit_price_cents = line.unit_price_cents.ok_or(DbError::RowInvariant(
                "priced items must have unit_price_cents",
            ))?;
            Ok(PricedOrderLine {
                sku: line.sku,
                quantity: line.quantity,
                unit_price: Money::from_cents(unit_price_cents),
            })
        })
        .collect()
}

fn positive_u64(value: i64, field: &'static str) -> Result<u64, DbError> {
    u64::try_from(value).map_err(|_| match field {
        "customer_id" => DbError::RowInvariant("customer_id must be >= 0"),
        _ => DbError::RowInvariant("numeric field must be >= 0"),
    })
}

fn customer_id_to_i64(value: CustomerId) -> Result<i64, DbError> {
    i64::try_from(value.get())
        .map_err(|_| DbError::RowInvariant("customer_id is too large to store"))
}

#[derive(Debug, Clone)]
struct OrderRepository {
    pool: SqlitePool,
}

impl OrderRepository {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn init(&self) -> Result<(), DbError> {
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY,
                customer_id INTEGER NOT NULL,
                email TEXT NOT NULL,
                status TEXT NOT NULL,
                payment_method TEXT NOT NULL,
                items_json TEXT NOT NULL,
                total_cents INTEGER,
                payment_reference TEXT,
                shipped_at TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn save(&self, order: &StoredOrder) -> Result<(), DbError> {
        let row = OrderRow::try_from(order)?;

        sqlx::query(
            r"
            INSERT INTO orders (
                id,
                customer_id,
                email,
                status,
                payment_method,
                items_json,
                total_cents,
                payment_reference,
                shipped_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                customer_id = excluded.customer_id,
                email = excluded.email,
                status = excluded.status,
                payment_method = excluded.payment_method,
                items_json = excluded.items_json,
                total_cents = excluded.total_cents,
                payment_reference = excluded.payment_reference,
                shipped_at = excluded.shipped_at
            ",
        )
        .bind(row.id)
        .bind(row.customer_id)
        .bind(row.email)
        .bind(row.status)
        .bind(row.payment_method)
        .bind(row.items_json)
        .bind(row.total_cents)
        .bind(row.payment_reference)
        .bind(row.shipped_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn load(&self, id: OrderId) -> Result<StoredOrder, DbError> {
        let row = sqlx::query(
            r"
            SELECT
                id,
                customer_id,
                email,
                status,
                payment_method,
                items_json,
                total_cents,
                payment_reference,
                shipped_at
            FROM orders
            WHERE id = ?
            ",
        )
        .bind(i64::try_from(id).map_err(DomainError::from)?)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DbError::NotFound(id))?;

        StoredOrder::try_from(OrderRow::from_sql_row(row)?)
    }
}

async fn create_order(
    State(state): State<AppState>,
    payload: Result<Json<CreateOrderRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<OrderResponse>), ApiError> {
    let request = payload.map_err(ApiError::from)?.0;
    let order_id = state.allocate_order_id()?;

    let paid = Order::<Unvalidated>::from_request(order_id, request)?
        .validate()?
        .price()?
        .charge();
    let stored = StoredOrder::Paid(paid);

    state.repository.save(&stored).await?;

    Ok((StatusCode::CREATED, Json(stored.to_response())))
}

async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, ApiError> {
    let order_id = OrderId::parse_text(&id).map_err(DomainError::from)?;
    let stored = state.repository.load(order_id).await?;
    Ok(Json(stored.to_response()))
}

async fn ship_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, ApiError> {
    let order_id = OrderId::parse_text(&id).map_err(DomainError::from)?;
    let loaded_order = state.repository.load(order_id).await?;

    let stored = match loaded_order {
        StoredOrder::Paid(order) => StoredOrder::Shipped(order.ship(Utc::now())),
        StoredOrder::Shipped(_) => return Err(DomainError::AlreadyShipped.into()),
        StoredOrder::Validated(_) | StoredOrder::Priced(_) => {
            return Err(DomainError::OrderNotReadyToShip.into());
        }
    };

    state.repository.save(&stored).await?;

    Ok(Json(stored.to_response()))
}
