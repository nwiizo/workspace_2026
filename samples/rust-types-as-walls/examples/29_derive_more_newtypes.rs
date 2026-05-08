//! `derive_more` で newtype のボイラープレートを減らす。
//! `CustomerId` と `OrderId` は不変条件を `NonZeroU64` に寄せたうえで、
//! `Display` / `AsRef` / `From<NonZeroU64>` を derive している。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use std::num::NonZeroU64;

use rust_types_as_walls::customer_id::CustomerId;
use rust_types_as_walls::order_service::OrderId;

fn non_zero(value: u64) -> Result<NonZeroU64, std::io::Error> {
    NonZeroU64::new(value).ok_or_else(|| std::io::Error::other("0 は使えません"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let customer_seed = non_zero(42)?;
    let order_seed = non_zero(9001)?;

    let customer_id = CustomerId::from(customer_seed);
    let order_id = OrderId::from(order_seed);
    let customer_inner: &NonZeroU64 = customer_id.as_ref();
    let order_inner: &NonZeroU64 = order_id.as_ref();

    println!("Display derive: customer_id={customer_id} order_id={order_id}");
    println!(
        "AsRef derive: customer_inner={} order_inner={}",
        customer_inner.get(),
        order_inner.get()
    );

    let parsed_customer = CustomerId::try_from(7)?;
    let raw_customer: u64 = parsed_customer.into();
    println!("TryFrom は境界で維持しつつ、内側は From で楽に組み立てられる: {raw_customer}");

    assert_eq!(customer_id.get(), 42);
    assert_eq!(order_id.to_string(), "9001");
    assert_eq!(raw_customer, 7);

    Ok(())
}
