//! General payload generator CLI
//!
//! Usage:
//!   payload-gen sqli auth-bypass
//!   payload-gen sqli union 9
//!   payload-gen sqli juice-shop
//!   payload-gen xss basic
//!   payload-gen xss bypass
//!   payload-gen xxe file /etc/passwd
//!   payload-gen nosql auth-bypass
//!   payload-gen traversal 5 etc/passwd
//!   payload-gen passwords top
//!   payload-gen passwords juice-shop
//!   payload-gen idor endpoints
//!   payload-gen tampering negative quantity 1
//!   payload-gen tampering mass-assignment
//!   payload-gen tampering juice-shop

use clap::{Parser, Subcommand};
use serde_json::json;
use web_security_toolkit::idor::*;
use web_security_toolkit::nosql::*;
use web_security_toolkit::param_tampering::*;
use web_security_toolkit::passwords::*;
use web_security_toolkit::sqli::*;
use web_security_toolkit::traversal::*;
use web_security_toolkit::xss::*;
use web_security_toolkit::xxe::*;

#[derive(Parser)]
#[command(name = "payload-gen")]
#[command(about = "Security payload generator for web testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// SQL injection payloads
    Sqli {
        #[command(subcommand)]
        subcommand: SqliCommands,
    },
    /// XSS payloads
    Xss {
        #[command(subcommand)]
        subcommand: XssCommands,
    },
    /// XXE payloads
    Xxe {
        #[command(subcommand)]
        subcommand: XxeCommands,
    },
    /// NoSQL injection payloads
    Nosql {
        #[command(subcommand)]
        subcommand: NosqlCommands,
    },
    /// Path traversal payloads
    Traversal {
        /// Traversal depth
        #[arg(default_value = "5")]
        depth: usize,
        /// Target file
        #[arg(default_value = "etc/passwd")]
        target: String,
    },
    /// Password utilities
    Passwords {
        #[command(subcommand)]
        subcommand: PasswordCommands,
    },
    /// IDOR testing utilities
    Idor {
        #[command(subcommand)]
        subcommand: IdorCommands,
    },
    /// Parameter tampering payloads
    Tampering {
        #[command(subcommand)]
        subcommand: TamperingCommands,
    },
}

#[derive(Subcommand)]
enum SqliCommands {
    /// Authentication bypass payloads
    AuthBypass,
    /// UNION-based column discovery
    Union {
        /// Number of columns
        #[arg(default_value = "9")]
        columns: usize,
    },
    /// User login bypass for specific email
    Login {
        /// Email address
        email: String,
    },
    /// SQLite specific payloads
    Sqlite,
    /// MySQL specific payloads
    Mysql,
    /// PostgreSQL specific payloads
    Postgresql,
    /// Juice Shop SQLi payloads
    JuiceShop,
}

#[derive(Subcommand)]
enum XssCommands {
    /// Basic XSS payloads
    Basic,
    /// Filter bypass payloads
    Bypass,
    /// DOM-based XSS payloads
    Dom,
    /// Polyglot payloads
    Polyglot,
    /// URL encode a payload
    Encode {
        /// Payload to encode
        payload: String,
    },
    /// Juice Shop XSS payloads
    JuiceShop,
}

#[derive(Subcommand)]
enum XxeCommands {
    /// File read payload
    File {
        /// Target file path
        #[arg(default_value = "/etc/passwd")]
        path: String,
    },
    /// SSRF payload
    Ssrf {
        /// Target URL
        url: String,
    },
    /// Billion Laughs DoS
    Dos,
    /// Out-of-band exfiltration
    Oob {
        /// Attacker URL
        url: String,
        /// Target file
        #[arg(default_value = "/etc/passwd")]
        file: String,
    },
    /// Cloud metadata payloads
    Cloud,
    /// Juice Shop XXE payloads
    JuiceShop,
}

