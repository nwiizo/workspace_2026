//! Z85 クーポン生成ツール
//! 
//! Usage: cargo run --bin z85-coupon [COUPON]
//! Example: cargo run --bin z85-coupon "JAN26-90"

use std::env;

/// クーポンを Z85 エンコードする
pub fn encode_coupon(coupon: &str) -> String {
    // Z85 は 4バイトの倍数が必要
    let padded_len = ((coupon.len() + 3) / 4) * 4;
    let mut padded = coupon.as_bytes().to_vec();
    padded.resize(padded_len, 0);
    
    z85::encode(&padded)
}

/// Z85 エンコードされたクーポンをデコードする
pub fn decode_coupon(encoded: &str) -> Option<String> {
    z85::decode(encoded)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).trim_end_matches('\0').to_string())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let coupon = args.get(1).map(|s| s.as_str()).unwrap_or("JAN26-90");
    
    println!("=== Juice Shop Coupon Generator ===\n");
    println!("Input coupon: {}", coupon);
    
    let encoded = encode_coupon(coupon);
    println!("Z85 encoded: {}", encoded);
    
    // デコードして確認
    if let Some(decoded) = decode_coupon(&encoded) {
        println!("Decoded back: {}", decoded);
    }
    
    println!("\n=== Sample coupons ===");
    let samples = vec![
        "JAN26-90",
        "FEB26-80",
        "DEC25-99",
        "MAR26-50",
    ];
    
    for sample in samples {
        let encoded = encode_coupon(sample);
        println!("{:12} → {}", sample, encoded);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let coupons = vec![
            "JAN26-90",
            "FEB26-80",
            "DEC25-99",
            "OCT13-10",
        ];
        
        for coupon in coupons {
            let encoded = encode_coupon(coupon);
            let decoded = decode_coupon(&encoded).unwrap();
            assert_eq!(decoded, coupon, "Roundtrip failed for {}", coupon);
        }
    }

    #[test]
    fn test_coupon_format() {
        // クーポン形式: MMMYY-VV
        let coupon = "JAN26-90";
        let parts: Vec<&str> = coupon.split('-').collect();
        
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 5); // MMMYY
        assert!(parts[1].parse::<u32>().is_ok()); // VV は数字
    }

    #[test]
    fn test_encoding_produces_valid_z85() {
        let encoded = encode_coupon("JAN26-90");
        
        // Z85 は印刷可能文字のみ
        assert!(encoded.chars().all(|c| c.is_ascii_graphic()));
        
        // デコード可能であること
        assert!(decode_coupon(&encoded).is_some());
    }

    #[test]
    fn test_high_discount_coupon() {
        let coupon = "DEC26-99"; // 99% オフ
        let encoded = encode_coupon(coupon);
        let decoded = decode_coupon(&encoded).unwrap();
        
        assert_eq!(decoded, coupon);
        
        // 割引率を確認
        let discount: u32 = coupon.split('-').nth(1).unwrap().parse().unwrap();
        assert!(discount >= 80, "Discount should be 80% or more for the challenge");
    }
}
