//! Security headers analysis
//!
//! Check for missing or misconfigured security headers.

use std::collections::HashMap;

/// Security header check result
#[derive(Debug, Clone)]
pub struct HeaderCheck {
    pub name: String,
    pub present: bool,
    pub value: Option<String>,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl HeaderCheck {
    fn missing(name: &str, severity: Severity, description: &str, recommendation: &str) -> Self {
        Self {
            name: name.to_string(),
            present: false,
            value: None,
            severity,
            description: description.to_string(),
            recommendation: recommendation.to_string(),
        }
    }

    fn found(
        name: &str,
        value: &str,
        severity: Severity,
        description: &str,
        recommendation: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            present: true,
            value: Some(value.to_string()),
            severity,
            description: description.to_string(),
            recommendation: recommendation.to_string(),
        }
    }

    fn ok(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            present: true,
            value: Some(value.to_string()),
            severity: Severity::Info,
            description: "Header configured correctly".to_string(),
            recommendation: String::new(),
        }
    }
}

/// Analyze security headers from a response
pub fn analyze_headers(headers: &HashMap<String, String>) -> Vec<HeaderCheck> {
    let mut results = Vec::new();

    // Normalize header names to lowercase for comparison
    let headers_lower: HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.to_lowercase(), v.clone()))
        .collect();

    // Strict-Transport-Security (HSTS)
    results.push(check_hsts(&headers_lower));

    // Content-Security-Policy
    results.push(check_csp(&headers_lower));

    // X-Content-Type-Options
    results.push(check_content_type_options(&headers_lower));

    // X-Frame-Options
    results.push(check_frame_options(&headers_lower));

    // X-XSS-Protection
    results.push(check_xss_protection(&headers_lower));

    // Referrer-Policy
    results.push(check_referrer_policy(&headers_lower));

    // Permissions-Policy
    results.push(check_permissions_policy(&headers_lower));

    // Cache-Control (for sensitive pages)
    results.push(check_cache_control(&headers_lower));

    // Server header (information disclosure)
    results.push(check_server_header(&headers_lower));

    // X-Powered-By (information disclosure)
    results.push(check_powered_by(&headers_lower));

    // CORS headers
    if let Some(cors_check) = check_cors(&headers_lower) {
        results.push(cors_check);
    }

    results
}

fn check_hsts(headers: &HashMap<String, String>) -> HeaderCheck {
    match headers.get("strict-transport-security") {
        None => HeaderCheck::missing(
            "Strict-Transport-Security",
            Severity::High,
            "HSTS not enabled - vulnerable to SSL stripping attacks",
            "Add: Strict-Transport-Security: max-age=31536000; includeSubDomains",
        ),
        Some(value) => {
            if !value.contains("max-age") {
                HeaderCheck::found(
                    "Strict-Transport-Security",
                    value,
                    Severity::Medium,
                    "HSTS present but missing max-age directive",
                    "Add max-age directive with at least 1 year (31536000)",
                )
            } else if let Some(age) = extract_max_age(value) {
                if age < 31536000 {
                    HeaderCheck::found(
                        "Strict-Transport-Security",
                        value,
                        Severity::Low,
                        "HSTS max-age is less than recommended (1 year)",
                        "Increase max-age to at least 31536000 (1 year)",
                    )
                } else {
                    HeaderCheck::ok("Strict-Transport-Security", value)
                }
            } else {
                HeaderCheck::ok("Strict-Transport-Security", value)
            }
        }
    }
}

fn check_csp(headers: &HashMap<String, String>) -> HeaderCheck {
    match headers.get("content-security-policy") {
        None => HeaderCheck::missing(
            "Content-Security-Policy",
            Severity::High,
            "No CSP - vulnerable to XSS and data injection attacks",
            "Implement a strict Content-Security-Policy",
        ),
        Some(value) => {
            let mut issues = Vec::new();

            if value.contains("'unsafe-inline'") {
                issues.push("unsafe-inline allows inline scripts");
            }
            if value.contains("'unsafe-eval'") {
                issues.push("unsafe-eval allows eval()");
            }
            if value.contains("data:") && !value.contains("img-src") {
                issues.push("data: URIs can be used for XSS");
            }
            if !value.contains("default-src") && !value.contains("script-src") {
                issues.push("Missing default-src or script-src directive");
            }

            if issues.is_empty() {
                HeaderCheck::ok("Content-Security-Policy", value)
            } else {
                HeaderCheck::found(
                    "Content-Security-Policy",
                    value,
                    Severity::Medium,
                    &format!("CSP weaknesses: {}", issues.join(", ")),
                    "Remove unsafe-inline/unsafe-eval, use nonces or hashes",
                )
            }
        }
    }
}

