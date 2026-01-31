//! Payload generators for fuzzing
//!
//! Provides functions to generate boundary value payloads, format strings,
//! special characters, and other common fuzzing test cases.

/// Generate boundary value payloads for integers
///
/// Returns values at and around common boundary conditions.
///
/// # Example
/// ```
/// use rectitude::fuzzing::generator::integer_boundaries;
/// let values = integer_boundaries(0, 100);
/// assert!(values.contains(&0));
/// assert!(values.contains(&100));
/// assert!(values.contains(&-1)); // Below min
/// assert!(values.contains(&101)); // Above max
/// ```
pub fn integer_boundaries(min: i64, max: i64) -> Vec<i64> {
    let mut values = vec![
        0,
        1,
        -1,
        min,
        max,
        min.saturating_sub(1),
        max.saturating_add(1),
        min.saturating_add(1),
        max.saturating_sub(1),
        i8::MIN as i64,
        i8::MAX as i64,
        i16::MIN as i64,
        i16::MAX as i64,
        i32::MIN as i64,
        i32::MAX as i64,
        u8::MAX as i64,
        u16::MAX as i64,
        u32::MAX as i64,
    ];

    // Dedup and sort
    values.sort();
    values.dedup();
    values
}

/// Generate boundary value payloads as strings
///
/// Useful when you need string representations of boundary values.
pub fn integer_boundaries_str(min: i64, max: i64) -> Vec<String> {
    integer_boundaries(min, max)
        .into_iter()
        .map(|v| v.to_string())
        .collect()
}

/// Generate string length payloads
///
/// Returns strings of various lengths for testing length validation.
///
/// # Example
/// ```
/// use rectitude::fuzzing::generator::string_lengths;
/// let strings = string_lengths(10);
/// assert!(strings.iter().any(|s| s.is_empty())); // Empty string
/// assert!(strings.iter().any(|s| s.len() == 10)); // Max length
/// assert!(strings.iter().any(|s| s.len() > 10)); // Over max
/// ```
pub fn string_lengths(max_len: usize) -> Vec<String> {
    let mut strings = vec![
        String::new(),                         // Empty
        "a".to_string(),                       // Single char
        "aa".to_string(),                      // Two chars
        "a".repeat(max_len.saturating_sub(1)), // Just under max
        "a".repeat(max_len),                   // Exactly max
        "a".repeat(max_len + 1),               // Just over max
        "a".repeat(max_len + 10),              // Well over max
        "a".repeat(max_len * 2),               // Double max
    ];

    // Add some power-of-2 lengths if they're interesting
    for pow in [4, 8, 16, 32, 64, 128, 256, 512, 1024] {
        if pow <= max_len * 2 && pow > 2 {
            strings.push("a".repeat(pow));
        }
    }

    strings.sort_by_key(|s| s.len());
    strings.dedup();
    strings
}

/// Generate format string payloads
///
/// Returns common format string attack patterns.
pub fn format_strings() -> Vec<String> {
    vec![
        // C-style format strings
        "%s".to_string(),
        "%n".to_string(),
        "%x".to_string(),
        "%d".to_string(),
        "%p".to_string(),
        "%s%s%s%s%s".to_string(),
        "%n%n%n%n%n".to_string(),
        "%x%x%x%x%x".to_string(),
        "AAAA%08x.%08x.%08x.%08x".to_string(),
        "%1000000s".to_string(),
        // Python/Rust format strings
        "{}".to_string(),
        "{0}".to_string(),
        "{name}".to_string(),
        "{{".to_string(),
        "}}".to_string(),
        "{0.__class__}".to_string(),
        "{0.__class__.__mro__}".to_string(),
        // Ruby/ERB
        "#{system('id')}".to_string(),
        "<%=7*7%>".to_string(),
        // Java MessageFormat
        "{0,number,#}".to_string(),
    ]
}

/// Generate special character payloads
///
/// Returns strings containing special characters that may cause issues.
pub fn special_chars() -> Vec<String> {
    vec![
        "\x00".to_string(),     // Null byte
        "\n".to_string(),       // Newline
        "\r".to_string(),       // Carriage return
        "\r\n".to_string(),     // CRLF
        "\t".to_string(),       // Tab
        "\x0b".to_string(),     // Vertical tab
        "\x0c".to_string(),     // Form feed
        "\x1b".to_string(),     // Escape
        "\x7f".to_string(),     // Delete
        " ".to_string(),        // Space
        "\u{00A0}".to_string(), // Non-breaking space
        "\u{2000}".to_string(), // En quad
        "\u{2028}".to_string(), // Line separator
        "\u{2029}".to_string(), // Paragraph separator
        "\u{FEFF}".to_string(), // BOM
        "\u{FFFF}".to_string(), // Non-character
        "\\".to_string(),       // Backslash
        "\"".to_string(),       // Double quote
        "'".to_string(),        // Single quote
        "`".to_string(),        // Backtick
        "$".to_string(),        // Dollar sign
        "&".to_string(),        // Ampersand
        "|".to_string(),        // Pipe
        ";".to_string(),        // Semicolon
        "*".to_string(),        // Asterisk
        "?".to_string(),        // Question mark
        "[".to_string(),        // Left bracket
        "]".to_string(),        // Right bracket
        "{".to_string(),        // Left brace
        "}".to_string(),        // Right brace
        "(#".to_string(),       // Parenthesis
        ")".to_string(),
        "<".to_string(), // Less than
        ">".to_string(), // Greater than
        "!".to_string(), // Exclamation
        "@".to_string(), // At sign
        "#".to_string(), // Hash
        "^".to_string(), // Caret
        "~".to_string(), // Tilde
    ]
}