#[derive(Subcommand)]
enum NosqlCommands {
    /// Authentication bypass payloads
    AuthBypass,
    /// Data exfiltration payloads
    Exfil,
    /// Blind regex extraction
    Blind {
        /// Field to extract
        field: String,
        /// Known prefix
        #[arg(default_value = "")]
        prefix: String,
    },
    /// Juice Shop NoSQL payloads
    JuiceShop,
}

#[derive(Subcommand)]
enum PasswordCommands {
    /// Top common passwords
    Top,
    /// Identify hash type
    Identify {
        /// Hash to identify
        hash: String,
    },
    /// Generate password variations
    Variations {
        /// Base word
        word: String,
    },
    /// Juice Shop credentials
    JuiceShop,
}

#[derive(Subcommand)]
enum IdorCommands {
    /// Common IDOR endpoints
    Endpoints,
    /// Generate ID variations
    Ids {
        /// Current ID
        current: i64,
        /// Range to test
        #[arg(default_value = "10")]
        range: usize,
    },
    /// Juice Shop IDOR endpoints
    JuiceShop,
}

#[derive(Subcommand)]
enum TamperingCommands {
    /// Negative value tests
    Negative {
        /// Field name
        field: String,
        /// Original value
        #[arg(default_value = "1")]
        value: i64,
    },
    /// Mass assignment payloads
    MassAssignment,
    /// Privilege escalation tests
    Privilege {
        /// Target user ID
        #[arg(default_value = "1")]
        target_id: i64,
    },
    /// Juice Shop tampering payloads
    JuiceShop,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sqli { subcommand } => handle_sqli(subcommand),
        Commands::Xss { subcommand } => handle_xss(subcommand),
        Commands::Xxe { subcommand } => handle_xxe(subcommand),
        Commands::Nosql { subcommand } => handle_nosql(subcommand),
        Commands::Traversal { depth, target } => handle_traversal(depth, &target),
        Commands::Passwords { subcommand } => handle_passwords(subcommand),
        Commands::Idor { subcommand } => handle_idor(subcommand),
        Commands::Tampering { subcommand } => handle_tampering(subcommand),
    }
}

fn handle_sqli(cmd: SqliCommands) {
    match cmd {
        SqliCommands::AuthBypass => {
            println!("=== SQLi Authentication Bypass ===\n");
            for p in auth_bypass_payloads() {
                println!("{:20} → {}", p.name, p.payload);
            }
        }
        SqliCommands::Union { columns } => {
            println!("=== UNION Column Discovery (1-{}) ===\n", columns);
            for p in union_column_discovery(columns) {
                println!("{}", p.payload);
            }
        }
        SqliCommands::Login { email } => {
            let payload = user_login_bypass(&email);
            println!("Login bypass for {}: {}", email, payload);
        }
        SqliCommands::Sqlite => {
            println!("=== SQLite Payloads ===\n");
            for p in sqlite_payloads() {
                println!("{}\n  {}\n", p.name, p.payload);
            }
        }
        SqliCommands::Mysql => {
            println!("=== MySQL Payloads ===\n");
            for p in mysql_payloads() {
                println!("{}\n  {}\n", p.name, p.payload);
            }
        }
        SqliCommands::Postgresql => {
            println!("=== PostgreSQL Payloads ===\n");
            for p in postgresql_payloads() {
                println!("{}\n  {}\n", p.name, p.payload);
            }
        }
        SqliCommands::JuiceShop => {
            println!("=== Juice Shop SQLi Payloads ===\n");
            for p in juice_shop_sqli() {
                println!("{}\n  {}\n", p.name, p.payload);
            }
        }
    }
}

