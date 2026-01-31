//! TOTP/2FA CLI tool
//!
//! Usage:
//!   totp-tool generate SECRET
//!   totp-tool window SECRET --size 2
//!   totp-tool analyze SECRET
//!   totp-tool bypasses
//!   totp-tool brute-force
//!   totp-tool juice-shop

use clap::{Parser, Subcommand};
use web_security_toolkit::totp::*;

#[derive(Parser)]
#[command(name = "totp-tool")]
#[command(about = "TOTP/2FA utility for security testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate TOTP code from secret
    Generate {
        /// Base32-encoded TOTP secret
        secret: String,
        /// Time offset in seconds (for testing timing)
        #[arg(short, long, default_value = "0")]
        offset: i64,
    },
    /// Generate TOTP codes for a time window
    Window {
        /// Base32-encoded TOTP secret
        secret: String,
        /// Window size (number of 30-second intervals before/after)
        #[arg(short, long, default_value = "2")]
        size: i64,
    },
    /// Analyze TOTP secret format
    Analyze {
        /// TOTP secret to analyze
        secret: String,
    },
    /// List 2FA bypass techniques
    Bypasses,
    /// Generate brute-force codes (common codes)
    BruteForce {
        /// Output as list only (one per line)
        #[arg(short, long)]
        list: bool,
    },
    /// Juice Shop 2FA challenge helpers
    JuiceShop,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { secret, offset } => {
            let code = generate_totp(&secret, offset);
            println!("TOTP Code: {}", code);

            if offset != 0 {
                println!("(offset: {} seconds)", offset);
            }

            println!("\nNote: Code is valid for ~30 seconds");
        }
        Commands::Window { secret, size } => {
            println!("=== TOTP Codes (window: +/-{} intervals) ===\n", size);

            let codes = generate_totp_window(&secret, size);

            for (offset, code) in codes {
                let marker = if offset == 0 { " <-- current" } else { "" };
                println!("  {:+2}: {}{}", offset, code, marker);
            }

            println!("\nEach interval is 30 seconds");
        }
        Commands::Analyze { secret } => {
            let analysis = analyze_secret(&secret);

            println!("=== Secret Analysis ===\n");
            println!("Original:       {}", analysis.original);
            println!("Normalized:     {}", analysis.normalized);
            println!("Length:         {} characters", analysis.length);
            println!("Valid Base32:   {}", analysis.is_valid_base32);

            if let Some(len) = analysis.decoded_length {
                println!("Decoded length: {} bytes", len);

                // Check for standard key lengths
                let strength = match len {
                    10 => "80-bit (minimum, weak)",
                    20 => "160-bit (SHA1 standard)",
                    32 => "256-bit (strong)",
                    _ => "non-standard",
                };
                println!("Key strength:   {}", strength);
            } else {
                println!("Decoded length: Invalid (decode failed)");
            }

            // Try to generate a code
            if analysis.is_valid_base32 {
                let code = generate_totp(&analysis.normalized, 0);
                println!("\nCurrent code:   {}", code);
            }
        }
        Commands::Bypasses => {
            println!("=== 2FA Bypass Techniques ===\n");

            for bypass in two_factor_bypasses() {
                println!("{}", bypass.name);
                println!("  {}", bypass.description);
                println!("  Type: {:?}\n", bypass.technique);
            }
        }
        Commands::BruteForce { list } => {
            let codes = brute_force_codes();

            if list {
                for code in codes {
                    println!("{}", code);
                }
            } else {
                println!("=== Common TOTP Codes for Brute Force ===\n");
                println!("Count: {} codes\n", codes.len());

                for (i, code) in codes.iter().enumerate() {
                    if i < 20 {
                        println!("  {}", code);
                    }
                }

                if codes.len() > 20 {
                    println!("  ... and {} more", codes.len() - 20);
                }

                println!("\nUse --list to output all codes (one per line)");
            }
        }
        Commands::JuiceShop => {
            let info = juice_shop_2fa();

            println!("=== Juice Shop 2FA Challenge ===\n");
            println!("{}\n", info.description);

            println!("SQLi Payload:");
            println!("  {}\n", info.sqli_payload);

            println!("Steps:");
            for step in &info.steps {
                println!("  {}", step);
            }

            println!("\nExample workflow:");
            println!("  1. Use SQLi to get TOTP secret from database");
            println!("  2. Run: totp-tool generate <extracted_secret>");
            println!("  3. Use the generated code to login");
        }
    }
}
