//! Rectitude CLI - Scenario-based security testing tool

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rectitude")]
#[command(about = "Scenario-based security testing tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run scenario files
    Run {
        /// Path to scenario file or directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Run only failed scenarios from last run
        #[arg(long)]
        failed: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// List available scenarios
    List {
        /// Path to search for scenarios
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },

    /// Initialize configuration
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },

    /// Generate security payloads
    Payloads {
        #[command(subcommand)]
        category: PayloadCategory,
    },
}

#[derive(Subcommand)]
enum PayloadCategory {
    /// SQL Injection payloads
    Sqli {
        /// Payload type (auth-bypass, union, blind)
        #[arg(short = 't', long, default_value = "auth-bypass")]
        payload_type: String,
    },

    /// XSS payloads
    Xss {
        /// Payload type (basic, filter-bypass)
        #[arg(short = 't', long, default_value = "basic")]
        payload_type: String,
    },

    /// JWT manipulation
    Jwt {
        /// JWT to decode/manipulate
        token: Option<String>,

        /// Create unsigned JWT
        #[arg(long)]
        unsigned: bool,

        /// Payload for new JWT
        #[arg(long)]
        payload: Option<String>,
    },

    /// SSRF payloads
    Ssrf {
        /// Target port
        #[arg(short, long, default_value = "80")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            path,
            tag,
            failed: _,
            output,
        } => {
            run_scenarios(&path, tag.as_deref(), &output)?;
        }

        Commands::List { path, tag } => {
            list_scenarios(&path, tag.as_deref())?;
        }

        Commands::Init { force } => {
            init_config(force)?;
        }

        Commands::Payloads { category } => {
            handle_payloads(category)?;
        }
    }

    Ok(())
}

fn init_config(force: bool) -> anyhow::Result<()> {
    use rectitude::config::RectitudeConfig;
    use std::path::Path;

    let config_path = Path::new("rectitude.toml");

    if config_path.exists() && !force {
        eprintln!("rectitude.toml already exists. Use --force to overwrite.");
        std::process::exit(1);
    }

    std::fs::write(config_path, RectitudeConfig::template())?;
    println!("Created rectitude.toml");

    // Also create an examples directory if it doesn't exist
    let examples_dir = Path::new("examples");
    if !examples_dir.exists() {
        std::fs::create_dir(examples_dir)?;
        println!("Created examples/ directory");
    }

    Ok(())
}

fn list_scenarios(path: &PathBuf, tag: Option<&str>) -> anyhow::Result<()> {
    let examples_dir = path.join("examples");
    let search_path = if examples_dir.exists() {
        &examples_dir
    } else {
        path
    };

    println!("Scenarios in {:?}:\n", search_path);

    let mut count = 0;
    let entries: Vec<_> = std::fs::read_dir(search_path)?
        .filter_map(|e| e.ok())
        .collect();

    for entry in entries {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            let name = path.file_stem().unwrap().to_string_lossy();

            // If tag filter is specified, check file contents (basic heuristic)
            if let Some(t) = tag {
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                if !content.contains(&format!(".tag(\"{}\")", t))
                    && !content.contains(&format!("\"{}\"", t))
                {
                    continue;
                }
            }

            println!("  {}", name);
            count += 1;
        }
    }

    if count == 0 {
        println!("  (no scenarios found)");
    } else {
        println!("\nTotal: {} scenario file(s)", count);
    }

    println!("\nRun with: cargo run --example <scenario_name>");

    Ok(())
}

