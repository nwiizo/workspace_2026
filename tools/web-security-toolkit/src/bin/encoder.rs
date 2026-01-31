//! Multi-format encoder/decoder CLI tool
//!
//! Usage:
//!   encoder encode z85 "text"
//!   encoder decode z85 "encoded"
//!   encoder encode base64 "text"
//!   encoder rot13 "text"
//!
//! Juice Shop coupon example:
//!   encoder encode z85 "JAN26-90"

use clap::{Parser, Subcommand};
use web_security_toolkit::encoding::*;

#[derive(Parser)]
#[command(name = "encoder")]
#[command(about = "Multi-format encoder/decoder for web security testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode data
    Encode {
        /// Encoding format (z85, base64, base64url, hex)
        format: String,
        /// Input text
        input: String,
    },
    /// Decode data
    Decode {
        /// Encoding format (z85, base64, hex)
        format: String,
        /// Encoded input
        input: String,
    },
    /// ROT13 transformation
    Rot13 {
        /// Input text
        input: String,
    },
    /// Generate Juice Shop coupon
    JuiceCoupon {
        /// Month (JAN, FEB, MAR, etc.)
        #[arg(default_value = "JAN")]
        month: String,
        /// Year (2 digits)
        #[arg(default_value = "26")]
        year: String,
        /// Discount percentage
        #[arg(default_value = "90")]
        discount: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode { format, input } => {
            let result = match format.to_lowercase().as_str() {
                "z85" => z85_encode(&input),
                "base64" => base64_encode(input.as_bytes()),
                "base64url" => base64url_encode(input.as_bytes()),
                "hex" => hex_encode(input.as_bytes()),
                _ => {
                    eprintln!(
                        "Unknown format: {}. Use z85, base64, base64url, or hex",
                        format
                    );
                    std::process::exit(1);
                }
            };
            println!("{}", result);
        }
        Commands::Decode { format, input } => {
            let result = match format.to_lowercase().as_str() {
                "z85" => z85_decode(&input).unwrap_or_else(|e| {
                    eprintln!("Decode error: {}", e);
                    std::process::exit(1);
                }),
                "base64" => {
                    let bytes = base64_decode(&input).unwrap_or_else(|e| {
                        eprintln!("Decode error: {}", e);
                        std::process::exit(1);
                    });
                    String::from_utf8_lossy(&bytes).to_string()
                }
                "hex" => {
                    let bytes = hex_decode(&input).unwrap_or_else(|e| {
                        eprintln!("Decode error: {}", e);
                        std::process::exit(1);
                    });
                    String::from_utf8_lossy(&bytes).to_string()
                }
                _ => {
                    eprintln!("Unknown format: {}. Use z85, base64, or hex", format);
                    std::process::exit(1);
                }
            };
            println!("{}", result);
        }
        Commands::Rot13 { input } => {
            println!("{}", rot13(&input));
        }
        Commands::JuiceCoupon {
            month,
            year,
            discount,
        } => {
            let coupon = format!("{}{}-{}", month.to_uppercase(), year, discount);
            let encoded = z85_encode(&coupon);
            println!("Coupon: {}", coupon);
            println!("Z85:    {}", encoded);
        }
    }
}
