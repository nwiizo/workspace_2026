#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "tests keep assertions and fixture setup direct"
)]

use rust_types_as_walls::api_evolution::{BillingPlan, CheckoutSummary, describe};

fn label(plan: BillingPlan) -> &'static str {
    match plan {
        BillingPlan::Free => "free",
        BillingPlan::Pro => "pro",
        _ => "future",
    }
}

#[test]
fn constructor_supports_forward_compatible_public_structs() {
    let summary = CheckoutSummary::new("future@example.com", BillingPlan::Pro);
    assert_eq!(summary.monthly_price(), 2_400);
    assert_eq!(label(summary.plan), "pro");
    assert_eq!(
        describe(&summary),
        "customer=future@example.com plan=Pro monthly_price=2400yen"
    );
}

#[test]
fn wildcard_match_still_handles_current_variants() {
    assert_eq!(label(BillingPlan::Free), "free");
    assert_eq!(label(BillingPlan::Pro), "pro");
}