/// Generate numeric edge case payloads
///
/// Returns string representations of problematic numeric values.
pub fn numeric_edges() -> Vec<String> {
    vec![
        "0".to_string(),
        "-0".to_string(),
        "00".to_string(),
        "0.0".to_string(),
        "-0.0".to_string(),
        "1".to_string(),
        "-1".to_string(),
        "0.1".to_string(),
        "-0.1".to_string(),
        "1e10".to_string(),
        "1e-10".to_string(),
        "1e308".to_string(), // Near max double
        "1e309".to_string(), // Overflow to infinity
        "-1e308".to_string(),
        "-1e309".to_string(),
        "NaN".to_string(),
        "nan".to_string(),
        "NAN".to_string(),
        "Infinity".to_string(),
        "infinity".to_string(),
        "INFINITY".to_string(),
        "-Infinity".to_string(),
        "+Infinity".to_string(),
        "Inf".to_string(),
        "-Inf".to_string(),
        "+Inf".to_string(),
        "0x0".to_string(), // Hex
        "0x7FFFFFFF".to_string(),
        "0xFFFFFFFF".to_string(),
        "0o0".to_string(), // Octal
        "0o777".to_string(),
        "0b0".to_string(), // Binary
        "0b1111111111111111".to_string(),
        "2147483647".to_string(),              // i32::MAX
        "2147483648".to_string(),              // i32::MAX + 1
        "-2147483648".to_string(),             // i32::MIN
        "-2147483649".to_string(),             // i32::MIN - 1
        "9223372036854775807".to_string(),     // i64::MAX
        "9223372036854775808".to_string(),     // i64::MAX + 1
        "-9223372036854775808".to_string(),    // i64::MIN
        "18446744073709551615".to_string(),    // u64::MAX
        "18446744073709551616".to_string(),    // u64::MAX + 1
        "1.7976931348623157E+308".to_string(), // f64::MAX
        "2.2250738585072014E-308".to_string(), // f64::MIN_POSITIVE
        "4.9406564584124654E-324".to_string(), // Smallest positive subnormal
        "".to_string(),                        // Empty
        " ".to_string(),                       // Space
        " 1".to_string(),                      // Leading space
        "1 ".to_string(),                      // Trailing space
        "1.".to_string(),                      // Trailing dot
        ".1".to_string(),                      // Leading dot
        "1,000".to_string(),                   // Comma separator
        "1_000".to_string(),                   // Underscore separator
        "+1".to_string(),                      // Explicit positive
        "++1".to_string(),                     // Double plus
        "--1".to_string(),                     // Double minus
    ]
}

/// Generate boolean-like payloads
///
/// Returns various representations of true/false values.
pub fn boolean_values() -> Vec<String> {
    vec![
        "true".to_string(),
        "false".to_string(),
        "True".to_string(),
        "False".to_string(),
        "TRUE".to_string(),
        "FALSE".to_string(),
        "1".to_string(),
        "0".to_string(),
        "yes".to_string(),
        "no".to_string(),
        "Yes".to_string(),
        "No".to_string(),
        "YES".to_string(),
        "NO".to_string(),
        "on".to_string(),
        "off".to_string(),
        "On".to_string(),
        "Off".to_string(),
        "ON".to_string(),
        "OFF".to_string(),
        "y".to_string(),
        "n".to_string(),
        "Y".to_string(),
        "N".to_string(),
        "t".to_string(),
        "f".to_string(),
        "T".to_string(),
        "F".to_string(),
        "".to_string(), // Empty
        "null".to_string(),
        "NULL".to_string(),
        "Null".to_string(),
        "nil".to_string(),
        "none".to_string(),
        "None".to_string(),
        "undefined".to_string(),
    ]
}

/// Generate date/time edge case payloads
pub fn datetime_edges() -> Vec<String> {
    vec![
        // Valid dates at boundaries
        "1970-01-01".to_string(), // Unix epoch
        "1970-01-01T00:00:00Z".to_string(),
        "2038-01-19T03:14:07Z".to_string(), // 32-bit overflow
        "2038-01-19T03:14:08Z".to_string(),
        "9999-12-31".to_string(),           // Far future
        "0001-01-01".to_string(),           // Far past
        "1969-12-31T23:59:59Z".to_string(), // Before epoch
        // Invalid dates
        "0000-00-00".to_string(),
        "2024-13-01".to_string(), // Invalid month
        "2024-02-30".to_string(), // Invalid day
        "2024-02-29".to_string(), // Leap year
        "2023-02-29".to_string(), // Non-leap year
        // Edge time values
        "00:00:00".to_string(),
        "23:59:59".to_string(),
        "24:00:00".to_string(), // Midnight next day
        "25:00:00".to_string(), // Invalid hour
        "12:60:00".to_string(), // Invalid minute
        "12:00:60".to_string(), // Invalid second
        // Various formats
        "01/01/2024".to_string(),
        "1/1/2024".to_string(),
        "2024/01/01".to_string(),
        "01-Jan-2024".to_string(),
        "January 1, 2024".to_string(),
        "2024-W01".to_string(), // Week number
        "2024-001".to_string(), // Day of year
        // Timezone edge cases
        "2024-01-01T00:00:00+00:00".to_string(),
        "2024-01-01T00:00:00-12:00".to_string(),
        "2024-01-01T00:00:00+14:00".to_string(),
        // Empty and special
        "".to_string(),
        "now".to_string(),
        "today".to_string(),
        "-1".to_string(),
    ]
}

