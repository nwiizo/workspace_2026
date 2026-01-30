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
        };
    }
    result
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
}
