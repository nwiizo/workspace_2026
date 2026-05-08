use rust_types_as_walls::api_evolution::{BillingPlan, CheckoutSummary};

fn main() {
    let _ = CheckoutSummary {
        customer_email: "future@example.com".into(),
        plan: BillingPlan::Free,
    };
}
