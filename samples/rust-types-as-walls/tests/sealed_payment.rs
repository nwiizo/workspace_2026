#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "tests keep assertions and fixture setup direct"
)]

use rust_types_as_walls::sealed_payment::{Authorized, Payment, PaymentMethod, audit_line};

#[test]
fn authorized_payment_can_only_advance_via_library_defined_states() {
    let authorized = Payment::<Authorized>::authorize("pay_001", PaymentMethod::Card);
    assert_eq!(authorized.state_label(), "authorized");

    let captured = authorized.capture();
    assert_eq!(captured.state_label(), "captured");
    assert_eq!(
        audit_line(&captured),
        "payment_id=pay_001 method=card state=captured"
    );
}

#[test]
fn audit_line_reflects_closed_enum_values() {
    let bank_transfer = Payment::<Authorized>::authorize("pay_002", PaymentMethod::BankTransfer);
    assert_eq!(
        audit_line(&bank_transfer),
        "payment_id=pay_002 method=bank-transfer state=authorized"
    );
}
