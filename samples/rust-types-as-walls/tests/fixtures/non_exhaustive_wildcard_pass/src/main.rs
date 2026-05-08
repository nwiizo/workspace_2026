use rust_types_as_walls::api_evolution::{BillingPlan, CheckoutSummary};

fn label(plan: BillingPlan) -> &'static str {
    match plan {
        BillingPlan::Free => "free",
        BillingPlan::Pro => "pro",
        _ => "future",
    }
}

fn main() {
    let summary = CheckoutSummary::new("future@example.com", BillingPlan::Free);
    let _ = label(summary.plan);
}