fn run_scenarios(path: &PathBuf, tag: Option<&str>, output: &str) -> anyhow::Result<()> {
    let config = rectitude::config::RectitudeConfig::load().unwrap_or_default();

    if output == "json" {
        // JSON output mode
        let result = serde_json::json!({
            "status": "info",
            "message": "Use library API to run scenarios programmatically",
            "config": {
                "base_url": config.base_url,
                "timeout": config.timeout_or_default(),
                "include_tags": config.include_tags,
                "exclude_tags": config.exclude_tags,
            },
            "hint": "See examples/ directory for scenario examples"
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        // Text output mode
        println!("=== Rectitude Scenario Runner ===\n");

        if let Some(base_url) = &config.base_url {
            println!("Base URL: {}", base_url);
        }
        println!("Timeout: {}s", config.timeout_or_default());

        if let Some(t) = tag {
            println!("Filter tag: {}", t);
        }

        println!();

        // List available examples
        let examples_dir = path.join("examples");
        let search_path = if examples_dir.exists() {
            &examples_dir
        } else {
            path
        };

        if search_path.exists() {
            println!("Available scenarios:");
            for entry in std::fs::read_dir(search_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "rs") {
                    let name = path.file_stem().unwrap().to_string_lossy();
                    println!("  cargo run --example {}", name);
                }
            }
        }

        println!("\n[INFO] Rectitude uses Rust code for scenario definitions.");
        println!("Run scenarios with: cargo run --example <scenario_name>");
    }

    Ok(())
}

fn handle_payloads(category: PayloadCategory) -> anyhow::Result<()> {
    match category {
        PayloadCategory::Sqli { payload_type } => {
            use rectitude::payloads::sqli;

            println!("=== SQLi Payloads ({}) ===\n", payload_type);

            match payload_type.as_str() {
                "auth-bypass" => {
                    for p in sqli::auth_bypass_payloads() {
                        println!("{}: {}", p.name, p.payload);
                    }
                }
                "union" => {
                    for p in sqli::union_column_discovery(9) {
                        println!("{}: {}", p.name, p.payload);
                    }
                }
                _ => println!("Unknown payload type: {}", payload_type),
            }
        }

        PayloadCategory::Xss { payload_type } => {
            use rectitude::payloads::xss;

            println!("=== XSS Payloads ({}) ===\n", payload_type);

            let payloads = match payload_type.as_str() {
                "basic" => xss::basic_payloads(),
                "filter-bypass" => xss::filter_bypass_payloads(),
                _ => {
                    println!("Unknown payload type: {}", payload_type);
                    return Ok(());
                }
            };

            for p in payloads {
                println!("{}: {}", p.name, p.payload);
            }
        }

        PayloadCategory::Jwt {
            token,
            unsigned,
            payload,
        } => {
            use rectitude::payloads::jwt;

            if let Some(t) = token {
                println!("=== JWT Decoded ===\n");
                match jwt::DecodedJwt::decode(&t) {
                    Ok(decoded) => {
                        println!("Header: {}", serde_json::to_string_pretty(&decoded.header)?);
                        println!(
                            "\nPayload: {}",
                            serde_json::to_string_pretty(&decoded.payload)?
                        );
                    }
                    Err(e) => println!("Error: {}", e),
                }
            } else if unsigned {
                let payload_value = if let Some(p) = payload {
                    serde_json::from_str(&p)?
                } else {
                    serde_json::json!({"role": "admin"})
                };
                let token = jwt::create_unsigned(&payload_value);
                println!("=== Unsigned JWT ===\n");
                println!("{}", token);
            } else {
                println!("Usage: rectitude payloads jwt <TOKEN>");
                println!(
                    r#"       rectitude payloads jwt --unsigned [--payload '{{"role":"admin"}}']"#
                );
            }
        }

        PayloadCategory::Ssrf { port } => {
            use rectitude::payloads::ssrf;

            println!("=== SSRF Localhost Variants (port {}) ===\n", port);
            for p in ssrf::localhost_variants(port) {
                println!("{}: {}", p.name, p.url);
            }

            println!("\n=== Cloud Metadata Endpoints ===\n");
            for p in ssrf::cloud_metadata_endpoints() {
                println!("{}: {}", p.name, p.url);
            }
        }
    }

    Ok(())
}
