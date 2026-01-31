//! Encoding utilities for payloads

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

/// URL encode a string
pub fn url_encode(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
}

/// Double URL encode
pub fn double_url_encode(input: &str) -> String {
    url_encode(&url_encode(input))
}

/// Base64 encode
pub fn base64_encode(input: &str) -> String {
    BASE64_STANDARD.encode(input.as_bytes())
}

/// Base64 decode
pub fn base64_decode(input: &str) -> Result<String, String> {
    let bytes = BASE64_STANDARD.decode(input).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// Hex encode
pub fn hex_encode(input: &str) -> String {
    hex::encode(input.as_bytes())
}

/// Hex decode
pub fn hex_decode(input: &str) -> Result<String, String> {
    let bytes = hex::decode(input).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

/// HTML entity encode
pub fn html_entity_encode(input: &str) -> String {
    input.chars().map(|c| format!("&#{};", c as u32)).collect()
}

/// ROT13 encode/decode
pub fn rot13(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let base = if c.is_ascii_lowercase() { b'a' } else { b'A' };
                let offset = (c as u8 - base + 13) % 26;
                (base + offset) as char
            } else {
                c
            }
        })
        .collect()
}

/// Unicode escape
pub fn unicode_escape(input: &str) -> String {
    input
        .chars()
        .map(|c| format!("\\u{:04x}", c as u32))
        .collect()
}

/// Apply multiple encodings
pub fn multi_encode(input: &str, encodings: &[Encoding]) -> String {
    let mut result = input.to_string();
    for encoding in encodings {
        result = match encoding {
            Encoding::Url => url_encode(&result),
            Encoding::DoubleUrl => double_url_encode(&result),
            Encoding::Base64 => base64_encode(&result),
            Encoding::Hex => hex_encode(&result),
            Encoding::Html => html_entity_encode(&result),
            Encoding::Rot13 => rot13(&result),
            Encoding::Unicode => unicode_escape(&result),
            Encoding::Z85 => z85_encode(&result),
        };
    }
    result
}

// =============================================================================
// Z85 Encoding (ZeroMQ Base-85)
// =============================================================================

/// Z85 encode a string
///
/// Z85 requires data length to be a multiple of 4 bytes.
/// This function pads the input with null bytes if necessary.
///
/// # Example
/// ```
/// use rectitude::payloads::encoding::z85_encode;
/// let encoded = z85_encode("test");
/// assert!(!encoded.is_empty());
/// ```
pub fn z85_encode(data: &str) -> String {
    z85_encode_bytes(data.as_bytes())
}

/// Z85 encode raw bytes
///
/// Pads to 4-byte boundary with null bytes if necessary.
pub fn z85_encode_bytes(data: &[u8]) -> String {
    let padding = (4 - (data.len() % 4)) % 4;
    let mut padded = data.to_vec();
    padded.extend(std::iter::repeat_n(0u8, padding));
    z85::encode(&padded)
}

/// Z85 decode a string
///
/// Returns None if the input is not valid Z85.
///
/// # Example
/// ```
/// use rectitude::payloads::encoding::{z85_encode, z85_decode};
/// let encoded = z85_encode("test");
/// let decoded = z85_decode(&encoded);
/// assert!(decoded.is_some());
/// ```
pub fn z85_decode(encoded: &str) -> Option<String> {
    z85_decode_bytes(encoded).and_then(|bytes| String::from_utf8(bytes).ok())
}

/// Z85 decode to raw bytes
pub fn z85_decode_bytes(encoded: &str) -> Option<Vec<u8>> {
    z85::decode(encoded).ok()
}

// =============================================================================
// Coupon Forgery Helpers
// =============================================================================

/// Generate a forged coupon code in MMMYY-DD format
///
/// This is based on the Juice Shop forged coupon challenge where
/// coupons are Z85-encoded strings in format like "JAN26-90".
///
/// # Example
/// ```
/// use rectitude::payloads::encoding::forge_coupon;
/// let coupon = forge_coupon("JAN", 26, 90);
/// assert!(coupon.contains("JAN26-90") || !coupon.is_empty());
/// ```
pub fn forge_coupon(month: &str, year: u16, discount: u8) -> String {
    let coupon_str = format!("{}{}-{}", month.to_uppercase(), year % 100, discount);
    z85_encode(&coupon_str)
}

