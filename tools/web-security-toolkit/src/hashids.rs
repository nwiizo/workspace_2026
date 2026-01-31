//! Hashids Encoder/Decoder
//!
//! Tools for encoding/decoding Hashids and discovering salts.
//!
//! Hashids is a library for encoding numbers into short, unique strings.
//! Used by Juice Shop for continue codes.
//!
//! # Example
//!
//! ```rust
//! use web_security_toolkit::hashids::{encode_hashid, decode_hashid, try_decode_with_salts};
//!
//! // Encode with known salt
//! let encoded = encode_hashid(&[1, 2, 3], "mysalt", 8);
//!
//! // Decode with known salt
//! if let Some(numbers) = decode_hashid(&encoded, "mysalt") {
//!     println!("Decoded: {:?}", numbers);
//! }
//!
//! // Try to find salt
//! let salts = vec!["salt1", "salt2", "mysalt"];
//! if let Some((salt, numbers)) = try_decode_with_salts(&encoded, &salts) {
//!     println!("Found salt: {}, numbers: {:?}", salt, numbers);
//! }
//! ```

use harsh::Harsh;

/// Encode numbers into a Hashid string
///
/// # Arguments
///
/// * `numbers` - Slice of numbers to encode
/// * `salt` - Salt string for encoding
/// * `min_length` - Minimum length of output string
///
/// # Returns
///
/// Encoded Hashid string
pub fn encode_hashid(numbers: &[u64], salt: &str, min_length: usize) -> String {
    let harsh = Harsh::builder()
        .salt(salt)
        .length(min_length)
        .build()
        .expect("Failed to build Harsh encoder");

    harsh.encode(numbers)
}

/// Decode a Hashid string into numbers
///
/// # Arguments
///
/// * `hashid` - The Hashid string to decode
/// * `salt` - Salt string used for encoding
///
/// # Returns
///
/// Some(Vec<u64>) if decoding succeeds, None otherwise
pub fn decode_hashid(hashid: &str, salt: &str) -> Option<Vec<u64>> {
    let harsh = Harsh::builder().salt(salt).build().ok()?;

    harsh.decode(hashid).ok()
}

/// Try to decode a Hashid with multiple salts
///
/// # Arguments
///
/// * `hashid` - The Hashid string to decode
/// * `salts` - Slice of salt strings to try
///
/// # Returns
///
/// Some((salt, numbers)) if a valid salt is found, None otherwise
pub fn try_decode_with_salts<'a>(hashid: &str, salts: &'a [&str]) -> Option<(&'a str, Vec<u64>)> {
    for salt in salts {
        if let Some(numbers) = decode_hashid(hashid, salt) {
            if !numbers.is_empty() {
                return Some((salt, numbers));
            }
        }
    }
    None
}

/// Common salts used by web applications
pub fn common_salts() -> Vec<&'static str> {
    vec![
        // Empty/simple
        "",
        "salt",
        "secret",
        "key",
        // Juice Shop themed
        "juice",
        "juiceshop",
        "juice-shop",
        "JuiceShop",
        "Juice Shop",
        "OWASP Juice Shop",
        "owasp",
        "bkimminich",
        "pwning",
        // Continue code specific (based on Juice Shop source)
        "this is my salt",
        "this-is-my-salt",
        "default",
        "hashid",
        // Common application salts
        "app",
        "application",
        "webapp",
        "myapp",
        "mysalt",
        // Security themed
        "ctf",
        "challenge",
        "flag",
        "hack",
        "security",
    ]
}

/// Juice Shop specific salts (from source code analysis)
pub fn juice_shop_salts() -> Vec<&'static str> {
    vec![
        // From continueCode.ts analysis - the actual salt used
        "this is my salt",
        // Other potential salts from Juice Shop codebase
        "hashids",
        "continueCode",
        "continue-code",
        "progress",
        "challenge",
        "challenges",
        // Configuration variants
        "zIxzswGAObxsmt6c", // Random-looking salt that might be used
    ]
}

