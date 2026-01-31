//! Hashids Encoder/Decoder CLI
//!
//! Usage:
//!   hashids-tool encode <numbers> --salt <salt>
//!   hashids-tool decode <hashid> --salt <salt>
//!   hashids-tool discover <hashid>
//!   hashids-tool juice-shop [options]
//!
//! Example:
//!   hashids-tool encode 1,2,3 --salt "my secret"
//!   hashids-tool decode "abc123" --salt "my secret"
//!   hashids-tool discover "someHashid"
//!   hashids-tool juice-shop --imaginary

use clap::{Parser, Subcommand};
use web_security_toolkit::hashids::{
    common_salts, decode_continue_code, decode_hashid, discover_salt, encode_hashid,
    generate_continue_code, generate_imaginary_challenge_codes, juice_shop_salts,
};

#[derive(Parser)]
#[command(name = "hashids-tool")]
#[command(about = "Hashids Encoder/Decoder for CTF")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode numbers into a Hashid
    Encode {
        /// Comma-separated numbers (e.g., "1,2,3")
        numbers: String,

        /// Salt for encoding
        #[arg(short, long, default_value = "")]
        salt: String,

        /// Minimum length of output
        #[arg(short, long, default_value = "0")]
        min_length: usize,
    },

    /// Decode a Hashid into numbers
    Decode {
        /// The Hashid to decode
        hashid: String,

        /// Salt for decoding
        #[arg(short, long, default_value = "")]
        salt: String,
    },

    /// Try to discover the salt used for a Hashid
    Discover {
        /// The Hashid to analyze
        hashid: String,

        /// Expected numbers for validation (comma-separated)
        #[arg(short, long)]
        expected: Option<String>,
    },

    /// Juice Shop specific tools
    JuiceShop {
        /// Generate imaginary challenge codes
        #[arg(long)]
        imaginary: bool,

        /// Decode a continue code
        #[arg(long)]
        decode: Option<String>,

        /// Try all known salts for a code
        #[arg(long)]
        discover: Option<String>,

        /// Generate continue code for challenge IDs
        #[arg(long)]
        encode: Option<String>,

        /// Salt to use (default: tries known salts)
        #[arg(short, long)]
        salt: Option<String>,
    },

    /// List known salts
    Salts {
        /// Include common salts (not just Juice Shop specific)
        #[arg(short, long)]
        all: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode {
            numbers,
            salt,
            min_length,
        } => {
            let nums: Result<Vec<u64>, _> = numbers.split(',').map(|s| s.trim().parse()).collect();

            match nums {
                Ok(n) => {
                    let encoded = encode_hashid(&n, &salt, min_length);
                    println!("Encoded: {}", encoded);
                }
                Err(e) => {
                    eprintln!("Error parsing numbers: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Decode { hashid, salt } => match decode_hashid(&hashid, &salt) {
            Some(numbers) => {
                let nums_str: Vec<String> = numbers.iter().map(|n| n.to_string()).collect();
                println!("Decoded: {}", nums_str.join(", "));
            }
            None => {
                eprintln!("Failed to decode (wrong salt or invalid hashid)");
                std::process::exit(1);
            }
        },

        Commands::Discover { hashid, expected } => {
            let expected_nums: Option<Vec<u64>> =
                expected.map(|e| e.split(',').filter_map(|s| s.trim().parse().ok()).collect());

            println!("Trying to discover salt for: {}", hashid);
            println!();

            match discover_salt(&hashid, expected_nums.as_deref()) {
                Some(salt) => {
                    println!("✅ Found salt: \"{}\"", salt);
                    if let Some(decoded) = decode_hashid(&hashid, &salt) {
                        let nums_str: Vec<String> = decoded.iter().map(|n| n.to_string()).collect();
                        println!("   Decoded values: {}", nums_str.join(", "));
                    }
                }
                None => {
                    println!("❌ Could not discover salt.");
                    println!(
                        "   Tried {} salts",
                        juice_shop_salts().len() + common_salts().len()
                    );
                }
            }
        }

        Commands::JuiceShop {
            imaginary,
            decode,
            discover,
            encode,
            salt,
        } => {
            if imaginary {
                println!("Generating imaginary challenge codes...\n");
                let codes = generate_imaginary_challenge_codes();
                for (s, code, desc) in codes {
                    println!("Salt: \"{}\" | {}", s, desc);
                    println!("Code: {}", code);
                    println!();
                }
                println!("Try submitting these codes at: /#/score-board");
                println!("\nTo submit, open browser console and run:");
                println!("  fetch('/rest/continue-code/apply/CODE_HERE').then(r => r.json()).then(console.log)");
                return;
            }

            if let Some(code) = decode {
                let salts_to_try: Vec<&str> = if let Some(ref s) = salt {
                    vec![s.as_str()]
                } else {
                    juice_shop_salts()
                };

                println!("Decoding continue code: {}\n", code);

                for s in salts_to_try {
                    if let Some(ids) = decode_continue_code(&code, s) {
                        let ids_str: Vec<String> = ids.iter().map(|n| n.to_string()).collect();
                        println!("✅ Salt: \"{}\"", s);
                        println!("   Challenge IDs: {}", ids_str.join(", "));
                        println!("   Count: {} challenges", ids.len());
                        return;
                    }
                }
                println!("❌ Could not decode with known salts.");
                return;
            }

            if let Some(code) = discover {
                println!("Discovering salt for continue code...\n");

                for s in juice_shop_salts() {
                    if let Some(ids) = decode_continue_code(&code, s) {
                        if !ids.is_empty() && ids.iter().all(|&id| id > 0 && id < 500) {
                            let ids_str: Vec<String> = ids.iter().map(|n| n.to_string()).collect();
                            println!("✅ Possible salt: \"{}\"", s);
                            println!("   Decoded IDs: {}", ids_str.join(", "));
                        }
                    }
                }
                return;
            }

            if let Some(ids_str) = encode {
                let ids: Vec<u64> = ids_str
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();

                let s = salt.as_deref().unwrap_or("this is my salt");
                let code = generate_continue_code(&ids, s);

                println!("Generated continue code:");
                println!("  Salt: \"{}\"", s);
                println!("  IDs: {:?}", ids);
                println!("  Code: {}", code);
                return;
            }

            // Default: show help for Juice Shop mode
            println!("Juice Shop Hashids Tools\n");
            println!("Options:");
            println!("  --imaginary     Generate codes for imaginary challenge");
            println!("  --decode CODE   Decode a continue code");
            println!("  --discover CODE Try to find the salt for a code");
            println!("  --encode IDS    Generate a continue code for IDs");
            println!("  --salt SALT     Specify salt to use");
        }

        Commands::Salts { all } => {
            println!("Known Juice Shop salts:");
            for s in juice_shop_salts() {
                println!("  \"{}\"", s);
            }

            if all {
                println!("\nCommon salts:");
                for s in common_salts() {
                    println!("  \"{}\"", s);
                }
            }
        }
    }
}
