#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use std::env;
use std::process;

use clap::{Parser, Subcommand};

use rustguard::config::{OutputFormat, RustGuardConfig};
use rustguard::driver::RustGuardCallbacks;

/// cargo-rustguard — MIR-level static analysis for Rust
#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyze Rust code for unsafe impact and ownership anomalies
    Rustguard(Args),
}

#[derive(Parser, Debug)]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// Output format: text, json, sarif
    #[arg(short, long, default_value = "text")]
    format: String,

    /// Additional arguments to pass to cargo
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    cargo_args: Vec<String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if is_wrapper_mode(&args) {
        run_as_rustc_wrapper(args);
    } else {
        run_as_cargo_subcommand();
    }
}

/// Detect if we're being invoked as a rustc wrapper by cargo.
/// When cargo uses us as RUSTC_WORKSPACE_WRAPPER, args[1] is the path to rustc.
fn is_wrapper_mode(args: &[String]) -> bool {
    args.get(1)
        .is_some_and(|arg| arg.ends_with("rustc") || arg == "rustc")
}

/// User-facing mode: `cargo rustguard [args]`
/// Sets up RUSTC_WORKSPACE_WRAPPER and invokes `cargo check`.
fn run_as_cargo_subcommand() {
    let cli = Cli::parse();
    let Commands::Rustguard(args) = cli.command;

    let self_path = match env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("rustguard: failed to get current executable path: {e}");
            process::exit(2);
        }
    };

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    let mut cmd = process::Command::new(cargo);
    cmd.arg("check")
        .env("RUSTC_WORKSPACE_WRAPPER", &self_path)
        .env("RUSTGUARD_FORMAT", &args.format);

    // Pass config file path if specified
    if let Some(ref config_path) = args.config {
        cmd.env("RUSTGUARD_CONFIG", config_path);
    }

    // Forward additional cargo args
    for arg in &args.cargo_args {
        cmd.arg(arg);
    }

    let status = match cmd.status() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rustguard: failed to run cargo: {e}");
            process::exit(2);
        }
    };

    process::exit(status.code().unwrap_or(1));
}

/// Wrapper mode: invoked by cargo as if we were rustc.
/// args[0] = our binary, args[1] = rustc path, args[2..] = rustc arguments
fn run_as_rustc_wrapper(mut args: Vec<String>) {
    // Load config from file or defaults
    let mut config = match RustGuardConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rustguard: {e}");
            process::exit(2);
        }
    };

    // Override output format from env
    if let Ok(fmt_str) = env::var("RUSTGUARD_FORMAT")
        && let Ok(fmt) = fmt_str.parse::<OutputFormat>()
    {
        config.output.format = fmt;
    }

    // Install ICE hook for better error reporting
    rustc_driver::install_ice_hook(
        "https://github.com/nwiizo/workspace_2026/issues/new",
        |_| (),
    );

    // Set up args: remove our binary name's position and set argv[0] to rustc
    // args = [cargo-rustguard, /path/to/rustc, source.rs, --edition=2024, ...]
    // We need: [/path/to/rustc, source.rs, --edition=2024, ...]
    let _self_path = args.remove(0);

    // Check if this is a primary package compilation (not a build script, proc-macro, etc.)
    // We only want to analyze the actual source code, not build.rs or proc macros
    // Args may be in --flag=value or --flag value format
    let is_primary_package = args
        .iter()
        .any(|a| a.starts_with("--crate-type") || a.starts_with("--edition"));

    if !is_primary_package || is_build_script(&args) {
        // For non-primary compilations, just pass through to real rustc
        let rustc = args.remove(0);
        let status = process::Command::new(rustc)
            .args(&args)
            .status()
            .unwrap_or_else(|e| {
                eprintln!("rustguard: failed to run rustc: {e}");
                process::exit(2);
            });
        process::exit(status.code().unwrap_or(1));
    }

    let mut callbacks = RustGuardCallbacks::new(config);

    // Run the compiler with our callbacks
    // run_compiler strips argv[0] internally, so args[0] should be the binary name
    let result = rustc_driver::catch_fatal_errors(|| {
        rustc_driver::run_compiler(&args, &mut callbacks);
    });

    match result {
        Ok(()) => {
            // Exit 1 if error-severity findings were detected (CI gating)
            if callbacks.has_errors() {
                process::exit(1);
            }
        }
        Err(_) => {
            eprintln!("rustguard: internal compiler error");
            process::exit(2);
        }
    }
}

fn is_build_script(args: &[String]) -> bool {
    args.iter()
        .any(|a| a.contains("build_script_build") || a.contains("build-script-build"))
}
