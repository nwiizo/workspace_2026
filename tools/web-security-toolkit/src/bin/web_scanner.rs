//! Web vulnerability scanner CLI
//!
//! Usage:
//!   web-scanner scan https://example.com
//!   web-scanner scan https://example.com --headers-only
//!   web-scanner scan https://example.com --output report.md
//!   web-scanner check-headers https://example.com

use clap::{Parser, Subcommand};
use web_security_toolkit::headers::{analyze_headers, recommended_headers, Severity};
use web_security_toolkit::http_client::SecurityClient;
use web_security_toolkit::scanner::{generate_report, ScanConfig, Scanner};

#[derive(Parser)]
#[command(name = "web-scanner")]
#[command(about = "Web vulnerability scanner for security testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a full vulnerability scan
    Scan {
        /// Target URL to scan
        url: String,
        /// Only check security headers
        #[arg(long)]
        headers_only: bool,
        /// Skip common paths check
        #[arg(long)]
        skip_paths: bool,
        /// Output file (markdown format)
        #[arg(short, long)]
        output: Option<String>,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Check security headers only
    CheckHeaders {
        /// Target URL
        url: String,
        /// Show recommendations
        #[arg(long)]
        recommendations: bool,
    },
    /// Check cookies security
    CheckCookies {
        /// Target URL
        url: String,
    },
    /// Test CORS configuration
    TestCors {
        /// Target URL
        url: String,
        /// Origin to test
        #[arg(short, long, default_value = "https://evil.com")]
        origin: String,
    },
    /// Show recommended security headers
    RecommendedHeaders,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            url,
            headers_only,
            skip_paths,
            output,
            json,
        } => {
            let config = ScanConfig {
                check_headers: true,
                check_cookies: !headers_only,
                check_info_disclosure: !headers_only,
                check_common_paths: !headers_only && !skip_paths,
                check_cors: !headers_only,
                ..Default::default()
            };

            println!("[*] Starting scan of {}", url);
            let scanner = Scanner::new(config);
            let result = scanner.scan(&url);

            if json {
                match serde_json::to_string_pretty(&result) {
                    Ok(json_str) => println!("{}", json_str),
                    Err(e) => eprintln!("[-] JSON serialization error: {}", e),
                }
            } else {
                println!("\n{}", format_summary(&result));

                if let Some(output_path) = output {
                    let report = generate_report(&result);
                    match std::fs::write(&output_path, report) {
                        Ok(_) => println!("\n[+] Report saved to {}", output_path),
                        Err(e) => eprintln!("[-] Failed to write report: {}", e),
                    }
                } else {
                    // Print findings to console
                    print_findings(&result);
                }
            }
        }
        Commands::CheckHeaders {
            url,
            recommendations,
        } => {
            println!("[*] Checking security headers for {}", url);

            let client = SecurityClient::new();
            match client.get(&url) {
                Ok(response) => {
                    let checks = analyze_headers(&response.headers);

                    println!("\n=== Security Headers Analysis ===\n");

                    for check in &checks {
                        let status = if check.present { "✓" } else { "✗" };
                        let severity = match check.severity {
                            Severity::Critical => "CRIT",
                            Severity::High => "HIGH",
                            Severity::Medium => "MED ",
                            Severity::Low => "LOW ",
                            Severity::Info => "INFO",
                        };

                        println!("{} [{}] {}", status, severity, check.name);

                        if let Some(value) = &check.value {
                            println!("       Value: {}", truncate(value, 60));
                        }

                        if !check.description.is_empty() && check.severity != Severity::Info {
                            println!("       {}", check.description);
                        }
                    }

                    if recommendations {
                        println!("\n=== Recommendations ===\n");
                        for check in checks.iter().filter(|c| {
                            !c.recommendation.is_empty() && c.severity != Severity::Info
                        }) {
                            println!("• {}: {}", check.name, check.recommendation);
                        }
                    }
                }
                Err(e) => eprintln!("[-] Request failed: {}", e),
            }
        }
        Commands::CheckCookies { url } => {
            println!("[*] Checking cookies for {}", url);

            let client = SecurityClient::new();
            match client.get(&url) {
                Ok(response) => {
                    if response.cookies.is_empty() {
                        println!("\n[*] No cookies set by the server");
                    } else {
                        println!("\n=== Cookie Analysis ===\n");

                        for cookie in &response.cookies {
                            println!("Cookie: {}", cookie.name);
                            println!("  Value: {}", truncate(&cookie.value, 40));
                            println!("  Secure: {}", if cookie.secure { "Yes" } else { "No ⚠" });
                            println!(
                                "  HttpOnly: {}",
                                if cookie.http_only { "Yes" } else { "No ⚠" }
                            );
                            println!(
                                "  SameSite: {}",
                                cookie.same_site.as_deref().unwrap_or("Not set ⚠")
                            );

                            let issues = cookie.security_issues();
                            if !issues.is_empty() {
                                println!("  Issues:");
                                for issue in issues {
                                    println!("    - {}", issue);
                                }
                            }
                            println!();
                        }
                    }
                }
                Err(e) => eprintln!("[-] Request failed: {}", e),
            }
        }
        Commands::TestCors { url, origin } => {
            println!("[*] Testing CORS for {} with origin {}", url, origin);

            let client = SecurityClient::new()
                .with_header("Origin", &origin)
                .unwrap_or_else(|_| SecurityClient::new());

            match client.get(&url) {
                Ok(response) => {
                    println!("\n=== CORS Analysis ===\n");

                    let acao = response.headers.get("access-control-allow-origin");
                    let acac = response.headers.get("access-control-allow-credentials");
                    let acam = response.headers.get("access-control-allow-methods");
                    let acah = response.headers.get("access-control-allow-headers");

                    println!(
                        "Access-Control-Allow-Origin: {}",
                        acao.unwrap_or(&"(not set)".to_string())
                    );
                    println!(
                        "Access-Control-Allow-Credentials: {}",
                        acac.unwrap_or(&"(not set)".to_string())
                    );
                    println!(
                        "Access-Control-Allow-Methods: {}",
                        acam.unwrap_or(&"(not set)".to_string())
                    );
                    println!(
                        "Access-Control-Allow-Headers: {}",
                        acah.unwrap_or(&"(not set)".to_string())
                    );

                    println!("\n=== Assessment ===\n");

                    match acao {
                        None => println!("[+] No CORS headers - resource not shared cross-origin"),
                        Some(v) if v == "*" => {
                            println!("[!] Wildcard origin - any site can access this resource");
                            if acac.map(|c| c == "true").unwrap_or(false) {
                                println!("[!!] CRITICAL: Credentials allowed with wildcard (should be rejected by browser)");
                            }
                        }
                        Some(v) if v == &origin => {
                            println!("[!!] Origin reflected - vulnerable to CORS bypass");
                            if acac.map(|c| c == "true").unwrap_or(false) {
                                println!(
                                    "[!!!] CRITICAL: Credentials allowed with reflected origin"
                                );
                            }
                        }
                        Some(v) if v == "null" => {
                            println!(
                                "[!] Null origin allowed - can be exploited via sandboxed iframes"
                            );
                        }
                        Some(_) => {
                            println!("[+] Origin not reflected - whitelist appears to be in place");
                        }
                    }
                }
                Err(e) => eprintln!("[-] Request failed: {}", e),
            }
        }
        Commands::RecommendedHeaders => {
            println!("=== Recommended Security Headers ===\n");

            for (name, value) in recommended_headers() {
                println!("{}: {}\n", name, value);
            }
        }
    }
}

fn format_summary(result: &web_security_toolkit::scanner::ScanResult) -> String {
    format!(
        "=== Scan Complete ===
Target: {}
Duration: {}ms

Findings Summary:
  Critical: {}
  High: {}
  Medium: {}
  Low: {}
  Info: {}
  Total: {}",
        result.target,
        result.duration_ms,
        result.summary.critical,
        result.summary.high,
        result.summary.medium,
        result.summary.low,
        result.summary.info,
        result.summary.total_findings
    )
}

fn print_findings(result: &web_security_toolkit::scanner::ScanResult) {
    let severity_order = ["Critical", "High", "Medium", "Low"];

    for severity in severity_order {
        let findings: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect();

        if !findings.is_empty() {
            println!("\n=== {} Findings ===\n", severity);
            for f in findings {
                println!("[{}] {}", f.category, f.title);
                println!("    {}", f.description);
                if let Some(evidence) = &f.evidence {
                    println!("    Evidence: {}", truncate(evidence, 60));
                }
                println!("    Fix: {}", f.recommendation);
                println!();
            }
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}
