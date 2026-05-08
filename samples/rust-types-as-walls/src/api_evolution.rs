//! `#[non_exhaustive]` を使った将来互換のある公開 API。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BillingPlan {
    Free,
    Pro,
}

impl BillingPlan {
    pub fn monthly_price(self) -> u64 {
        match self {
            Self::Free => 0,
            Self::Pro => 2_400,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CheckoutSummary {
    pub customer_email: String,
    pub plan: BillingPlan,
}

impl CheckoutSummary {
    pub fn new(customer_email: impl Into<String>, plan: BillingPlan) -> Self {
        Self {
            customer_email: customer_email.into(),
            plan,
        }
    }

    pub fn monthly_price(&self) -> u64 {
        self.plan.monthly_price()
    }
}

pub fn describe(summary: &CheckoutSummary) -> String {
    format!(
        "customer={} plan={:?} monthly_price={}yen",
        summary.customer_email,
        summary.plan,
        summary.monthly_price()
    )
}