/// Generate email-like payloads for testing email validation
pub fn email_edge_cases() -> Vec<String> {
    vec![
        // Valid but unusual
        "test@example.com".to_string(),
        "test+tag@example.com".to_string(),
        "test.name@example.com".to_string(),
        "test@subdomain.example.com".to_string(),
        "\"test\"@example.com".to_string(),
        "test@123.123.123.123".to_string(),
        "test@[123.123.123.123]".to_string(),
        "a@b.co".to_string(),
        "very.long.email.address.that.is.still.valid@very.long.domain.name.example.com".to_string(),
        // Invalid
        "".to_string(),
        "test".to_string(),
        "@example.com".to_string(),
        "test@".to_string(),
        "test@.com".to_string(),
        "test@@example.com".to_string(),
        "test @example.com".to_string(),
        "test@ example.com".to_string(),
        "test@example".to_string(),
        "test@example..com".to_string(),
        "<test@example.com>".to_string(),
        "test@example.com.".to_string(),
        ".test@example.com".to_string(),
        // Injection attempts
        "test@example.com\nBcc: attacker@evil.com".to_string(),
        "test@example.com%0ABcc:attacker@evil.com".to_string(),
        "test@example.com\r\nBcc: attacker@evil.com".to_string(),
    ]
}

/// Generate URL-like payloads for testing URL validation
pub fn url_edge_cases() -> Vec<String> {
    vec![
        // Valid
        "http://example.com".to_string(),
        "https://example.com".to_string(),
        "https://example.com/path".to_string(),
        "https://example.com/path?query=value".to_string(),
        "https://example.com/path#fragment".to_string(),
        "https://user:pass@example.com".to_string(),
        "https://example.com:8080".to_string(),
        // Protocol variations
        "ftp://example.com".to_string(),
        "file:///etc/passwd".to_string(),
        "javascript:alert(1)".to_string(),
        "data:text/html,<script>alert(1)</script>".to_string(),
        // Invalid/edge cases
        "".to_string(),
        "//example.com".to_string(), // Protocol-relative
        "/path".to_string(),         // Relative path
        "example.com".to_string(),   // No protocol
        "http://".to_string(),
        "http:///".to_string(),
        "http://localhost".to_string(),
        "http://127.0.0.1".to_string(),
        "http://0.0.0.0".to_string(),
        "http://[::1]".to_string(),           // IPv6 localhost
        "http://169.254.169.254".to_string(), // AWS metadata
        // SSRF payloads
        "http://localhost:22".to_string(),
        "http://127.0.0.1:6379".to_string(),
        "gopher://127.0.0.1:6379/_".to_string(),
        "dict://127.0.0.1:6379/info".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_boundaries() {
        let values = integer_boundaries(0, 100);
        assert!(values.contains(&0));
        assert!(values.contains(&100));
        assert!(values.contains(&-1));
        assert!(values.contains(&101));
        assert!(values.contains(&1));
    }

    #[test]
    fn test_string_lengths() {
        let strings = string_lengths(10);
        assert!(strings.iter().any(|s| s.is_empty()));
        assert!(strings.iter().any(|s| s.len() == 10));
        assert!(strings.iter().any(|s| s.len() == 11));
    }

    #[test]
    fn test_format_strings() {
        let payloads = format_strings();
        assert!(payloads.contains(&"%s".to_string()));
        assert!(payloads.contains(&"%n".to_string()));
        assert!(payloads.contains(&"{}".to_string()));
    }

    #[test]
    fn test_special_chars() {
        let chars = special_chars();
        assert!(chars.contains(&"\x00".to_string()));
        assert!(chars.contains(&"\n".to_string()));
        assert!(chars.contains(&"'".to_string()));
    }

    #[test]
    fn test_numeric_edges() {
        let edges = numeric_edges();
        assert!(edges.contains(&"0".to_string()));
        assert!(edges.contains(&"NaN".to_string()));
        assert!(edges.contains(&"Infinity".to_string()));
    }

    #[test]
    fn test_boolean_values() {
        let values = boolean_values();
        assert!(values.contains(&"true".to_string()));
        assert!(values.contains(&"false".to_string()));
        assert!(values.contains(&"1".to_string()));
        assert!(values.contains(&"0".to_string()));
    }
}
