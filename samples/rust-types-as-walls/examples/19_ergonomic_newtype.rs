//! newtype の安全性を保ったまま、`From` / `TryFrom` と `serde(transparent)` で
//! 境界の書き味を落とさない例。

#![allow(
    clippy::panic,
    clippy::print_stdout,
    reason = "examples print their narrative output and may use unreachable demo branches"
)]

use rust_types_as_walls::customer_id::{CustomerId, CustomerIdError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct CustomerResponse {
    id: CustomerId,
    loyalty_points: u32,
}

fn load_customer(id: u64) -> Result<CustomerResponse, CustomerIdError> {
    let customer_id = CustomerId::try_from(id)?;
    Ok(CustomerResponse {
        id: customer_id,
        loyalty_points: 120,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let response = load_customer(42)?;
    let raw_id: u64 = response.id.into();
    println!("legacy DB key として使う: {raw_id}");

    let json = serde_json::to_string(&response)?;
    println!("JSON では素の数値として出る: {json}");

    let decoded: CustomerResponse = serde_json::from_str(&json)?;
    println!(
        "復元した customer_id={} / points={}",
        decoded.id.get(),
        decoded.loyalty_points
    );

    let bad_json = r#"{"id":0,"loyalty_points":10}"#;
    match serde_json::from_str::<CustomerResponse>(bad_json) {
        Err(error) => println!("不正値を弾いた: {error}"),
        Ok(_) => unreachable!(),
    }

    Ok(())
}
