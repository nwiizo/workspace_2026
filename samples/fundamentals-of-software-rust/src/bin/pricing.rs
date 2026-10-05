use fundamentals_of_software_rust::pricing::{
    OrderItem, PricingError, bulk_price, discounted_total,
};

fn main() -> Result<(), PricingError> {
    let bulk_total = bulk_price(50_000);
    println!(
        "Chapter 2: bulk order 500.00 -> {}.{:02}",
        bulk_total / 100,
        bulk_total % 100
    );
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
    let total = discounted_total(&items, 10)?;
    println!(
        "Chapter 6: (10.00 x 2 + 15.00) with 10% discount -> {}.{:02}",
        total / 100,
        total % 100
    );
    Ok(())
}