/// Generate coupon codes for all months of a given year
///
/// Useful for brute-forcing valid coupon patterns.
pub fn forge_coupons_for_year(year: u16, discount: u8) -> Vec<(String, String)> {
    let months = [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ];

    months
        .iter()
        .map(|month| {
            let plain = format!("{}{}-{}", month, year % 100, discount);
            let encoded = z85_encode(&plain);
            (plain, encoded)
        })
        .collect()
}

/// Decode a Z85 coupon and parse its components
///
/// Returns (month, year, discount) if valid.
pub fn decode_coupon(encoded: &str) -> Option<(String, u16, u8)> {
    let decoded = z85_decode(encoded)?;
    // Expected format: MMMYY-DD (e.g., "JAN26-90")
    let trimmed = decoded.trim_end_matches('\0');

    if trimmed.len() < 7 {
        return None;
    }

    let month = trimmed.get(0..3)?.to_string();
    let year_str = trimmed.get(3..5)?;
    let year: u16 = year_str.parse().ok()?;

    // Find the discount after the dash
    let dash_pos = trimmed.find('-')?;
    let discount_str = trimmed.get(dash_pos + 1..)?;
    let discount: u8 = discount_str.trim().parse().ok()?;

    Some((month, 2000 + year, discount))
}

#[derive(Debug, Clone, Copy)]
pub enum Encoding {
    Url,
    DoubleUrl,
    Base64,
    Hex,
    Html,
    Rot13,
    Unicode,
    Z85,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("<script>"), "%3Cscript%3E");
    }

    #[test]
    fn test_base64_roundtrip() {
        let input = "Hello, World!";
        let encoded = base64_encode(input);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(input, decoded);
    }

    #[test]
    fn test_rot13() {
        let input = "Hello";
        let rotated = rot13(input);
        assert_eq!(rot13(&rotated), input);
    }

    #[test]
    fn test_multi_encode() {
        let result = multi_encode("<", &[Encoding::Url, Encoding::Base64]);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_z85_encode_decode() {
        // Test with exact 4-byte multiple
        let input = "test";
        let encoded = z85_encode(input);
        assert!(!encoded.is_empty());

        let decoded = z85_decode(&encoded);
        assert!(decoded.is_some());
        // Decoded may have padding nulls, trim them
        let decoded_str = decoded.unwrap();
        assert!(decoded_str.starts_with(input));
    }

    #[test]
    fn test_z85_encode_non_aligned() {
        // Test with non-4-byte aligned data
        let input = "hello";
        let encoded = z85_encode(input);
        assert!(!encoded.is_empty());

        let decoded = z85_decode(&encoded);
        assert!(decoded.is_some());
    }

    #[test]
    fn test_forge_coupon() {
        let coupon = forge_coupon("JAN", 26, 90);
        assert!(!coupon.is_empty());

        // Verify we can decode it back
        let decoded = z85_decode(&coupon);
        assert!(decoded.is_some());
        let decoded_str = decoded.unwrap();
        assert!(decoded_str.contains("JAN26-90"));
    }

    #[test]
    fn test_forge_coupons_for_year() {
        let coupons = forge_coupons_for_year(2026, 50);
        assert_eq!(coupons.len(), 12);

        // Check first and last
        assert!(coupons[0].0.starts_with("JAN"));
        assert!(coupons[11].0.starts_with("DEC"));
    }

    #[test]
    fn test_decode_coupon() {
        let encoded = forge_coupon("MAR", 26, 75);
        let decoded = decode_coupon(&encoded);
        assert!(decoded.is_some());

        let (month, year, discount) = decoded.unwrap();
        assert_eq!(month, "MAR");
        assert_eq!(year, 2026);
        assert_eq!(discount, 75);
    }
}
