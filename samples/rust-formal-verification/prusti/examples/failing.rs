use prusti_contracts::*;

#[derive(Clone, Copy)]
struct DiscountRate {
    value: u8,
}

#[requires(rate.value <= 100)]
#[ensures(result <= price)]
fn apply_discount(price: u16, rate: DiscountRate) -> u16 {
    let remaining_percent = 100_u16 - rate.value as u16;
    price * remaining_percent / 100
}

fn main() {
    let _ = apply_discount(1_000, DiscountRate { value: 25 });
}
