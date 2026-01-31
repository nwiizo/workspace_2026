//! Mutation strategies for fuzzing payloads
//!
//! Provides various encoding and transformation strategies that can be applied
//! to payloads during fuzzing to bypass filters and explore edge cases.

use urlencoding;

/// Mutation strategy for transforming payloads
#[derive(Debug, Clone, PartialEq)]
pub enum MutationStrategy {
    /// No mutation, use payload as-is
    None,
    /// URL encoding variations
    UrlEncode {
        /// Apply double URL encoding
        double: bool,
    },
    /// HTML entity encoding
    HtmlEncode,
    /// Unicode variations (homoglyphs, etc.)
    Unicode,
    /// Case variations (upper, lower, mixed)
    CaseVariation,
    /// Add prefix/suffix
    Wrapper { prefix: String, suffix: String },
    /// Combine multiple strategies
    Chain(Vec<MutationStrategy>),
    /// Hex encoding
    HexEncode,
    /// Base64 encoding
    Base64Encode,
    /// Reverse the payload
    Reverse,
    /// Insert null bytes
    NullByte,
    /// Newline variations
    NewlineVariation,
}

impl MutationStrategy {
    /// Apply the mutation strategy to a payload, returning all variations
    pub fn apply(&self, payload: &str) -> Vec<String> {
        match self {
            Self::None => vec![payload.to_string()],

            Self::UrlEncode { double } => {
                let encoded = urlencoding::encode(payload).to_string();
                if *double {
                    vec![encoded.clone(), urlencoding::encode(&encoded).to_string()]
                } else {
                    vec![encoded]
                }
            }

            Self::HtmlEncode => {
                vec![
                    html_encode(payload),
                    html_encode_decimal(payload),
                    html_encode_hex(payload),
                ]
            }

            Self::Unicode => unicode_variations(payload),

            Self::CaseVariation => case_variations(payload),

            Self::Wrapper { prefix, suffix } => {
                vec![format!("{}{}{}", prefix, payload, suffix)]
            }

            Self::Chain(strategies) => {
                // Chain collects all variations from all strategies applied to the original payload
                let mut results = Vec::new();
                for strategy in strategies {
                    results.extend(strategy.apply(payload));
                }
                // Deduplicate while preserving order
                let mut seen = std::collections::HashSet::new();
                results.retain(|x| seen.insert(x.clone()));
                results
            }

            Self::HexEncode => {
                vec![
                    payload.bytes().map(|b| format!("\\x{:02x}", b)).collect(),
                    payload.bytes().map(|b| format!("%{:02x}", b)).collect(),
                ]
            }

            Self::Base64Encode => {
                use base64::{Engine as _, engine::general_purpose};
                vec![general_purpose::STANDARD.encode(payload)]
            }

            Self::Reverse => {
                vec![payload.chars().rev().collect()]
            }

            Self::NullByte => {
                vec![
                    format!("{}\x00", payload),
                    format!("{}%00", payload),
                    format!("{}\0", payload),
                ]
            }

            Self::NewlineVariation => {
                vec![
                    format!("{}\n", payload),
                    format!("{}\r\n", payload),
                    format!("{}\r", payload),
                    format!("\n{}", payload),
                    format!("{}\n{}", payload, payload),
                ]
            }
        }
    }

    /// Create a chain of all common encoding strategies
    pub fn all_encodings() -> Self {
        Self::Chain(vec![
            Self::None,
            Self::UrlEncode { double: false },
            Self::UrlEncode { double: true },
            Self::HtmlEncode,
            Self::HexEncode,
        ])
    }

    /// Create a URL encoding strategy
    pub fn url_encode() -> Self {
        Self::UrlEncode { double: false }
    }

    /// Create a double URL encoding strategy
    pub fn double_url_encode() -> Self {
        Self::UrlEncode { double: true }
    }

    /// Create a wrapper strategy with prefix and suffix
    pub fn wrap(prefix: impl Into<String>, suffix: impl Into<String>) -> Self {
        Self::Wrapper {
            prefix: prefix.into(),
            suffix: suffix.into(),
        }
    }

    /// Create a chain from multiple strategies
    pub fn chain(strategies: Vec<MutationStrategy>) -> Self {
        Self::Chain(strategies)
    }

    /// Common XSS bypass encodings
    pub fn xss_bypass_encodings() -> Self {
        Self::Chain(vec![
            Self::None,
            Self::HtmlEncode,
            Self::UrlEncode { double: false },
            Self::Unicode,
            Self::CaseVariation,
        ])
    }

    /// Common SQLi bypass encodings
    pub fn sqli_bypass_encodings() -> Self {
        Self::Chain(vec![
            Self::None,
            Self::UrlEncode { double: false },
            Self::UrlEncode { double: true },
            Self::Unicode,
            Self::CaseVariation,
        ])
    }
}

/// HTML entity encode a string using named entities
fn html_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#x27;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// HTML entity encode using decimal numeric references
fn html_encode_decimal(s: &str) -> String {
    s.chars().map(|c| format!("&#{};", c as u32)).collect()
}

/// HTML entity encode using hexadecimal numeric references
fn html_encode_hex(s: &str) -> String {
    s.chars().map(|c| format!("&#x{:x};", c as u32)).collect()
}

