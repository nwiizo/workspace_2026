//! Encoding utilities for web security testing

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodingError {
    #[error("Invalid input for encoding: {0}")]
    InvalidInput(String),
    #[error("Decoding failed: {0}")]
    DecodeFailed(String),
}

/// Z85 encode a string (pads to 4-byte boundary)
pub fn z85_encode(input: &str) -> String {
    let padded_len = input.len().div_ceil(4) * 4;
    let mut padded = input.as_bytes().to_vec();
    padded.resize(padded_len, 0);
    z85::encode(&padded)
}

/// Z85 decode a string
pub fn z85_decode(input: &str) -> Result<String, EncodingError> {
    z85::decode(input)
        .map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .trim_end_matches('\0')
                .to_string()
        })
        .map_err(|e| EncodingError::DecodeFailed(e.to_string()))
}

/// Base64 encode
pub fn base64_encode(input: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(input)
}

/// Base64 decode
pub fn base64_decode(input: &str) -> Result<Vec<u8>, EncodingError> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD
        .decode(input)
        .map_err(|e| EncodingError::DecodeFailed(e.to_string()))
}

/// URL-safe Base64 encode (for JWT)
pub fn base64url_encode(input: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD.encode(input)
}

/// Hex encode
pub fn hex_encode(input: &[u8]) -> String {
    hex::encode(input)
}

/// Hex decode
pub fn hex_decode(input: &str) -> Result<Vec<u8>, EncodingError> {
    hex::decode(input).map_err(|e| EncodingError::DecodeFailed(e.to_string()))
}

/// ROT13 transformation
pub fn rot13(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'a'..='m' | 'A'..='M' => (c as u8 + 13) as char,
            'n'..='z' | 'N'..='Z' => (c as u8 - 13) as char,
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_z85_roundtrip() {
        let original = "test123";
        let encoded = z85_encode(original);
        let decoded = z85_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"hello world";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_hex_roundtrip() {
        let original = b"\xde\xad\xbe\xef";
        let encoded = hex_encode(original);
        assert_eq!(encoded, "deadbeef");
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_rot13() {
        assert_eq!(rot13("hello"), "uryyb");
        assert_eq!(rot13("uryyb"), "hello");
        assert_eq!(rot13("Hello World!"), "Uryyb Jbeyq!");
    }

    #[test]
    fn test_juice_shop_coupon() {
        // Juice Shop coupon format: MMMYY-VV
        let coupon = "JAN26-90";
        let encoded = z85_encode(coupon);
        let decoded = z85_decode(&encoded).unwrap();
        assert_eq!(decoded, coupon);
    }
}
