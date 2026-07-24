#[cfg(verus_only)]
use vstd::arithmetic::div_mod::{lemma_div_by_multiple, lemma_div_is_ordered};
use vstd::prelude::*;

verus! {

#[derive(Structural, Copy, Clone, PartialEq, Eq)]
pub struct DiscountRate {
    value: u8,
}

impl DiscountRate {
    pub closed spec fn spec_value(self) -> u8 {
        self.value
    }

    pub fn new(value: u8) -> (result: Option<Self>)
        ensures
            match result {
                Some(rate) => rate.spec_value() == value && value <= 100,
                None => value > 100,
            },
    {
        if value <= 100 {
            Some(Self { value })
        } else {
            None
        }
    }

    pub fn value(self) -> (result: u8)
        ensures
            result == self.spec_value(),
    {
        self.value
    }
}

pub fn apply_discount(price: u16, rate: DiscountRate) -> (result: u32)
    requires
        rate.spec_value() <= 100,
    ensures
        result <= price as u32,
        rate.spec_value() == 100 ==> result == 0,
        result as int
            == ((price as int) * (100 - rate.spec_value() as int)) / 100,
{
    let rate_value = rate.value();
    let remaining_percent = 100_u32 - rate_value as u32;
    proof {
        assert(rate_value as int <= 100);
        assert(remaining_percent as int == 100 - rate_value as int);
        assert(price as int <= 65_535);
        assert(remaining_percent as int <= 100);
        assert(
            (price as int) * (remaining_percent as int) <= 6_553_500
        ) by (nonlinear_arith)
            requires
                0 <= price as int <= 65_535,
                0 <= remaining_percent as int <= 100;
        assert(6_553_500 <= 4_294_967_295);
    }

    let numerator = (price as u32) * remaining_percent;
    proof {
        assert(numerator as int == (price as int) * (remaining_percent as int));
        assert(remaining_percent as int <= 100);
        assert(numerator as int <= (price as int) * 100) by (nonlinear_arith)
            requires
                numerator as int == (price as int) * (remaining_percent as int),
                0 <= price as int,
                remaining_percent as int <= 100;
        lemma_div_is_ordered(numerator as int, (price as int) * 100, 100);
        lemma_div_by_multiple(price as int, 100);
    }
    numerator / 100
}

} // verus!

#[cfg(test)]
mod tests {
    use super::{DiscountRate, apply_discount};

    #[test]
    fn creates_boundary_rates() {
        assert_eq!(DiscountRate::new(0).map(DiscountRate::value), Some(0));
        assert_eq!(DiscountRate::new(100).map(DiscountRate::value), Some(100));
        assert!(DiscountRate::new(101).is_none());
    }

    #[test]
    fn applies_discount() {
        let Some(rate) = DiscountRate::new(25) else {
            panic!("25 must be a valid discount rate");
        };
        assert_eq!(apply_discount(1_000, rate), 750);
    }
}
