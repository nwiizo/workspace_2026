//! Fuzzing module for automated payload generation and testing
//!
//! This module provides tools for:
//! - **Mutation strategies** - Transform payloads with various encodings
//! - **Payload generators** - Generate boundary values, format strings, etc.
//! - **Fuzzers** - HTTP parameter, SQLi, XSS, and path traversal fuzzing
//! - **Wordlists** - Common usernames, passwords, paths, and endpoints
//!
//! # Example
//!
//! ```rust,ignore
//! use rectitude::fuzzing::{ParamFuzzer, MutationStrategy, SuccessCriteria};
//! use rectitude::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = SecurityClient::with_base_url("http://localhost:3000")?;
//!
//!     let fuzzer = ParamFuzzer::new(client, "/api/search", "q")
//!         .mutation(MutationStrategy::all_encodings())
//!         .success_when(SuccessCriteria::StatusCode(200))
//!         .concurrency(10);
//!
//!     let payloads = vec!["test", "'", "\"", "<script>", "{{7*7}}"]
//!         .into_iter()
//!         .map(String::from)
//!         .collect();
//!
//!     let result = fuzzer.run(payloads).await;
//!
//!     println!("Found {} successful payloads", result.successful.len());
//!     println!("Rate: {:.2} req/s", result.stats.requests_per_second);
//!
//!     Ok(())
//! }
//! ```

pub mod fuzzer;
pub mod generator;
pub mod mutation;
pub mod result;
pub mod wordlist;

// Re-exports for convenient access
pub use fuzzer::{
    Fuzzer, ParamFuzzer, ParamLocation, SqliFuzzer, SuccessCriteria, TraversalFuzzer, XssContext,
    XssFuzzer,
};
pub use generator::{
    boolean_values, datetime_edges, email_edge_cases, format_strings, integer_boundaries,
    integer_boundaries_str, numeric_edges, special_chars, string_lengths, url_edge_cases,
};
pub use mutation::MutationStrategy;
pub use result::{FuzzingError, FuzzingHit, FuzzingMiss, FuzzingResult, FuzzingStats};
pub use wordlist::{
    common_endpoints, common_extensions, common_headers, common_params, common_passwords,
    common_paths, common_subdomains, common_usernames,
};
