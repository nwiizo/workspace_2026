//! Fuzzing Module Demo
//!
//! Demonstrates the fuzzing capabilities of Rectitude including:
//! - Mutation strategies for payload encoding
//! - Boundary value generators
//! - ParamFuzzer for HTTP parameter fuzzing
//! - Specialized fuzzers (SQLi, XSS, Path Traversal)
//! - Built-in wordlists
//!
//! Run with: cargo run --example fuzzing_demo

use rectitude::client::SecurityClient;
use rectitude::fuzzing::{
    // Mutation strategies
    MutationStrategy,
    // Fuzzers
    ParamFuzzer,
    ParamLocation,
    SqliFuzzer,
    SuccessCriteria,
    TraversalFuzzer,
    XssContext,
    XssFuzzer,
    // Wordlists
    common_passwords,
    common_usernames,
    // Generators
    format_strings,
    integer_boundaries_str,
    numeric_edges,
    special_chars,
    string_lengths,
};
use rectitude::payloads::sqli::DbType;
use reqwest::Method;

const BASE_URL: &str = "http://localhost:3000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Rectitude Fuzzing Module Demo ===\n");

    // Demo 1: Mutation Strategies
    demo_mutation_strategies();

    // Demo 2: Generators
    demo_generators();

    // Demo 3: Wordlists
    demo_wordlists();

    // Demo 4: ParamFuzzer (requires running server)
    if std::env::var("RUN_HTTP_TESTS").is_ok() {
        demo_param_fuzzer().await?;
        demo_sqli_fuzzer().await?;
        demo_xss_fuzzer().await?;
        demo_traversal_fuzzer().await?;
        demo_auth_bruteforce().await?;
    } else {
        println!("\n[Skipping HTTP tests - set RUN_HTTP_TESTS=1 to enable]");
        println!("These tests require a running server at {}", BASE_URL);
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}

// ============ Demo 1: Mutation Strategies ============

fn demo_mutation_strategies() {
    println!("--- Mutation Strategies ---\n");

    let payload = "<script>alert(1)</script>";

    // URL encoding
    let url_encoded = MutationStrategy::url_encode().apply(payload);
    println!("URL Encoded: {:?}", url_encoded);

    // Double URL encoding
    let double_encoded = MutationStrategy::double_url_encode().apply(payload);
    println!("Double URL Encoded: {:?}", double_encoded);

    // HTML entity encoding
    let html_encoded = MutationStrategy::HtmlEncode.apply(payload);
    println!(
        "HTML Encoded (first 3): {:?}",
        &html_encoded[..3.min(html_encoded.len())]
    );

    // Case variations
    let case_varied = MutationStrategy::CaseVariation.apply("Script");
    println!("Case Variations: {:?}", case_varied);

    // Unicode homoglyphs
    let unicode = MutationStrategy::Unicode.apply("admin");
    println!("Unicode Variations: {:?}", &unicode[..5.min(unicode.len())]);

    // Wrapper (prefix/suffix)
    let wrapped = MutationStrategy::wrap("<!--", "-->").apply("payload");
    println!("Wrapped: {:?}", wrapped);

    // Chained strategies (collect all variations)
    let all = MutationStrategy::all_encodings().apply("<");
    println!("All Encodings for '<': {:?}", all);

    // XSS bypass chain
    let xss_bypass = MutationStrategy::xss_bypass_encodings().apply("<img src=x>");
    println!("XSS Bypass Variations: {} total", xss_bypass.len());

    println!();
}

// ============ Demo 2: Generators ============

fn demo_generators() {
    println!("--- Payload Generators ---\n");

    // Integer boundaries
    let int_bounds = integer_boundaries_str(0, 100);
    println!(
        "Integer Boundaries (0-100): {:?}",
        &int_bounds[..10.min(int_bounds.len())]
    );

    // String lengths
    let str_lens = string_lengths(10);
    println!(
        "String Lengths (max 10): lengths = {:?}",
        str_lens.iter().map(|s| s.len()).collect::<Vec<_>>()
    );

    // Format strings
    let fmt_strings = format_strings();
    println!("Format Strings: {:?}", &fmt_strings[..5]);

    // Special characters
    let special = special_chars();
    println!("Special Chars: {} total", special.len());

    // Numeric edge cases
    let numeric = numeric_edges();
    println!("Numeric Edges: {:?}", &numeric[..10.min(numeric.len())]);

    println!();
}

// ============ Demo 3: Wordlists ============

fn demo_wordlists() {
    println!("--- Built-in Wordlists ---\n");

    let usernames = common_usernames();
    println!("Common Usernames: {} entries", usernames.len());
    println!("  Examples: {:?}", &usernames[..5]);

    let passwords = common_passwords();
    println!("Common Passwords: {} entries", passwords.len());
    println!("  Examples: {:?}", &passwords[..5]);

    println!();
}

// ============ Demo 4: ParamFuzzer ============

