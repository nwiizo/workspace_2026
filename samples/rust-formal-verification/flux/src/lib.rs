use flux_rs::attrs::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[refined_by(value: int)]
#[invariant(0 <= value && value <= 100)]
pub struct DiscountRate {
    #[field(u8[value])]
    value: u8,
}

impl DiscountRate {
    #[sig(fn(value: u8{value <= 100}) -> DiscountRate[value])]
    fn new_valid(value: u8) -> Self {
        Self { value }
    }

    pub fn new(value: u8) -> Option<Self> {
        if value <= 100 {
            Some(Self::new_valid(value))
        } else {
            None
        }
    }

    #[sig(fn(DiscountRate[@value]) -> u8{result: result == value && result <= 100})]
    pub fn value(self) -> u8 {
        self.value
    }
}

#[sig(fn(price: u16, DiscountRate[@rate]) -> u16{result: result <= price})]
pub fn apply_discount(price: u16, rate: DiscountRate) -> u16 {
    let remaining_percent = 100_u32 - rate.value() as u32;
    let discounted = (price as u32) * remaining_percent / 100;
    discounted as u16
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
