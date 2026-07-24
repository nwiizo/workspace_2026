#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiscountRate(u8);

impl DiscountRate {
    pub fn new(value: u8) -> Option<Self> {
        (value <= 100).then_some(Self(value))
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

pub fn apply_discount(price: u16, rate: DiscountRate) -> u16 {
    let remaining_percent = 100_u32 - u32::from(rate.value());
    let discounted = u32::from(price) * remaining_percent / 100;
    discounted as u16
}

#[cfg(any(kani, feature = "failing"))]
pub mod intentionally_wrong {
    use super::DiscountRate;

    pub fn new_discount_rate(value: u8) -> Option<DiscountRate> {
        (value <= 101).then_some(DiscountRate(value))
    }

    pub fn apply_discount(price: u16, rate: DiscountRate) -> u16 {
        let remaining_percent = 100_u16 - u16::from(rate.value());
        price * remaining_percent / 100
    }
}

#[cfg(kani)]
mod verification {
    use super::{DiscountRate, apply_discount, intentionally_wrong};

    #[kani::proof]
    fn valid_rate_round_trips() {
        let raw = kani::any::<u8>();
        kani::assume(raw <= 100);

        let rate = DiscountRate::new(raw);
        assert!(matches!(rate, Some(value) if value.value() == raw));
    }

    #[kani::proof]
    fn invalid_rate_is_rejected() {
        let raw = kani::any::<u8>();
        kani::assume(raw > 100);

        assert!(DiscountRate::new(raw).is_none());
    }

    #[kani::proof]
    fn discounted_price_never_increases() {
        let price = kani::any::<u16>();
        let raw = kani::any::<u8>();
        kani::assume(raw <= 100);

        let Some(rate) = DiscountRate::new(raw) else {
            unreachable!();
        };
        assert!(apply_discount(price, rate) <= price);
    }

    #[kani::proof]
    fn detects_off_by_one_constructor() {
        let raw = kani::any::<u8>();
        kani::assume(raw > 100);

        assert!(intentionally_wrong::new_discount_rate(raw).is_none());
    }

    #[kani::proof]
    fn detects_intermediate_overflow() {
        let price = kani::any::<u16>();
        let raw = kani::any::<u8>();
        kani::assume(raw <= 100);

        let Some(rate) = DiscountRate::new(raw) else {
            unreachable!();
        };
        let discounted = intentionally_wrong::apply_discount(price, rate);
        assert!(discounted <= price);
    }
}

#[cfg(test)]
mod tests {
    use super::{DiscountRate, apply_discount};
    use proptest::prelude::*;

    #[cfg(feature = "failing")]
    use super::intentionally_wrong;

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

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn generated_valid_discounts_never_increase(
            price in any::<u16>(),
            raw in 0_u8..=100,
        ) {
            let Some(rate) = DiscountRate::new(raw) else {
                return Err(TestCaseError::fail("generated rate must be valid"));
            };
            prop_assert!(apply_discount(price, rate) <= price);
        }
    }

    #[cfg(feature = "failing")]
    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 256,
            failure_persistence: None,
            ..ProptestConfig::default()
        })]

        #[test]
        #[ignore = "expected to find the intentionally wrong implementation"]
        fn generated_inputs_find_intermediate_overflow(
            price in any::<u16>(),
            raw in 0_u8..=100,
        ) {
            let Some(rate) = DiscountRate::new(raw) else {
                return Err(TestCaseError::fail("generated rate must be valid"));
            };
            let discounted = intentionally_wrong::apply_discount(price, rate);
            prop_assert!(discounted <= price);
        }
    }
}