fn check_content_type_options(headers: &HashMap<String, String>) -> HeaderCheck {
    match headers.get("x-content-type-options") {
        None => HeaderCheck::missing(
            "X-Content-Type-Options",
            Severity::Medium,
            "Missing X-Content-Type-Options - vulnerable to MIME sniffing",
            "Add: X-Content-Type-Options: nosniff",
        ),
        Some(value) if value.to_lowercase() == "nosniff" => {
            HeaderCheck::ok("X-Content-Type-Options", value)
        }
        Some(value) => HeaderCheck::found(
            "X-Content-Type-Options",
            value,
            Severity::Low,
            "Invalid X-Content-Type-Options value",
            "Set value to 'nosniff'",
        ),
    }
}

fn check_frame_options(headers: &HashMap<String, String>) -> HeaderCheck {
    // Check both X-Frame-Options and CSP frame-ancestors
    let xfo = headers.get("x-frame-options");
    let csp = headers.get("content-security-policy");
    let has_frame_ancestors = csp.map(|c| c.contains("frame-ancestors")).unwrap_or(false);

    match (xfo, has_frame_ancestors) {
        (None, false) => HeaderCheck::missing(
            "X-Frame-Options",
            Severity::Medium,
            "No clickjacking protection - vulnerable to UI redressing",
            "Add: X-Frame-Options: DENY or CSP frame-ancestors 'none'",
        ),
        (Some(value), _) => {
            let v = value.to_uppercase();
            if v == "DENY" || v == "SAMEORIGIN" {
                HeaderCheck::ok("X-Frame-Options", value)
            } else if v.starts_with("ALLOW-FROM") {
                HeaderCheck::found(
                    "X-Frame-Options",
                    value,
                    Severity::Low,
                    "ALLOW-FROM is deprecated and not supported by modern browsers",
                    "Use CSP frame-ancestors instead",
                )
            } else {
                HeaderCheck::found(
                    "X-Frame-Options",
                    value,
                    Severity::Low,
                    "Invalid X-Frame-Options value",
                    "Use DENY or SAMEORIGIN",
                )
            }
        }
        (None, true) => HeaderCheck::ok("X-Frame-Options", "(Using CSP frame-ancestors)"),
    }
}

fn check_xss_protection(headers: &HashMap<String, String>) -> HeaderCheck {
    match headers.get("x-xss-protection") {
        None => HeaderCheck::found(
            "X-XSS-Protection",
            "(not set)",
            Severity::Info,
            "X-XSS-Protection is deprecated - modern browsers don't need it",
            "Remove if present, rely on CSP instead",
        ),
        Some(value) if value == "0" => HeaderCheck::ok("X-XSS-Protection", value),
        Some(value) => HeaderCheck::found(
            "X-XSS-Protection",
            value,
            Severity::Low,
            "X-XSS-Protection is deprecated and can cause issues",
            "Set to '0' or remove entirely, use CSP instead",
        ),
    }
}

fn check_referrer_policy(headers: &HashMap<String, String>) -> HeaderCheck {
    match headers.get("referrer-policy") {
        None => HeaderCheck::missing(
            "Referrer-Policy",
            Severity::Low,
            "No Referrer-Policy - may leak sensitive URLs to external sites",
            "Add: Referrer-Policy: strict-origin-when-cross-origin",
        ),
        Some(value) => {
            let safe_policies = [
                "no-referrer",
                "same-origin",
                "strict-origin",
                "strict-origin-when-cross-origin",
            ];
            let v = value.to_lowercase();
            if safe_policies.iter().any(|p| v.contains(p)) {
                HeaderCheck::ok("Referrer-Policy", value)
            } else if v.contains("unsafe-url") || v.contains("no-referrer-when-downgrade") {
                HeaderCheck::found(
                    "Referrer-Policy",
                    value,
                    Severity::Low,
                    "Referrer-Policy may leak sensitive URL information",
                    "Use strict-origin-when-cross-origin or stricter",
                )
            } else {
                HeaderCheck::ok("Referrer-Policy", value)
            }
        }
    }
}

