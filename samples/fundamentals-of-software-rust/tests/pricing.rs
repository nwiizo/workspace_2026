use fundamentals_of_software_rust::pricing::{OrderItem, PricingError, bulk_price, subtotal};

#[test]
fn large_order_receives_ten_percent_discount() {
    assert_eq!(bulk_price(50_000), 45_000);
}

#[test]
fn discount_starts_at_the_threshold_and_rounds_final_cents_down() {
    assert_eq!(bulk_price(49_999), 49_999);
    assert_eq!(bulk_price(50_001), 45_000);
    assert_eq!(bulk_price(0), 0);
    assert_eq!(bulk_price(u64::MAX), 16_602_069_666_338_596_453);
}

#[test]
fn item_quantities_produce_the_book_subtotal() {
    let items = [
        OrderItem {
            unit_price_cents: 1_000,
            quantity: 2,
        },
        OrderItem {
            unit_price_cents: 1_500,
            quantity: 1,
        },
    ];
    assert_eq!(subtotal(&items), Ok(3_500));
    assert_eq!(subtotal(&[]), Ok(0));
}

#[test]
fn explicit_discount_preserves_the_chapter_six_example() {
    let items = [
        OrderItem {
            unit_price_cents: 1_000,
            quantity: 2,
        },
        OrderItem {
            unit_price_cents: 1_500,
            quantity: 1,
        },
    ];
    assert_eq!(
        fundamentals_of_software_rust::pricing::discounted_total(&items, 10),
        Ok(3_150)
    );
}

#[test]
fn subtotal_reports_both_line_and_sum_overflow() {
    let huge_line = [OrderItem {
        unit_price_cents: u64::MAX,
        quantity: 2,
    }];
    assert_eq!(subtotal(&huge_line), Err(PricingError::Overflow));
    let huge_sum = [
        OrderItem {
            unit_price_cents: u64::MAX,
            quantity: 1,
        },
        OrderItem {
            unit_price_cents: 1,
            quantity: 1,
        },
    ];
    assert_eq!(subtotal(&huge_sum), Err(PricingError::Overflow));
}

#[test]
fn explicit_discount_validates_its_percentage_and_preserves_overflow_errors() {
    use fundamentals_of_software_rust::pricing::discounted_total;
    let items = [OrderItem {
        unit_price_cents: 101,
        quantity: 1,
    }];
    assert_eq!(discounted_total(&items, 0), Ok(101));
    assert_eq!(discounted_total(&items, 10), Ok(90));
    assert_eq!(discounted_total(&items, 100), Ok(0));
    assert_eq!(
        discounted_total(&items, 101),
        Err(PricingError::InvalidDiscount)
    );
    let huge_line = [OrderItem {
        unit_price_cents: u64::MAX,
        quantity: 2,
    }];
    assert_eq!(
        discounted_total(&huge_line, 10),
        Err(PricingError::Overflow)
    );
}