fn handle_xss(cmd: XssCommands) {
    match cmd {
        XssCommands::Basic => {
            println!("=== Basic XSS Payloads ===\n");
            for p in basic_payloads() {
                println!("{}\n  {}\n", p.name, p.payload);
            }
        }
        XssCommands::Bypass => {
            println!("=== Filter Bypass Payloads ===\n");
            for p in filter_bypass_payloads() {
                println!("{}\n  {}\n", p.name, p.payload);
            }
        }
        XssCommands::Dom => {
            println!("=== DOM-based XSS Payloads ===\n");
            for p in dom_based_payloads() {
                println!("{}\n  {}\n", p.name, p.payload);
            }
        }
        XssCommands::Polyglot => {
            println!("=== Polyglot XSS Payloads ===\n");
            for p in polyglot_payloads() {
                println!("{}\n  {}\n", p.name, p.payload);
            }
        }
        XssCommands::Encode { payload } => {
            println!("Original: {}", payload);
            println!("URL encoded: {}", url_encode_xss(&payload));
            println!("HTML entities: {}", html_entity_encode(&payload));
        }
        XssCommands::JuiceShop => {
            println!("=== Juice Shop XSS Payloads ===\n");
            for p in juice_shop_xss() {
                println!("{}\n  {}\n", p.name, p.payload);
            }
        }
    }
}

fn handle_xxe(cmd: XxeCommands) {
    match cmd {
        XxeCommands::File { path } => {
            println!("=== XXE File Read ===\n");
            println!("{}", file_read_xxe(&path));
        }
        XxeCommands::Ssrf { url } => {
            println!("=== XXE SSRF ===\n");
            println!("{}", ssrf_xxe(&url));
        }
        XxeCommands::Dos => {
            println!("=== Billion Laughs DoS ===\n");
            println!("{}", billion_laughs_xxe());
        }
        XxeCommands::Oob { url, file } => {
            println!("=== Out-of-Band XXE ===\n");
            println!("Payload:\n{}\n", oob_xxe(&url, &file));
            println!("DTD file content:\n{}", oob_dtd(&url));
        }
        XxeCommands::Cloud => {
            println!("=== Cloud Metadata XXE ===\n");
            for p in cloud_metadata_xxe() {
                println!("{}\n{}\n", p.name, p.payload);
            }
        }
        XxeCommands::JuiceShop => {
            println!("=== Juice Shop XXE Payloads ===\n");
            for p in juice_shop_xxe() {
                println!("{}\n{}\n", p.name, p.payload);
            }
        }
    }
}

fn handle_nosql(cmd: NosqlCommands) {
    match cmd {
        NosqlCommands::AuthBypass => {
            println!("=== NoSQL Auth Bypass ===\n");
            for p in mongo_auth_bypass() {
                println!("{}\n  {}\n", p.name, p.payload_string);
            }
            println!("\nURL parameter format:");
            for param in url_params_auth_bypass() {
                println!("  {}", param);
            }
        }
        NosqlCommands::Exfil => {
            println!("=== NoSQL Data Exfiltration ===\n");
            for p in mongo_data_exfil() {
                println!("{}\n  {}\n", p.name, p.payload_string);
            }
        }
        NosqlCommands::Blind { field, prefix } => {
            println!("=== Blind NoSQL Extraction ===\n");
            println!("Field: {}, Prefix: {}\n", field, prefix);
            let charset = "abcdefghijklmnopqrstuvwxyz0123456789";
            for p in blind_char_extraction(&field, &prefix, charset)
                .iter()
                .take(10)
            {
                println!("{}", p.payload_string);
            }
            println!("... ({} total)", charset.len());
        }
        NosqlCommands::JuiceShop => {
            println!("=== Juice Shop NoSQL Payloads ===\n");
            for p in juice_shop_nosql() {
                println!("{}\n  {}\n", p.name, p.payload_string);
            }
        }
    }
}

