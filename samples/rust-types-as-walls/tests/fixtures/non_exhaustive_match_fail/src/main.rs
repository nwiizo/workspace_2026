use rust_types_as_walls::api_evolution::BillingPlan;

fn label(plan: BillingPlan) -> &'static str {
    match plan {
        BillingPlan::Free => "free",
        BillingPlan::Pro => "pro",
    }
}

fn main() {
    let _ = label(BillingPlan::Free);
}
