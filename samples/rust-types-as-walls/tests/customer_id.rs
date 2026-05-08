#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "tests keep assertions and fixture setup direct"
)]

use rust_types_as_walls::customer_id::{CustomerId, CustomerIdError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Payload {
    id: CustomerId,
    name: String,
}

#[test]
fn zero_is_rejected() {
    assert_eq!(CustomerId::try_from(0), Err(CustomerIdError::Zero));
}

#[test]
fn into_raw_u64_roundtrips() -> Result<(), CustomerIdError> {
    let id = CustomerId::try_from(42)?;
    let raw: u64 = id.into();
    assert_eq!(raw, 42);
    Ok(())
}

#[test]
fn serde_roundtrip_uses_plain_number() -> Result<(), Box<dyn std::error::Error>> {
    let payload = Payload {
        id: CustomerId::try_from(42)?,
        name: "Alice".into(),
    };

    let json = serde_json::to_string(&payload)?;
    assert_eq!(json, r#"{"id":42,"name":"Alice"}"#);

    let decoded: Payload = serde_json::from_str(&json)?;
    assert_eq!(decoded, payload);

    Ok(())
}

#[test]
fn serde_rejects_invalid_customer_ids() {
    let error = serde_json::from_str::<Payload>(r#"{"id":0,"name":"Alice"}"#)
        .expect_err("id=0 must be rejected");
    assert!(error.to_string().contains("1 以上"));
}
