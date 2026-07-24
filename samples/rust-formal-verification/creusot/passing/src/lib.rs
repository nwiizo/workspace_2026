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
    #[ensures(match result {
        Some(rate) => rate.spec_value() == value@,
        None => value@ > 100,
    })]
    pub fn new(value: u8) -> Option<Self> {
        if value <= 100 {
            Some(Self(value))
        } else {
            None
        }
    }

    #[ensures(result@ == self.spec_value())]
    pub fn value(self) -> u8 {
        self.0
    }
}

#[ensures(result@ <= price@)]
#[ensures(rate.spec_value() == 100 ==> result@ == 0)]
pub fn apply_discount(price: u16, rate: DiscountRate) -> u32 {
    let remaining_percent = 100_u32 - u32::from(rate.0);
    u32::from(price) * remaining_percent / 100
}

#[cfg(test)]
mod tests {
    use super::{DiscountRate, apply_discount};

    #[test]
    fn creates_boundary_rates() {
        assert_eq!(DiscountRate::new(0).map(DiscountRate::value), Some(0));
        assert_eq!(DiscountRate::new(100).map(DiscountRate::value), Some(100));
        assert_eq!(DiscountRate::new(101), None);
    }

    #[test]
    fn applies_discount() {
        let Some(rate) = DiscountRate::new(25) else {
            panic!("25 must be a valid discount rate");
        };
        assert_eq!(apply_discount(1_000, rate), 750);
    }
}