fn check_permissions_policy(headers: &HashMap<String, String>) -> HeaderCheck {
    // Check both Permissions-Policy and deprecated Feature-Policy
    let pp = headers.get("permissions-policy");
    let fp = headers.get("feature-policy");

    match (pp, fp) {
        (None, None) => HeaderCheck::missing(
            "Permissions-Policy",
            Severity::Low,
            "No Permissions-Policy - browser features not restricted",
            "Add Permissions-Policy to restrict camera, microphone, geolocation, etc.",
        ),
        (Some(value), _) => HeaderCheck::ok("Permissions-Policy", value),
        (None, Some(value)) => HeaderCheck::found(
            "Permissions-Policy",
            value,
            Severity::Info,
            "Using deprecated Feature-Policy header",
            "Migrate to Permissions-Policy header",
        ),
    }
}

fn check_cache_control(headers: &HashMap<String, String>) -> HeaderCheck {
    match headers.get("cache-control") {
        None => HeaderCheck::missing(
            "Cache-Control",
            Severity::Info,
            "No Cache-Control header",
            "For sensitive pages: Cache-Control: no-store, no-cache, must-revalidate",
        ),
        Some(value) => {
            if value.contains("no-store") || value.contains("private") {
                HeaderCheck::ok("Cache-Control", value)
            } else if value.contains("public") {
                HeaderCheck::found(
                    "Cache-Control",
                    value,
                    Severity::Info,
                    "Public caching may expose sensitive data",
                    "Use 'private' or 'no-store' for sensitive content",
                )
            } else {
                HeaderCheck::ok("Cache-Control", value)
            }
        }
    }
}

fn check_server_header(headers: &HashMap<String, String>) -> HeaderCheck {
    match headers.get("server") {
        None => HeaderCheck::ok("Server", "(not disclosed)"),
        Some(value) => {
            // Check if version is disclosed
            if value.chars().any(|c| c.is_ascii_digit()) {
                HeaderCheck::found(
                    "Server",
                    value,
                    Severity::Low,
                    "Server version disclosed - aids attacker reconnaissance",
                    "Remove version information from Server header",
                )
            } else {
                HeaderCheck::found(
                    "Server",
                    value,
                    Severity::Info,
                    "Server type disclosed",
                    "Consider removing Server header entirely",
                )
            }
        }
    }
}

fn check_powered_by(headers: &HashMap<String, String>) -> HeaderCheck {
    match headers.get("x-powered-by") {
        None => HeaderCheck::ok("X-Powered-By", "(not disclosed)"),
        Some(value) => HeaderCheck::found(
            "X-Powered-By",
            value,
            Severity::Low,
            "Technology stack disclosed - aids attacker reconnaissance",
            "Remove X-Powered-By header",
        ),
    }
}

fn check_cors(headers: &HashMap<String, String>) -> Option<HeaderCheck> {
    let acao = headers.get("access-control-allow-origin")?;

    if acao == "*" {
        Some(HeaderCheck::found(
            "Access-Control-Allow-Origin",
            acao,
            Severity::Medium,
            "CORS allows any origin - may expose data to malicious sites",
            "Restrict to specific trusted origins",
        ))
    } else if headers
        .get("access-control-allow-credentials")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        if acao == "*" || acao == "null" {
            Some(HeaderCheck::found(
                "Access-Control-Allow-Origin",
                acao,
                Severity::High,
                "CORS with credentials and wildcard/null origin is dangerous",
                "Never use credentials with wildcard or null origin",
            ))
        } else {
            Some(HeaderCheck::ok("Access-Control-Allow-Origin", acao))
        }
    } else {
        Some(HeaderCheck::ok("Access-Control-Allow-Origin", acao))
    }
}

fn extract_max_age(hsts: &str) -> Option<u64> {
    hsts.split(';')
        .find(|p| p.trim().to_lowercase().starts_with("max-age"))
        .and_then(|p| p.split('=').nth(1))
        .and_then(|v| v.trim().parse().ok())
}