fn handle_traversal(depth: usize, target: &str) {
    println!(
        "=== Path Traversal (depth: {}, target: {}) ===\n",
        depth, target
    );

    println!("Basic:");
    for p in basic_traversals(depth, target) {
        println!("  {} → {}", p.name, p.payload);
    }

    println!("\nURL Encoded:");
    for p in url_encoded_traversals(depth, target).iter().take(3) {
        println!("  {} → {}", p.name, p.payload);
    }

    println!("\nNull Byte (ext: md):");
    for p in null_byte_traversals(depth, target, "md") {
        println!("  {} → {}", p.name, p.payload);
    }

    println!("\nJuice Shop Specific:");
    for p in juice_shop_traversal() {
        println!("  {} → {}", p.name, p.payload);
    }
}

fn handle_passwords(cmd: PasswordCommands) {
    match cmd {
        PasswordCommands::Top => {
            println!("=== Top Common Passwords ===\n");
            for p in top_passwords().iter().take(20) {
                println!("  {:20} ({:?})", p.password, p.category);
            }
        }
        PasswordCommands::Identify { hash } => {
            let hash_type = identify_hash(&hash);
            println!("Hash: {}", hash);
            println!("Type: {:?}", hash_type);
        }
        PasswordCommands::Variations { word } => {
            println!("=== Password Variations for '{}' ===\n", word);
            for v in generate_variations(&word) {
                println!("  {}", v);
            }
        }
        PasswordCommands::JuiceShop => {
            println!("=== Juice Shop Credentials ===\n");
            for c in juice_shop_credentials() {
                println!("{}", c.email);
                println!("  Password: {}", c.password);
                println!("  Note: {}\n", c.description);
            }

            println!("=== Security Question Answers ===\n");
            for a in juice_shop_security_answers() {
                println!("{}", a.email);
                println!("  Q: {}", a.question);
                println!("  A: {}\n", a.answer);
            }
        }
    }
}

fn handle_idor(cmd: IdorCommands) {
    match cmd {
        IdorCommands::Endpoints => {
            println!("=== Common IDOR Endpoints ===\n");
            for e in common_idor_endpoints() {
                println!("{:40} - {}", e.pattern, e.description);
            }
        }
        IdorCommands::Ids { current, range } => {
            println!("=== ID Variations for {} (range: {}) ===\n", current, range);
            let ids = generate_id_variations(current, range);
            for id in ids {
                println!("  {}", id);
            }
        }
        IdorCommands::JuiceShop => {
            println!("=== Juice Shop IDOR Endpoints ===\n");
            for e in juice_shop_idor_endpoints() {
                println!("{}", e.pattern);
                println!("  {}\n", e.description);
            }

            println!("Example (View Basket challenge):");
            println!("  fetch('/rest/basket/1', {{");
            println!("    headers: {{ 'Authorization': 'Bearer ' + token }}");
            println!("  }}).then(r => r.json())");
        }
    }
}

fn handle_tampering(cmd: TamperingCommands) {
    match cmd {
        TamperingCommands::Negative { field, value } => {
            println!("=== Negative Value Tests for '{}' ===\n", field);
            for t in negative_value_tests(&field, value) {
                println!("{}", t.name);
                println!("  Original: {}", t.original);
                println!("  Tampered: {}\n", t.tampered);
            }
        }
        TamperingCommands::MassAssignment => {
            println!("=== Mass Assignment Payloads ===\n");
            let base = json!({"email": "test@test.com", "password": "test123"});
            for t in mass_assignment_tests(&base) {
                println!("{}", t.name);
                println!("  {}\n", t.tampered);
            }
        }
        TamperingCommands::Privilege { target_id } => {
            println!(
                "=== Privilege Escalation Tests (target: {}) ===\n",
                target_id
            );
            let base = json!({"data": "test"});
            for t in privilege_escalation_tests(&base, target_id) {
                println!("{}", t.name);
                println!("  {}\n", t.tampered);
            }
        }
        TamperingCommands::JuiceShop => {
            println!("=== Juice Shop Parameter Tampering ===\n");
            for t in juice_shop_tampering_tests() {
                println!("{} ({:?})", t.name, t.category);
                println!("  Original: {}", t.original);
                println!("  Tampered: {}\n", t.tampered);
            }
        }
    }
}
