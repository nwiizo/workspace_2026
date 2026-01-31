//! Security-focused HTTP client CLI
//!
//! Usage:
//!   http-client get https://example.com
//!   http-client post https://api.com/login -d '{"user":"admin"}'
//!   http-client get https://example.com -H "Authorization: Bearer token"
//!   http-client get https://example.com --jwt "eyJhbGciOi..."
//!   http-client get https://example.com --cookie "session=abc123"

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use web_security_toolkit::http_client::SecurityClient;

#[derive(Parser)]
#[command(name = "http-client")]
#[command(about = "Security-focused HTTP client for web testing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send GET request
    Get {
        /// Target URL
        url: String,
        /// Add header (can be used multiple times)
        #[arg(short = 'H', long = "header")]
        headers: Vec<String>,
        /// JWT token for Authorization header
        #[arg(long)]
        jwt: Option<String>,
        /// Cookie to send
        #[arg(long)]
        cookie: Vec<String>,
        /// Follow redirects
        #[arg(short, long)]
        follow: bool,
        /// Show response headers
        #[arg(long)]
        show_headers: bool,
        /// Only show status code
        #[arg(long)]
        status_only: bool,
    },
    /// Send POST request
    Post {
        /// Target URL
        url: String,
        /// Request body (JSON)
        #[arg(short, long)]
        data: Option<String>,
        /// Form data (key=value)
        #[arg(short = 'F', long = "form")]
        form: Vec<String>,
        /// Add header
        #[arg(short = 'H', long = "header")]
        headers: Vec<String>,
        /// JWT token
        #[arg(long)]
        jwt: Option<String>,
        /// Cookie
        #[arg(long)]
        cookie: Vec<String>,
        /// Show response headers
        #[arg(long)]
        show_headers: bool,
    },
    /// Send custom method request
    Request {
        /// HTTP method (PUT, DELETE, PATCH, etc.)
        method: String,
        /// Target URL
        url: String,
        /// Request body
        #[arg(short, long)]
        data: Option<String>,
        /// Content-Type header
        #[arg(short, long)]
        content_type: Option<String>,
        /// Add header
        #[arg(short = 'H', long = "header")]
        headers: Vec<String>,
        /// JWT token
        #[arg(long)]
        jwt: Option<String>,
        /// Show response headers
        #[arg(long)]
        show_headers: bool,
    },
    /// Analyze response cookies
    Cookies {
        /// Target URL
        url: String,
    },
    /// Extract value from JSON response
    JsonExtract {
        /// Target URL
        url: String,
        /// JSON path (e.g., "data.token")
        path: String,
        /// Add header
        #[arg(short = 'H', long = "header")]
        headers: Vec<String>,
        /// JWT token
        #[arg(long)]
        jwt: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Get {
            url,
            headers,
            jwt,
            cookie,
            follow,
            show_headers,
            status_only,
        } => {
            let client = build_client(&headers, jwt.as_deref(), &cookie);

            match client.get(&url) {
                Ok(mut response) => {
                    // Handle redirects manually if requested
                    if follow {
                        let mut count = 0;
                        while response.is_redirect() && count < 10 {
                            if let Some(location) = response.redirect_location() {
                                let next_url = if location.starts_with("http") {
                                    location.to_string()
                                } else {
                                    format!("{}{}", url.trim_end_matches('/'), location)
                                };
                                eprintln!("[*] Following redirect to {}", next_url);
                                match client.get(&next_url) {
                                    Ok(r) => response = r,
                                    Err(e) => {
                                        eprintln!("[-] Redirect failed: {}", e);
                                        break;
                                    }
                                }
                            } else {
                                break;
                            }
                            count += 1;
                        }
                    }

                    if status_only {
                        println!("{}", response.status);
                    } else {
                        print_response(&response, show_headers);
                    }
                }
                Err(e) => eprintln!("[-] Request failed: {}", e),
            }
        }
        Commands::Post {
            url,
            data,
            form,
            headers,
            jwt,
            cookie,
            show_headers,
        } => {
            let client = build_client(&headers, jwt.as_deref(), &cookie);

            let result = if !form.is_empty() {
                let form_data: HashMap<String, String> = form
                    .iter()
                    .filter_map(|f| {
                        let parts: Vec<&str> = f.splitn(2, '=').collect();
                        if parts.len() == 2 {
                            Some((parts[0].to_string(), parts[1].to_string()))
                        } else {
                            None
                        }
                    })
                    .collect();
                client.post_form(&url, &form_data)
            } else if let Some(json_data) = data {
                match serde_json::from_str::<serde_json::Value>(&json_data) {
                    Ok(json) => client.post_json(&url, &json),
                    Err(e) => {
                        eprintln!("[-] Invalid JSON: {}", e);
                        return;
                    }
                }
            } else {
                client.post_json(&url, &serde_json::json!({}))
            };

            match result {
                Ok(response) => print_response(&response, show_headers),
                Err(e) => eprintln!("[-] Request failed: {}", e),
            }
        }
        Commands::Request {
            method,
            url,
            data,
            content_type,
            headers,
            jwt,
            show_headers,
        } => {
            let client = build_client(&headers, jwt.as_deref(), &[]);

            match client.request(&method, &url, data.as_deref(), content_type.as_deref()) {
                Ok(response) => print_response(&response, show_headers),
                Err(e) => eprintln!("[-] Request failed: {}", e),
            }
        }
        Commands::Cookies { url } => {
            let client = SecurityClient::new();

            match client.get(&url) {
                Ok(response) => {
                    if response.cookies.is_empty() {
                        println!("No cookies set");
                    } else {
                        println!("=== Cookies ===\n");
                        for cookie in &response.cookies {
                            println!("{}={}", cookie.name, cookie.value);
                            println!("  Path: {}", cookie.path.as_deref().unwrap_or("/"));
                            println!(
                                "  Domain: {}",
                                cookie.domain.as_deref().unwrap_or("(default)")
                            );
                            println!("  Secure: {}", cookie.secure);
                            println!("  HttpOnly: {}", cookie.http_only);
                            println!(
                                "  SameSite: {}",
                                cookie.same_site.as_deref().unwrap_or("(not set)")
                            );

                            let issues = cookie.security_issues();
                            if !issues.is_empty() {
                                println!("  Security Issues:");
                                for issue in issues {
                                    println!("    ⚠ {}", issue);
                                }
                            }
                            println!();
                        }
                    }
                }
                Err(e) => eprintln!("[-] Request failed: {}", e),
            }
        }
        Commands::JsonExtract {
            url,
            path,
            headers,
            jwt,
        } => {
            let client = build_client(&headers, jwt.as_deref(), &[]);

            match client.get(&url) {
                Ok(response) => {
                    if let Some(value) = response.json_value(&path) {
                        match value {
                            serde_json::Value::String(s) => println!("{}", s),
                            v => println!("{}", v),
                        }
                    } else {
                        eprintln!("[-] Path '{}' not found in response", path);
                    }
                }
                Err(e) => eprintln!("[-] Request failed: {}", e),
            }
        }
    }
}

