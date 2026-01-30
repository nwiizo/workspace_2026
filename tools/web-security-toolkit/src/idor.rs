//! IDOR (Insecure Direct Object Reference) testing utilities
//!
//! Provides tools for testing IDOR vulnerabilities by manipulating object IDs.


/// IDOR test result
#[derive(Debug, Clone)]
pub struct IdorTestResult {
    pub endpoint: String,
    pub original_id: String,
    pub tested_id: String,
    pub status_code: u16,
    pub accessible: bool,
    pub response_size: usize,
}

/// Generate ID variations for IDOR testing
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::idor::generate_id_variations;
///
/// let ids = generate_id_variations(5, 10);
/// assert!(ids.contains(&1));
/// assert!(ids.contains(&10));
/// ```
pub fn generate_id_variations(current_id: i64, range: usize) -> Vec<i64> {
    let mut ids = Vec::new();

    // IDs before current
    for i in 1..=range {
        let id = current_id - i as i64;
        if id > 0 {
            ids.push(id);
        }
    }

    // IDs after current
    for i in 1..=range {
        ids.push(current_id + i as i64);
    }

    // Common interesting IDs
    let interesting = [0, 1, 2, 100, 1000, -1];
    for &id in &interesting {
        if id != current_id && !ids.contains(&id) {
            ids.push(id);
        }
    }

    ids.sort();
    ids.dedup();
    ids
}

/// Generate string ID variations (for UUID-like IDs)
pub fn generate_string_id_variations(current_id: &str) -> Vec<String> {
    let mut ids = Vec::new();

    // Try numeric extraction
    if let Ok(num) = current_id.parse::<i64>() {
        for id in generate_id_variations(num, 10) {
            ids.push(id.to_string());
        }
    }

    // Common string variations
    ids.push("admin".to_string());
    ids.push("root".to_string());
    ids.push("test".to_string());
    ids.push("null".to_string());
    ids.push("undefined".to_string());
    ids.push("0".to_string());
    ids.push("1".to_string());
    ids.push("-1".to_string());
    ids.push("../".to_string());
    ids.push("..%2f".to_string());

    // If UUID-like, try variations
    if current_id.contains('-') && current_id.len() > 30 {
        // Try incrementing last part
        if let Some(pos) = current_id.rfind('-') {
            let (prefix, suffix) = current_id.split_at(pos + 1);
            if let Ok(num) = i64::from_str_radix(suffix, 16) {
                for delta in [-1i64, 1, 2, -2] {
                    let new_suffix = format!("{:012x}", (num as i64 + delta) as u64);
                    ids.push(format!("{}{}", prefix, new_suffix));
                }
            }
        }
    }

    ids
}

/// Common IDOR-vulnerable endpoints patterns
pub fn common_idor_endpoints() -> Vec<IdorEndpoint> {
    vec![
        IdorEndpoint::new("/api/users/{id}", "User profile", IdorType::Numeric),
        IdorEndpoint::new("/api/users/{id}/profile", "User profile details", IdorType::Numeric),
        IdorEndpoint::new("/api/orders/{id}", "Order details", IdorType::Numeric),
        IdorEndpoint::new("/api/invoices/{id}", "Invoice", IdorType::Numeric),
        IdorEndpoint::new("/api/documents/{id}", "Document", IdorType::Numeric),
        IdorEndpoint::new("/api/files/{id}", "File download", IdorType::Numeric),
        IdorEndpoint::new("/api/messages/{id}", "Private message", IdorType::Numeric),
        IdorEndpoint::new("/api/tickets/{id}", "Support ticket", IdorType::Numeric),
        IdorEndpoint::new("/api/payments/{id}", "Payment details", IdorType::Numeric),
        IdorEndpoint::new("/api/accounts/{id}", "Account info", IdorType::Numeric),
        IdorEndpoint::new("/rest/basket/{id}", "Shopping basket", IdorType::Numeric),
        IdorEndpoint::new("/rest/user/{id}", "User info", IdorType::Numeric),
        IdorEndpoint::new("/download?file={id}", "File download", IdorType::String),
        IdorEndpoint::new("/profile?user={id}", "User profile", IdorType::String),
    ]
}

