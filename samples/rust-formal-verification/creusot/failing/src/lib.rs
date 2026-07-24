extern crate creusot_std;

use creusot_std::{invariant::Invariant, prelude::*};

#[cfg_attr(not(creusot), derive(Clone, Copy, Debug, Eq, PartialEq))]
pub struct DiscountRate(u8);

impl DiscountRate {
    #[logic(open(self))]
    pub fn spec_value(self) -> Int {
        pearlite! { self.0@ }
    }
}

impl Invariant for DiscountRate {
    #[logic(open)]
    fn invariant(self) -> bool {
        pearlite! { self.spec_value() <= 100 }
    }
}

impl DiscountRate {
    #[requires(value@ <= 100)]
    pub fn new(value: u8) -> Self {
        Self(value)
    }
}

#[ensures(result@ <= price@)]
pub fn apply_discount(price: u16, rate: DiscountRate) -> u16 {
    let remaining_percent = 100_u16 - u16::from(rate.0);
    price * remaining_percent / 100
}
