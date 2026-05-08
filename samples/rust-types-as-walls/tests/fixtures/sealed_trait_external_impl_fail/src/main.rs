use rust_types_as_walls::sealed_payment::PaymentState;

struct Refunded;

impl PaymentState for Refunded {
    fn label() -> &'static str {
        "refunded"
    }
}

fn main() {}