/// Generate a continue code for Juice Shop
///
/// The continue code encodes solved challenge IDs using Hashids.
///
/// # Arguments
///
/// * `challenge_ids` - Slice of solved challenge IDs
/// * `salt` - Salt to use (try juice_shop_salts() values)
///
/// # Returns
///
/// The continue code string
pub fn generate_continue_code(challenge_ids: &[u64], salt: &str) -> String {
    encode_hashid(challenge_ids, salt, 60)
}

/// Decode a Juice Shop continue code
///
/// # Arguments
///
/// * `code` - The continue code to decode
/// * `salt` - Salt to use for decoding
///
/// # Returns
///
/// Some(challenge_ids) if successful, None otherwise
pub fn decode_continue_code(code: &str, salt: &str) -> Option<Vec<u64>> {
    let harsh = Harsh::builder().salt(salt).length(60).build().ok()?;

    harsh.decode(code).ok()
}

/// Try to discover the salt used by a continue code
///
/// # Arguments
///
/// * `code` - The continue code to analyze
/// * `expected_ids` - Optional expected challenge IDs for validation
///
/// # Returns
///
/// Some(salt) if found, None otherwise
pub fn discover_salt(code: &str, expected_ids: Option<&[u64]>) -> Option<String> {
    let all_salts: Vec<&str> = juice_shop_salts()
        .into_iter()
        .chain(common_salts())
        .collect();

    for salt in all_salts {
        if let Some(decoded) = decode_continue_code(code, salt) {
            // If we have expected IDs, validate
            if let Some(expected) = expected_ids {
                if decoded == expected {
                    return Some(salt.to_string());
                }
            } else {
                // No expected IDs, just check if decoding produces valid-looking IDs
                // Valid challenge IDs should be positive integers, typically 1-200
                if !decoded.is_empty() && decoded.iter().all(|&id| id > 0 && id < 500) {
                    return Some(salt.to_string());
                }
            }
        }
    }
    None
}

/// Brute force possible numeric combinations for a Hashid
///
/// Useful when you know the encoding but not the input numbers.
///
/// # Arguments
///
/// * `hashid` - The Hashid to match
/// * `salt` - Salt to use
/// * `max_value` - Maximum value to try for each position
/// * `positions` - Number of positions to try
///
/// # Returns
///
/// Some(numbers) if a match is found, None otherwise
pub fn brute_force_numbers(
    hashid: &str,
    salt: &str,
    max_value: u64,
    positions: usize,
) -> Option<Vec<u64>> {
    // Single position
    if positions == 1 {
        for n in 0..=max_value {
            if encode_hashid(&[n], salt, 0) == hashid {
                return Some(vec![n]);
            }
        }
        return None;
    }

    // Multiple positions (limited for performance)
    if positions == 2 && max_value <= 200 {
        for n1 in 0..=max_value {
            for n2 in 0..=max_value {
                if encode_hashid(&[n1, n2], salt, 0) == hashid {
                    return Some(vec![n1, n2]);
                }
            }
        }
    }

    None
}

/// Generate all possible continue codes for "Imaginary Challenge"
///
/// The Imaginary Challenge (ID doesn't exist) requires submitting a continue
/// code that would encode a non-existent challenge ID.
///
/// # Returns
///
/// Vec of (salt, code, description) tuples for potential imaginary challenge codes
pub fn generate_imaginary_challenge_codes() -> Vec<(String, String, String)> {
    let mut codes = Vec::new();

    // Common imaginary/non-existent challenge IDs
    let imaginary_ids: Vec<(u64, &str)> = vec![
        (999, "ID 999 (common imaginary)"),
        (1000, "ID 1000 (round number)"),
        (9999, "ID 9999 (large)"),
        (0, "ID 0 (zero)"),
        (200, "ID 200 (just beyond typical range)"),
        (201, "ID 201"),
        (255, "ID 255 (byte max)"),
        (256, "ID 256"),
        (u64::MAX, "MAX_U64 (overflow)"),
        (u64::MAX - 1, "MAX_U64 - 1"),
    ];

    // Try various salts with non-existent challenge IDs
    for salt in juice_shop_salts() {
        for (id, desc) in &imaginary_ids {
            let code = generate_continue_code(&[*id], salt);
            codes.push((salt.to_string(), code, desc.to_string()));
        }
    }

    codes
}