/// Generate Unicode variations of a string (homoglyphs)
fn unicode_variations(s: &str) -> Vec<String> {
    let mut variations = vec![s.to_string()];

    // Common homoglyph substitutions
    let homoglyphs: Vec<(char, &[char])> = vec![
        ('a', &['а', 'ɑ', 'α']),          // Cyrillic а, Latin alpha, Greek alpha
        ('e', &['е', 'ɛ', 'ε']),          // Cyrillic е, etc.
        ('o', &['о', 'ο', '0']),          // Cyrillic о, Greek omicron, zero
        ('c', &['с', 'ϲ']),               // Cyrillic с, Greek lunate sigma
        ('p', &['р', 'ρ']),               // Cyrillic р, Greek rho
        ('s', &['ѕ', 'ꜱ']),               // Cyrillic ѕ
        ('x', &['х', 'χ']),               // Cyrillic х, Greek chi
        ('i', &['і', 'ι', '1']),          // Cyrillic і, Greek iota
        ('<', &['\u{FF1C}']),             // Fullwidth less-than
        ('>', &['\u{FF1E}']),             // Fullwidth greater-than
        ('/', &['\u{2215}', '\u{FF0F}']), // Division slash, fullwidth solidus
    ];

    // Generate variations by substituting one character at a time
    for (original, substitutes) in homoglyphs {
        for sub in substitutes.iter() {
            let variation = s.replace(original, &sub.to_string());
            if variation != s && !variations.contains(&variation) {
                variations.push(variation);
            }
        }
    }

    variations
}

/// Generate case variations of a string
fn case_variations(s: &str) -> Vec<String> {
    let mut variations = vec![s.to_string(), s.to_uppercase(), s.to_lowercase()];

    // Mixed case: alternate
    let alternating: String = s
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i % 2 == 0 {
                c.to_uppercase().next().unwrap_or(c)
            } else {
                c.to_lowercase().next().unwrap_or(c)
            }
        })
        .collect();
    if !variations.contains(&alternating) {
        variations.push(alternating);
    }

    // Capitalize first letter only
    let capitalized = capitalize_first(s);
    if !variations.contains(&capitalized) {
        variations.push(capitalized);
    }

    variations
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_mutation() {
        let strategy = MutationStrategy::None;
        let result = strategy.apply("test");
        assert_eq!(result, vec!["test"]);
    }

    #[test]
    fn test_url_encode() {
        let strategy = MutationStrategy::UrlEncode { double: false };
        let result = strategy.apply("<script>");
        assert_eq!(result, vec!["%3Cscript%3E"]);
    }

    #[test]
    fn test_double_url_encode() {
        let strategy = MutationStrategy::UrlEncode { double: true };
        let result = strategy.apply("<");
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"%3C".to_string()));
        assert!(result.contains(&"%253C".to_string()));
    }

    #[test]
    fn test_html_encode() {
        let strategy = MutationStrategy::HtmlEncode;
        let result = strategy.apply("<script>");
        assert!(result.iter().any(|s| s.contains("&lt;")));
        assert!(result.iter().any(|s| s.contains("&#60;"))); // decimal
        assert!(result.iter().any(|s| s.contains("&#x3c;"))); // hex
    }

    #[test]
    fn test_case_variations() {
        let strategy = MutationStrategy::CaseVariation;
        let result = strategy.apply("Script");
        assert!(result.contains(&"Script".to_string()));
        assert!(result.contains(&"SCRIPT".to_string()));
        assert!(result.contains(&"script".to_string()));
    }

    #[test]
    fn test_wrapper() {
        let strategy = MutationStrategy::wrap("<!--", "-->");
        let result = strategy.apply("payload");
        assert_eq!(result, vec!["<!--payload-->"]);
    }

    #[test]
    fn test_chain() {
        let strategy = MutationStrategy::chain(vec![
            MutationStrategy::None,
            MutationStrategy::UrlEncode { double: false },
        ]);
        let result = strategy.apply("<");
        // Chain applies each strategy to all results from previous
        assert!(result.contains(&"<".to_string()));
        assert!(result.contains(&"%3C".to_string()));
    }

    #[test]
    fn test_hex_encode() {
        let strategy = MutationStrategy::HexEncode;
        let result = strategy.apply("AB");
        assert!(result.iter().any(|s| s.contains("\\x41\\x42")));
        assert!(result.iter().any(|s| s.contains("%41%42")));
    }

    #[test]
    fn test_base64_encode() {
        let strategy = MutationStrategy::Base64Encode;
        let result = strategy.apply("test");
        assert_eq!(result, vec!["dGVzdA=="]);
    }

    #[test]
    fn test_null_byte() {
        let strategy = MutationStrategy::NullByte;
        let result = strategy.apply("test");
        assert!(result.iter().any(|s| s.ends_with("%00")));
    }

    #[test]
    fn test_all_encodings() {
        let strategy = MutationStrategy::all_encodings();
        let result = strategy.apply("<");
        assert!(result.len() > 1);
        assert!(result.contains(&"<".to_string()));
        assert!(result.contains(&"%3C".to_string()));
    }
}
