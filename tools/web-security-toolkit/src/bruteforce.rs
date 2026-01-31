//! Brute force utilities for security testing
//!
//! Provides wordlist generation, pattern matching, and rate limit bypass techniques.

use std::collections::HashSet;

/// Generate numeric sequences
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::bruteforce::numeric_sequence;
///
/// let codes = numeric_sequence(4, 0, 100);
/// assert_eq!(codes.len(), 101);
/// assert_eq!(codes[0], "0000");
/// ```
pub fn numeric_sequence(digits: usize, start: u64, end: u64) -> Vec<String> {
    (start..=end)
        .map(|n| format!("{:0>width$}", n, width = digits))
        .collect()
}

/// Generate alphanumeric combinations
pub fn alphanumeric_combinations(length: usize, charset: &str) -> Vec<String> {
    if length == 0 {
        return vec![String::new()];
    }

    let chars: Vec<char> = charset.chars().collect();
    let mut results = Vec::new();

    fn generate(chars: &[char], length: usize, current: String, results: &mut Vec<String>) {
        if current.len() == length {
            results.push(current);
            return;
        }
        for &c in chars {
            generate(chars, length, format!("{}{}", current, c), results);
        }
    }

    generate(&chars, length, String::new(), &mut results);
    results
}

/// Common PIN patterns
pub fn common_pins() -> Vec<String> {
    let mut pins = HashSet::new();

    // Sequential
    pins.insert("0000".to_string());
    pins.insert("1111".to_string());
    pins.insert("2222".to_string());
    pins.insert("1234".to_string());
    pins.insert("4321".to_string());
    pins.insert("0123".to_string());
    pins.insert("9876".to_string());

    // Years (birth years, etc.)
    for year in 1950..=2025 {
        pins.insert(format!("{}", year));
    }

    // Dates MMDD
    for month in 1..=12 {
        for day in 1..=31 {
            pins.insert(format!("{:02}{:02}", month, day));
        }
    }

    // Common patterns
    let patterns = [
        "0000", "1111", "2222", "3333", "4444", "5555", "6666", "7777", "8888", "9999", "1234",
        "2345", "3456", "4567", "5678", "6789", "4321", "0987", "1212", "2121", "1010", "2020",
        "6969", "0007", "0666", "1313", "7777", "1122", "2233",
    ];

    for p in patterns {
        pins.insert(p.to_string());
    }

    pins.into_iter().collect()
}

/// Rate limit bypass techniques
#[derive(Debug, Clone)]
pub struct RateLimitBypass {
    pub name: String,
    pub technique: String,
    pub headers: Vec<(String, String)>,
}