/// Generate a security headers report
pub fn generate_report(checks: &[HeaderCheck]) -> String {
    let mut report = String::new();
    report.push_str("# Security Headers Report\n\n");

    let (critical, high, medium, low, info): (Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>) = {
        let mut c = Vec::new();
        let mut h = Vec::new();
        let mut m = Vec::new();
        let mut l = Vec::new();
        let mut i = Vec::new();

        for check in checks {
            match check.severity {
                Severity::Critical => c.push(check),
                Severity::High => h.push(check),
                Severity::Medium => m.push(check),
                Severity::Low => l.push(check),
                Severity::Info => i.push(check),
            }
        }

        (c, h, m, l, i)
    };

    report.push_str(&format!(
        "## Summary\n- Critical: {}\n- High: {}\n- Medium: {}\n- Low: {}\n- Info: {}\n\n",
        critical.len(),
        high.len(),
        medium.len(),
        low.len(),
        info.len()
    ));

    for (severity, items) in [
        ("Critical", critical),
        ("High", high),
        ("Medium", medium),
        ("Low", low),
    ] {
        if !items.is_empty() {
            report.push_str(&format!("## {} Severity\n\n", severity));
            for check in items {
                report.push_str(&format!("### {}\n", check.name));
                report.push_str(&format!("- Present: {}\n", check.present));
                if let Some(v) = &check.value {
                    report.push_str(&format!("- Value: `{}`\n", v));
                }
                report.push_str(&format!("- Issue: {}\n", check.description));
                report.push_str(&format!("- Fix: {}\n\n", check.recommendation));
            }
        }
    }

    report
}

/// Recommended security headers for a web application
pub fn recommended_headers() -> HashMap<&'static str, &'static str> {
    let mut headers = HashMap::new();
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains; preload",
    );
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'",
    );
    headers.insert("X-Content-Type-Options", "nosniff");
    headers.insert("X-Frame-Options", "DENY");
    headers.insert("Referrer-Policy", "strict-origin-when-cross-origin");
    headers.insert(
        "Permissions-Policy",
        "camera=(), microphone=(), geolocation=()",
    );
    headers.insert("Cache-Control", "no-store, no-cache, must-revalidate");
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_missing_headers() {
        let headers = HashMap::new();
        let results = analyze_headers(&headers);

        // Should have findings for missing security headers
        assert!(results
            .iter()
            .any(|r| r.name == "Strict-Transport-Security" && !r.present));
        assert!(results
            .iter()
            .any(|r| r.name == "Content-Security-Policy" && !r.present));
    }

    #[test]
    fn test_analyze_good_headers() {
        let mut headers = HashMap::new();
        headers.insert(
            "strict-transport-security".to_string(),
            "max-age=31536000".to_string(),
        );
        headers.insert(
            "content-security-policy".to_string(),
            "default-src 'self'".to_string(),
        );
        headers.insert("x-content-type-options".to_string(), "nosniff".to_string());
        headers.insert("x-frame-options".to_string(), "DENY".to_string());

        let results = analyze_headers(&headers);
        let high_severity: Vec<_> = results
            .iter()
            .filter(|r| r.severity == Severity::High || r.severity == Severity::Critical)
            .collect();

        assert!(high_severity.is_empty());
    }

    #[test]
    fn test_weak_csp() {
        let mut headers = HashMap::new();
        headers.insert(
            "content-security-policy".to_string(),
            "default-src 'self' 'unsafe-inline' 'unsafe-eval'".to_string(),
        );

        let results = analyze_headers(&headers);
        let csp_check = results
            .iter()
            .find(|r| r.name == "Content-Security-Policy")
            .unwrap();

        assert_eq!(csp_check.severity, Severity::Medium);
        assert!(csp_check.description.contains("unsafe-inline"));
    }

    #[test]
    fn test_cors_wildcard() {
        let mut headers = HashMap::new();
        headers.insert("access-control-allow-origin".to_string(), "*".to_string());

        let results = analyze_headers(&headers);
        let cors_check = results
            .iter()
            .find(|r| r.name == "Access-Control-Allow-Origin")
            .unwrap();

        assert_eq!(cors_check.severity, Severity::Medium);
    }

    #[test]
    fn test_extract_max_age() {
        assert_eq!(extract_max_age("max-age=31536000"), Some(31536000));
        assert_eq!(
            extract_max_age("max-age=31536000; includeSubDomains"),
            Some(31536000)
        );
        assert_eq!(
            extract_max_age("includeSubDomains; max-age=86400"),
            Some(86400)
        );
    }
}
