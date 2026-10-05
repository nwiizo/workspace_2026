//! Chapters 2, 3 and 6: readable pricing rules and characterization tests.
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct OrderItem {
    pub unit_price_cents: u64,
    pub quantity: u32,
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum PricingError {
    #[error("order total exceeds the supported amount")]
    Overflow,
    #[error("discount percentage must be between 0 and 100")]
    InvalidDiscount,
}

/// Applies a 10% discount at 500.00; rounds the final price down to whole cents.
pub fn bulk_price(subtotal_cents: u64) -> u64 {
    // Wide intermediate arithmetic keeps even u64::MAX safe before rounding.
    if subtotal_cents >= 50_000 {
        (u128::from(subtotal_cents) * 90 / 100) as u64
    } else {
        subtotal_cents
    }
}

/// Sums item totals without silently wrapping on overflow.
pub fn subtotal(items: &[OrderItem]) -> Result<u64, PricingError> {
    items.iter().try_fold(0_u64, |total, item| {
        let line_total = item
            .unit_price_cents
            .checked_mul(u64::from(item.quantity))
            .ok_or(PricingError::Overflow)?;
        total.checked_add(line_total).ok_or(PricingError::Overflow)
    })
}

/// Calculates the chapter 6 item total with an explicit percentage discount.
pub fn discounted_total(items: &[OrderItem], discount_percent: u8) -> Result<u64, PricingError> {
    if discount_percent > 100 {
        return Err(PricingError::InvalidDiscount);
    }
    let total = subtotal(items)?;
    // The result is at most total, so converting back to u64 cannot truncate it.
    Ok((u128::from(total) * u128::from(100 - discount_percent) / 100) as u64)
}