async fn demo_param_fuzzer() -> anyhow::Result<()> {
    println!("--- ParamFuzzer Demo ---\n");

    let client = SecurityClient::with_base_url(BASE_URL)?;

    // Basic parameter fuzzer
    let fuzzer = ParamFuzzer::new(client, "/rest/products/search", "q")
        .method(Method::GET)
        .location(ParamLocation::Query)
        .mutation(MutationStrategy::all_encodings())
        .success_when(SuccessCriteria::StatusCode(200))
        .concurrency(5);

    // Test payloads
    let payloads = vec![
        "test".to_string(),
        "'".to_string(),
        "\"".to_string(),
        "<script>".to_string(),
        "{{7*7}}".to_string(),
        "%00".to_string(),
        "../".to_string(),
    ];

    println!("Fuzzing /rest/products/search?q=...");
    let result = fuzzer.run(payloads).await;

    println!("{}", result.to_report());

    Ok(())
}

// ============ Demo 5: SQLi Fuzzer ============

async fn demo_sqli_fuzzer() -> anyhow::Result<()> {
    println!("--- SQLi Fuzzer Demo ---\n");

    let client = SecurityClient::with_base_url(BASE_URL)?;

    // SQLi fuzzer with auto-generated payloads
    let fuzzer = SqliFuzzer::new(client, "/rest/user/login", "email")
        .db_type(DbType::Sqlite)
        .include_blind(true)
        .include_time_based(false) // Skip time-based for speed
        .with_param("password", "anything")
        .success_when(SuccessCriteria::StatusCode(200))
        .concurrency(3);

    println!(
        "Generated {} SQLi payloads",
        fuzzer.generate_payloads().len()
    );
    println!("Fuzzing /rest/user/login (email parameter)...");

    let result = fuzzer.run().await;

    if result.has_success() {
        println!(
            "\n[!] Found {} successful SQLi payloads!",
            result.successful.len()
        );
        for hit in result.successful.iter().take(3) {
            println!("    - Status {}: (payload hidden)", hit.response_status);
        }
    } else {
        println!("No successful SQLi payloads found");
    }

    println!("Stats: {}", result.stats);
    println!();

    Ok(())
}

// ============ Demo 6: XSS Fuzzer ============

async fn demo_xss_fuzzer() -> anyhow::Result<()> {
    println!("--- XSS Fuzzer Demo ---\n");

    let client = SecurityClient::with_base_url(BASE_URL)?;

    // XSS fuzzer for HTML context
    let fuzzer = XssFuzzer::new(client, "/rest/products/search", "q")
        .context(XssContext::Html)
        .include_polyglots(true)
        .method(Method::GET)
        .location(ParamLocation::Query)
        .success_when(
            SuccessCriteria::StatusCode(200)
                .and(SuccessCriteria::BodyContains("script".to_string())),
        )
        .concurrency(5);

    println!(
        "Generated {} XSS payloads",
        fuzzer.generate_payloads().len()
    );
    println!("Fuzzing /rest/products/search for XSS...");

    let result = fuzzer.run().await;

    if result.has_success() {
        println!(
            "\n[!] Found {} reflected XSS candidates!",
            result.successful.len()
        );
    } else {
        println!("No XSS reflection found (payloads may be sanitized)");
    }

    println!("Stats: {}", result.stats);
    println!();

    Ok(())
}

// ============ Demo 7: Path Traversal Fuzzer ============

async fn demo_traversal_fuzzer() -> anyhow::Result<()> {
    println!("--- Path Traversal Fuzzer Demo ---\n");

    let client = SecurityClient::with_base_url(BASE_URL)?;

    // Path traversal fuzzer
    let fuzzer = TraversalFuzzer::new(client, "/ftp", "file")
        .target_file("/etc/passwd")
        .max_depth(5)
        .encoding_variations(true)
        .method(Method::GET)
        .location(ParamLocation::Query)
        .success_when(SuccessCriteria::BodyContains("root:".to_string()));

    println!(
        "Generated {} traversal payloads",
        fuzzer.generate_payloads().len()
    );
    println!("Fuzzing /ftp?file=... for path traversal...");

    let result = fuzzer.run().await;

    if result.has_success() {
        println!("\n[!] Path traversal successful!");
        for hit in &result.successful {
            println!("    - Status {}", hit.response_status);
        }
    } else {
        println!("No path traversal found");
    }

    println!("Stats: {}", result.stats);
    println!();

    Ok(())
}

// ============ Demo 8: Auth Bruteforce ============

async fn demo_auth_bruteforce() -> anyhow::Result<()> {
    println!("--- Auth Bruteforce Demo ---\n");

    let client = SecurityClient::with_base_url(BASE_URL)?;

    // Combine usernames and passwords for credential testing
    let usernames = &common_usernames()[..5]; // Limit for demo
    let passwords = &common_passwords()[..5];

    let mut credentials = Vec::new();
    for user in usernames {
        for pass in passwords {
            credentials.push(format!("{}:{}", user, pass));
        }
    }

    println!("Testing {} credential combinations...", credentials.len());

    // Use ParamFuzzer with JSON body
    let _fuzzer = ParamFuzzer::new(client, "/rest/user/login", "email")
        .method(Method::POST)
        .location(ParamLocation::JsonBody)
        .with_param("password", "test") // Will be overridden per-request
        .success_when(SuccessCriteria::StatusCode(200))
        .concurrency(3);

    // For demo, just show configuration
    println!("Fuzzer configured for credential testing");
    println!("  Endpoint: POST /rest/user/login");
    println!("  Usernames: {:?}", usernames);
    println!("  Passwords: {:?}", passwords);
    println!();

    Ok(())
}