/// Generate continue codes that include an imaginary challenge ID
/// along with real challenge IDs (to make the code look more legitimate)
///
/// # Arguments
///
/// * `real_ids` - Real challenge IDs already solved
/// * `imaginary_id` - The fake challenge ID to inject
///
/// # Returns
///
/// Vec of (salt, code) pairs
pub fn generate_forged_continue_codes(
    real_ids: &[u64],
    imaginary_id: u64,
) -> Vec<(String, String)> {
    let mut codes = Vec::new();

    // Combine real IDs with imaginary ID
    let mut combined = real_ids.to_vec();
    combined.push(imaginary_id);
    combined.sort_unstable();

    for salt in juice_shop_salts() {
        let code = generate_continue_code(&combined, salt);
        codes.push((salt.to_string(), code));
    }

    codes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let salt = "test-salt";
        let numbers = vec![1, 2, 3];
        let encoded = encode_hashid(&numbers, salt, 0);
        let decoded = decode_hashid(&encoded, salt);
        assert_eq!(decoded, Some(numbers));
    }

    #[test]
    fn test_wrong_salt() {
        let encoded = encode_hashid(&[1, 2, 3], "correct-salt", 0);
        let decoded = decode_hashid(&encoded, "wrong-salt");
        // With wrong salt, decoding should either fail or produce different numbers
        assert!(decoded.is_none() || decoded != Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_try_decode_with_salts() {
        let correct_salt = "my-secret-salt";
        let numbers = vec![42, 123];
        let encoded = encode_hashid(&numbers, correct_salt, 0);

        let salts = vec!["wrong1", "wrong2", "my-secret-salt", "wrong3"];
        let result = try_decode_with_salts(&encoded, &salts);

        assert!(result.is_some());
        let (found_salt, decoded) = result.unwrap();
        assert_eq!(found_salt, correct_salt);
        assert_eq!(decoded, numbers);
    }

    #[test]
    fn test_common_salts() {
        let salts = common_salts();
        assert!(!salts.is_empty());
        assert!(salts.contains(&""));
        assert!(salts.contains(&"salt"));
    }

    #[test]
    fn test_juice_shop_salts() {
        let salts = juice_shop_salts();
        assert!(!salts.is_empty());
    }

    #[test]
    fn test_continue_code_roundtrip() {
        let salt = "this is my salt";
        let challenge_ids = vec![1, 5, 10, 15, 20];
        let code = generate_continue_code(&challenge_ids, salt);
        let decoded = decode_continue_code(&code, salt);
        assert_eq!(decoded, Some(challenge_ids));
    }

    #[test]
    fn test_generate_imaginary_challenge_codes() {
        let codes = generate_imaginary_challenge_codes();
        // Should generate codes for multiple salts and multiple IDs
        assert!(!codes.is_empty());
        // Each code should be a tuple of (salt, code, description)
        for (salt, code, desc) in &codes {
            assert!(!salt.is_empty() || salt == "");
            assert!(!code.is_empty());
            assert!(!desc.is_empty());
        }
    }

    #[test]
    fn test_generate_forged_continue_codes() {
        let real_ids = vec![1, 2, 3];
        let imaginary_id = 999;
        let codes = generate_forged_continue_codes(&real_ids, imaginary_id);

        assert!(!codes.is_empty());

        // Verify the codes can be decoded back
        for (salt, code) in &codes {
            if let Some(decoded) = decode_continue_code(code, salt) {
                // Should contain both real and imaginary IDs
                assert!(decoded.contains(&imaginary_id));
                for id in &real_ids {
                    assert!(decoded.contains(id));
                }
            }
        }
    }
}