/// IDOR endpoint definition
#[derive(Debug, Clone)]
pub struct IdorEndpoint {
    pub pattern: String,
    pub description: String,
    pub id_type: IdorType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IdorType {
    Numeric,
    String,
    Uuid,
}

impl IdorEndpoint {
    pub fn new(pattern: &str, description: &str, id_type: IdorType) -> Self {
        Self {
            pattern: pattern.to_string(),
            description: description.to_string(),
            id_type,
        }
    }

    /// Generate test URLs for this endpoint
    pub fn generate_test_urls(&self, base_url: &str, current_id: &str) -> Vec<String> {
        let ids = match self.id_type {
            IdorType::Numeric => {
                if let Ok(num) = current_id.parse::<i64>() {
                    generate_id_variations(num, 10)
                        .iter()
                        .map(|id| id.to_string())
                        .collect()
                } else {
                    generate_string_id_variations(current_id)
                }
            }
            IdorType::String | IdorType::Uuid => generate_string_id_variations(current_id),
        };

        ids.iter()
            .map(|id| format!("{}{}", base_url, self.pattern.replace("{id}", id)))
            .collect()
    }
}

/// Juice Shop specific IDOR endpoints
pub fn juice_shop_idor_endpoints() -> Vec<IdorEndpoint> {
    vec![
        IdorEndpoint::new("/rest/basket/{id}", "Shopping basket (View Basket challenge)", IdorType::Numeric),
        IdorEndpoint::new("/api/Users/{id}", "User information", IdorType::Numeric),
        IdorEndpoint::new("/api/Feedbacks/{id}", "Feedback entry", IdorType::Numeric),
        IdorEndpoint::new("/api/Products/{id}", "Product details", IdorType::Numeric),
        IdorEndpoint::new("/api/Complaints/{id}", "Complaint details", IdorType::Numeric),
        IdorEndpoint::new("/api/Recycles/{id}", "Recycle entry", IdorType::Numeric),
        IdorEndpoint::new("/api/BasketItems/{id}", "Basket item", IdorType::Numeric),
    ]
}

/// Detect potential sensitive data in IDOR response
pub fn analyze_idor_response(response: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let response_lower = response.to_lowercase();

    // Check for PII
    let pii_patterns = [
        ("email", "Email address"),
        ("password", "Password field"),
        ("ssn", "Social Security Number"),
        ("credit", "Credit card"),
        ("phone", "Phone number"),
        ("address", "Physical address"),
        ("dob", "Date of birth"),
        ("birth", "Birth information"),
    ];

    for (pattern, description) in pii_patterns {
        if response_lower.contains(pattern) {
            findings.push(format!("Potential {} exposure", description));
        }
    }

    // Check for authentication data
    if response_lower.contains("token") || response_lower.contains("jwt") {
        findings.push("Authentication token exposed".to_string());
    }

    // Check for internal IDs
    if response_lower.contains("_id") || response_lower.contains("userid") {
        findings.push("Internal ID exposed".to_string());
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id_variations() {
        let ids = generate_id_variations(5, 3);
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
        assert!(ids.contains(&4));
        assert!(ids.contains(&6));
        assert!(ids.contains(&7));
        assert!(ids.contains(&8));
    }

    #[test]
    fn test_generate_string_variations() {
        let ids = generate_string_id_variations("123");
        assert!(!ids.is_empty());
        assert!(ids.iter().any(|id| id == "124"));
    }

    #[test]
    fn test_common_endpoints() {
        let endpoints = common_idor_endpoints();
        assert!(!endpoints.is_empty());
        assert!(endpoints.iter().any(|e| e.pattern.contains("users")));
    }

    #[test]
    fn test_juice_shop_endpoints() {
        let endpoints = juice_shop_idor_endpoints();
        assert!(endpoints.iter().any(|e| e.pattern.contains("basket")));
    }

    #[test]
    fn test_analyze_response() {
        let response = r#"{"email": "user@test.com", "password_hash": "xxx"}"#;
        let findings = analyze_idor_response(response);
        assert!(findings.iter().any(|f| f.contains("Email")));
        assert!(findings.iter().any(|f| f.contains("Password")));
    }
}