fn build_client(headers: &[String], jwt: Option<&str>, cookies: &[String]) -> SecurityClient {
    let mut client = SecurityClient::new();

    if let Some(token) = jwt {
        client = client.with_jwt(token);
    }

    for cookie in cookies {
        if let Some((name, value)) = cookie.split_once('=') {
            client = client.with_cookie(name.trim(), value.trim());
        }
    }

    // Add headers using mutable reference
    for header in headers {
        if let Some((name, value)) = header.split_once(':') {
            let _ = client.add_header(name.trim(), value.trim());
        }
    }

    client
}

fn print_response(
    response: &web_security_toolkit::http_client::SecurityResponse,
    show_headers: bool,
) {
    println!("HTTP {}", response.status);

    if show_headers {
        println!("\n=== Headers ===");
        let mut headers: Vec<_> = response.headers.iter().collect();
        headers.sort_by_key(|(k, _)| k.as_str());
        for (name, value) in headers {
            println!("{}: {}", name, value);
        }
        println!();
    }

    if !response.body.is_empty() {
        // Try to pretty-print JSON
        if response
            .content_type
            .as_ref()
            .map(|ct| ct.contains("json"))
            .unwrap_or(false)
        {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response.body) {
                if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                    println!("{}", pretty);
                    return;
                }
            }
        }

        println!("{}", response.body);
    }
}
