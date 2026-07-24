use vstd::prelude::*;

verus! {

pub struct DiscountRate {
    value: u8,
}

impl DiscountRate {
    pub closed spec fn spec_value(self) -> u8 {
        self.value
    }
}

pub fn apply_discount(price: u16, rate: DiscountRate) -> (result: u16)
    requires
        rate.spec_value() <= 100,
    ensures
        result <= price,
{
    let remaining_percent = 100_u16 - rate.value as u16;
    price * remaining_percent / 100
}

fn main() {}

} // verus!
