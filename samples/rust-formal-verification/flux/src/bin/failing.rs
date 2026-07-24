use flux_rs::attrs::*;

#[derive(Clone, Copy)]
#[refined_by(value: int)]
#[invariant(0 <= value && value <= 100)]
pub struct DiscountRate {
    #[field(u8[value])]
    value: u8,
}

impl DiscountRate {
    #[sig(fn(DiscountRate[@value]) -> u8{result: result == value && result <= 100})]
    pub fn value(self) -> u8 {
        self.value
    }
}

#[sig(fn(price: u16, DiscountRate) -> u16{result: result <= price})]
pub fn apply_discount(price: u16, rate: DiscountRate) -> u16 {
    let remaining_percent = 100_u16 - rate.value() as u16;
    price * remaining_percent / 100
}

fn main() {}