impl RateLimitBypass {
    pub fn new(name: &str, technique: &str, headers: Vec<(&str, &str)>) -> Self {
        Self {
            name: name.to_string(),
            technique: technique.to_string(),
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

/// Common rate limit bypass methods
pub fn rate_limit_bypasses() -> Vec<RateLimitBypass> {
    vec![
        RateLimitBypass::new(
            "X-Forwarded-For rotation",
            "Change IP address in X-Forwarded-For header",
            vec![("X-Forwarded-For", "127.0.0.1")],
        ),
        RateLimitBypass::new(
            "X-Real-IP rotation",
            "Change IP address in X-Real-IP header",
            vec![("X-Real-IP", "127.0.0.1")],
        ),
        RateLimitBypass::new(
            "X-Originating-IP",
            "Use X-Originating-IP header",
            vec![("X-Originating-IP", "127.0.0.1")],
        ),
        RateLimitBypass::new(
            "X-Client-IP",
            "Use X-Client-IP header",
            vec![("X-Client-IP", "127.0.0.1")],
        ),
        RateLimitBypass::new(
            "True-Client-IP",
            "Cloudflare True-Client-IP header",
            vec![("True-Client-IP", "127.0.0.1")],
        ),
        RateLimitBypass::new(
            "Multiple headers",
            "Use multiple IP headers",
            vec![
                ("X-Forwarded-For", "127.0.0.1"),
                ("X-Real-IP", "127.0.0.1"),
                ("X-Client-IP", "127.0.0.1"),
            ],
        ),
        RateLimitBypass::new("Case variation", "Use different URL case", vec![]),
        RateLimitBypass::new("Add parameter", "Add random query parameter", vec![]),
        RateLimitBypass::new("HTTP method", "Try different HTTP methods", vec![]),
    ]
}

/// Generate IP addresses for X-Forwarded-For rotation
pub fn generate_ip_rotation(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| {
            format!(
                "{}.{}.{}.{}",
                (i / 256 / 256 / 256) % 256,
                (i / 256 / 256) % 256,
                (i / 256) % 256,
                i % 256
            )
        })
        .collect()
}

/// Username enumeration patterns
pub fn username_enumeration_indicators() -> Vec<EnumerationIndicator> {
    vec![
        EnumerationIndicator {
            indicator_type: IndicatorType::ResponseTime,
            description: "Different response times for valid vs invalid usernames".to_string(),
        },
        EnumerationIndicator {
            indicator_type: IndicatorType::ErrorMessage,
            description: "Different error messages: 'Invalid username' vs 'Invalid password'"
                .to_string(),
        },
        EnumerationIndicator {
            indicator_type: IndicatorType::StatusCode,
            description: "Different status codes for valid/invalid users".to_string(),
        },
        EnumerationIndicator {
            indicator_type: IndicatorType::ResponseSize,
            description: "Different response body sizes".to_string(),
        },
        EnumerationIndicator {
            indicator_type: IndicatorType::PasswordReset,
            description: "Password reset reveals valid accounts".to_string(),
        },
        EnumerationIndicator {
            indicator_type: IndicatorType::Registration,
            description: "Registration shows if email/username exists".to_string(),
        },
    ]
}

#[derive(Debug, Clone)]
pub struct EnumerationIndicator {
    pub indicator_type: IndicatorType,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorType {
    ResponseTime,
    ErrorMessage,
    StatusCode,
    ResponseSize,
    PasswordReset,
    Registration,
}

/// Generate security question answer candidates
pub fn security_question_wordlist(question_type: &str) -> Vec<String> {
    match question_type.to_lowercase().as_str() {
        "pet" | "pet name" | "ペット" => vec![
            "Max", "Buddy", "Charlie", "Jack", "Cooper", "Rocky", "Bear", "Duke", "Tucker",
            "Bella", "Luna", "Lucy", "Daisy", "Molly", "Sadie", "Maggie", "Sophie", "Chloe",
            "Bailey", "Zaya", // Juice Shop
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        "city" | "birthplace" | "出身" => vec![
            "New York",
            "Los Angeles",
            "Chicago",
            "Houston",
            "Phoenix",
            "Philadelphia",
            "San Antonio",
            "San Diego",
            "Dallas",
            "San Jose",
            "Austin",
            "Tokyo",
            "London",
            "Paris",
            "Berlin",
            "Madrid",
            "Rome",
            "Sydney",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        "mother" | "maiden" | "旧姓" => vec![
            "Smith",
            "Johnson",
            "Williams",
            "Brown",
            "Jones",
            "Garcia",
            "Miller",
            "Davis",
            "Rodriguez",
            "Martinez",
            "Hernandez",
            "Lopez",
            "Gonzalez",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        "school" | "高校" | "学校" => vec![
            "Lincoln High",
            "Washington High",
            "Central High",
            "North High",
            "South High",
            "West High",
            "East High",
            "Jefferson High",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        "company" | "employer" | "会社" | "勤務先" => vec![
            "Google",
            "Microsoft",
            "Apple",
            "Amazon",
            "Facebook",
            "Netflix",
            "IBM",
            "Oracle",
            "Intel",
            "Cisco",
            "ITsec",
            "Stop'n'Drop", // Juice Shop
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        "sibling" | "brother" | "sister" | "兄弟" => vec![
            "James",
            "John",
            "Robert",
            "Michael",
            "William",
            "David",
            "Richard",
            "Joseph",
            "Thomas",
            "Charles",
            "Mary",
            "Patricia",
            "Jennifer",
            "Linda",
            "Elizabeth",
            "Barbara",
            "Susan",
            "Samuel", // Juice Shop (Jim's brother)
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect(),

        _ => vec![],
    }
}

/// Password reset token patterns
pub fn reset_token_patterns() -> Vec<TokenPattern> {
    vec![
        TokenPattern {
            name: "Sequential".to_string(),
            description: "Tokens are sequential numbers".to_string(),
            example: "token=1234 → token=1235".to_string(),
        },
        TokenPattern {
            name: "Timestamp-based".to_string(),
            description: "Token is based on timestamp".to_string(),
            example: "token=1706745600 (Unix timestamp)".to_string(),
        },
        TokenPattern {
            name: "Predictable UUID".to_string(),
            description: "UUID v1 contains timestamp".to_string(),
            example: "UUID v1 can be predicted".to_string(),
        },
        TokenPattern {
            name: "User ID based".to_string(),
            description: "Token derived from user ID".to_string(),
            example: "base64(user_id) or md5(user_id)".to_string(),
        },
        TokenPattern {
            name: "Email hash".to_string(),
            description: "Token is hash of email".to_string(),
            example: "md5(email) or sha1(email)".to_string(),
        },
    ]
}

#[derive(Debug, Clone)]
pub struct TokenPattern {
    pub name: String,
    pub description: String,
    pub example: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_sequence() {
        let codes = numeric_sequence(4, 0, 10);
        assert_eq!(codes.len(), 11);
        assert_eq!(codes[0], "0000");
        assert_eq!(codes[10], "0010");
    }

    #[test]
    fn test_common_pins() {
        let pins = common_pins();
        assert!(pins.contains(&"1234".to_string()));
        assert!(pins.contains(&"0000".to_string()));
    }

    #[test]
    fn test_rate_limit_bypasses() {
        let bypasses = rate_limit_bypasses();
        assert!(!bypasses.is_empty());
        assert!(bypasses.iter().any(|b| b.name.contains("X-Forwarded-For")));
    }

    #[test]
    fn test_generate_ip_rotation() {
        let ips = generate_ip_rotation(10);
        assert_eq!(ips.len(), 10);
        assert!(ips[0].contains('.'));
    }

    #[test]
    fn test_security_question_wordlist() {
        let pets = security_question_wordlist("pet");
        assert!(pets.contains(&"Zaya".to_string()));

        let companies = security_question_wordlist("company");
        assert!(companies.contains(&"Stop'n'Drop".to_string()));
    }
}
