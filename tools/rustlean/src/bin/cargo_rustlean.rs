use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "cargo-rustlean")]
#[command(about = "MIR-based optimization assistance tool for Rust")]
#[command(version)]
struct Args {
    /// Cargo subcommand name (automatically set to "rustlean")
    #[arg(hide = true)]
    _subcommand: Option<String>,

    /// Path to rustlean.toml configuration file
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Output format: text or json
    #[arg(long, default_value = "text")]
    format: String,

    /// Additional arguments to pass to cargo check
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    cargo_args: Vec<String>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    // Find rustlean-driver binary (sibling of this binary)
    let driver = match find_driver() {
        Some(d) => d,
        None => {
            eprintln!("rustlean: error: could not find rustlean-driver binary");
            return ExitCode::FAILURE;
        }
    };

    // Get sysroot for dynamic library path
    let sysroot = match get_sysroot() {
        Some(s) => s,
        None => {
            eprintln!("rustlean: error: could not determine rustc sysroot");
            return ExitCode::FAILURE;
        }
    };

    let lib_dir = PathBuf::from(&sysroot).join("lib");

    let mut cmd = Command::new("cargo");
    cmd.arg("check");

    // Set RUSTC_WRAPPER to our driver
    cmd.env("RUSTC_WRAPPER", &driver);

    // Set dynamic library path for rustc_private linking
    let lib_path_var = if cfg!(target_os = "macos") {
        "DYLD_LIBRARY_PATH"
    } else {
        "LD_LIBRARY_PATH"
    };
    let existing = env::var(lib_path_var).unwrap_or_default();
    let new_path = if existing.is_empty() {
        lib_dir.display().to_string()
    } else {
        format!("{}:{existing}", lib_dir.display())
    };
    cmd.env(lib_path_var, new_path);

    // Pass config path if provided
    if let Some(config_path) = &args.config {
        cmd.env("RUSTLEAN_CONFIG", config_path);
    }

    // Pass output format
    cmd.env("RUSTLEAN_FORMAT", &args.format);

    // Forward additional cargo arguments
    for arg in &args.cargo_args {
        cmd.arg(arg);
    }

    match cmd.status() {
        Ok(status) => {
            if status.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("rustlean: error running cargo: {e}");
            ExitCode::FAILURE
        }
    }
}

fn find_driver() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;
    let driver = dir.join("rustlean-driver");
    if driver.exists() {
        Some(driver)
    } else {
        // Fallback: look in PATH
        which_rustlean_driver()
    }
}

fn which_rustlean_driver() -> Option<PathBuf> {
    // Scan PATH directories instead of relying on `which` (not available on Windows)
    let path_var = env::var("PATH").ok()?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join("rustlean-driver");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn get_sysroot() -> Option<String> {
    let output = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}
