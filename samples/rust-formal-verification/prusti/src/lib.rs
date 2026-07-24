use prusti_contracts::*;

#[derive(Clone, Copy)]
#[cfg_attr(not(prusti), derive(Debug, Eq, PartialEq))]
pub struct DiscountRate {
    value: u8,
}

impl DiscountRate {
    #[ensures(match result {
        Some(rate) => rate.value == value && value <= 100,
        None => value > 100,
    })]
    pub fn new(value: u8) -> Option<Self> {
        if value <= 100 {
            Some(Self { value })
        } else {
            None
        }
    }

    #[pure]
    pub fn value(self) -> u8 {
        self.value
    }
}

#[requires(rate.value <= 100)]
#[ensures(result <= price as u32)]
#[ensures(rate.value == 100 ==> result == 0)]
pub fn apply_discount(price: u16, rate: DiscountRate) -> u32 {
    let remaining_percent = 100_u32 - rate.value as u32;
    (price as u32) * remaining_percent / 100
}

#[cfg(test)]
mod tests {
    use super::{apply_discount, DiscountRate};

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
