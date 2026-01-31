//! Brute force utilities CLI
//!
//! Usage:
//!   bruteforce-gen pins
//!   bruteforce-gen numeric 4 0 9999
//!   bruteforce-gen rate-limit
//!   bruteforce-gen security-question pet
//!   bruteforce-gen ip-rotation 100
//!   bruteforce-gen token-patterns

use clap::{Parser, Subcommand};
use web_security_toolkit::bruteforce::*;

#[derive(Parser)]
#[command(name = "bruteforce-gen")]
#[command(about = "Brute force utilities for security testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate common PIN patterns
    Pins {
        /// Output as list only
        #[arg(short, long)]
        list: bool,
    },
    /// Generate numeric sequences
    Numeric {
        /// Number of digits
        digits: usize,
        /// Start value
        start: u64,
        /// End value
        end: u64,
        /// Output as list only
        #[arg(short, long)]
        list: bool,
    },
    /// Rate limit bypass techniques
    RateLimit,
    /// Generate IP addresses for X-Forwarded-For rotation
    IpRotation {
        /// Number of IPs to generate
        #[arg(default_value = "100")]
        count: usize,
        /// Output as list only
        #[arg(short, long)]
        list: bool,
    },
    /// Security question answer wordlist
    SecurityQuestion {
        /// Question type (pet, city, mother, school, company, sibling)
        question_type: String,
        /// Output as list only
        #[arg(short, long)]
        list: bool,
    },
    /// Username enumeration indicators
    Enumeration,
    /// Password reset token patterns
    TokenPatterns,
    /// Generate alphanumeric combinations
    Alphanumeric {
        /// Length of combinations
        length: usize,
        /// Character set to use [default: 0-9a-z]
        #[arg(short, long, default_value = "0123456789abcdefghijklmnopqrstuvwxyz")]
        charset: String,
        /// Maximum number to generate (combinations can be huge)
        #[arg(short, long, default_value = "1000")]
        max: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pins { list } => {
            let pins = common_pins();

            if list {
                for pin in pins {
                    println!("{}", pin);
                }
            } else {
                println!("=== Common PIN Patterns ===\n");
                println!("Count: {} patterns\n", pins.len());

                println!("Sample:");
                for pin in pins.iter().take(30) {
                    println!("  {}", pin);
                }

                if pins.len() > 30 {
                    println!("  ... and {} more", pins.len() - 30);
                }

                println!("\nUse --list for output suitable for tools");
            }
        }
        Commands::Numeric {
            digits,
            start,
            end,
            list,
        } => {
            if end < start {
                eprintln!("Error: end must be >= start");
                std::process::exit(1);
            }

            let count = end - start + 1;
            if count > 1_000_000 && !list {
                eprintln!(
                    "Warning: {} combinations. Use --list for large outputs.",
                    count
                );
                std::process::exit(1);
            }

            let codes = numeric_sequence(digits, start, end);

            if list {
                for code in codes {
                    println!("{}", code);
                }
            } else {
                println!(
                    "=== Numeric Sequence ({} digits, {}-{}) ===\n",
                    digits, start, end
                );
                println!("Count: {}\n", codes.len());

                for code in codes.iter().take(20) {
                    println!("  {}", code);
                }

                if codes.len() > 20 {
                    println!("  ...");
                    println!("  {}", codes.last().unwrap());
                }

                println!("\nUse --list for full output");
            }
        }
        Commands::RateLimit => {
            println!("=== Rate Limit Bypass Techniques ===\n");

            for bypass in rate_limit_bypasses() {
                println!("{}", bypass.name);
                println!("  Technique: {}", bypass.technique);

                if !bypass.headers.is_empty() {
                    println!("  Headers:");
                    for (key, value) in &bypass.headers {
                        println!("    {}: {}", key, value);
                    }
                }
                println!();
            }

            println!("Additional techniques:");
            println!("  - Add ?cachebuster=random to URL");
            println!("  - Use different HTTP methods (GET vs POST)");
            println!("  - Change User-Agent header");
            println!("  - Use IPv6 if available");
        }
        Commands::IpRotation { count, list } => {
            let ips = generate_ip_rotation(count);

            if list {
                for ip in ips {
                    println!("{}", ip);
                }
            } else {
                println!("=== IP Rotation Addresses ===\n");
                println!("Count: {}\n", ips.len());

                println!("Sample:");
                for ip in ips.iter().take(10) {
                    println!("  {}", ip);
                }

                println!("\nUsage with X-Forwarded-For:");
                println!("  curl -H 'X-Forwarded-For: {}' ...", ips[0]);

                println!("\nUse --list for full output");
            }
        }
        Commands::SecurityQuestion {
            question_type,
            list,
        } => {
            let answers = security_question_wordlist(&question_type);

            if answers.is_empty() {
                eprintln!("Unknown question type: {}", question_type);
                eprintln!("\nSupported types:");
                eprintln!("  pet, city, mother, school, company, sibling");
                std::process::exit(1);
            }

            if list {
                for answer in answers {
                    println!("{}", answer);
                }
            } else {
                println!(
                    "=== Security Question Answers: {} ===\n",
                    question_type.to_uppercase()
                );
                println!("Count: {}\n", answers.len());

                for answer in &answers {
                    println!("  {}", answer);
                }

                println!("\nUse --list for output suitable for tools");
            }
        }
        Commands::Enumeration => {
            println!("=== Username Enumeration Indicators ===\n");

            for indicator in username_enumeration_indicators() {
                println!("{:?}", indicator.indicator_type);
                println!("  {}\n", indicator.description);
            }

            println!("Testing methodology:");
            println!("  1. Try login with known valid username + wrong password");
            println!("  2. Try login with invalid username + wrong password");
            println!("  3. Compare response time, message, status code, size");
        }
        Commands::TokenPatterns => {
            println!("=== Password Reset Token Patterns ===\n");

            for pattern in reset_token_patterns() {
                println!("{}", pattern.name);
                println!("  {}", pattern.description);
                println!("  Example: {}\n", pattern.example);
            }

            println!("Attack methodology:");
            println!("  1. Request multiple reset tokens");
            println!("  2. Analyze for patterns or predictability");
            println!("  3. Try to predict the next token");
        }
        Commands::Alphanumeric {
            length,
            charset,
            max,
        } => {
            let total = charset.len().pow(length as u32);

            if total > max {
                println!("=== Alphanumeric Combinations ===\n");
                println!("Total combinations: {} (limited to {})", total, max);
                println!("Charset: {} ({} chars)", charset, charset.len());
                println!("Length: {}\n", length);

                // Generate only up to max
                let combinations = alphanumeric_combinations(length, &charset);
                for combo in combinations.iter().take(max) {
                    println!("{}", combo);
                }

                if total > max {
                    println!("\n... truncated at {} combinations", max);
                }
            } else {
                let combinations = alphanumeric_combinations(length, &charset);
                for combo in combinations {
                    println!("{}", combo);
                }
            }
        }
    }
}
