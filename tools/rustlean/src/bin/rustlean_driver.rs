#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;

use std::env;
use std::process::ExitCode;

use rustlean::config::{OutputFormat, RustLeanConfig};
use rustlean::cost::compute_scores;
use rustlean::driver::RustLeanCallbacks;
use rustlean::report::AnalysisReport;

fn main() -> ExitCode {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Load config from environment variable or defaults
    let config = match env::var("RUSTLEAN_CONFIG") {
        Ok(path) => RustLeanConfig::load(std::path::Path::new(&path)).unwrap_or_else(|e| {
            eprintln!("rustlean: warning: failed to load config: {e}, using defaults");
            RustLeanConfig::default()
        }),
        Err(_) => RustLeanConfig::load_or_default(),
    };

    let output_format = env::var("RUSTLEAN_FORMAT")
        .ok()
        .and_then(|f| match f.as_str() {
            "json" => Some(OutputFormat::Json),
            "text" => Some(OutputFormat::Text),
            _ => None,
        })
        .unwrap_or(config.output);

    // Collect rustc arguments
    let mut args: Vec<String> = env::args().collect();

    // If invoked as RUSTC_WRAPPER, args[1] is the path to rustc.
    // Detect this by checking if args[1] looks like a rustc binary path.
    if args.len() > 1 {
        let second_arg = &args[1];
        if second_arg.ends_with("rustc")
            || second_arg.contains("rustc") && !second_arg.starts_with("-")
        {
            args.remove(1);
        }
    }

    // Ensure sysroot is set
    if !args.iter().any(|a| a == "--sysroot")
        && let Ok(output) = std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
    {
        let sysroot = String::from_utf8_lossy(&output.stdout).trim().to_string();
        args.push("--sysroot".into());
        args.push(sysroot);
    }

    let mut callbacks = RustLeanCallbacks::new(config.clone());

    let crate_name = extract_crate_name(&args);

    // Run the compiler with our callbacks
    let compiler_result = rustc_driver::catch_fatal_errors(|| {
        rustc_driver::run_compiler(&args, &mut callbacks);
    });

    // Collect diagnostics and generate report
    let diagnostics = callbacks.take_diagnostics();

    if !diagnostics.is_empty() {
        let score = compute_scores(&diagnostics, &config.cost_weights);
        let report = AnalysisReport {
            crate_name,
            diagnostics,
            score,
        };

        match report.render(output_format) {
            Ok(output) => eprintln!("{output}"),
            Err(e) => eprintln!("rustlean: error rendering report: {e}"),
        }
    }

    match compiler_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn extract_crate_name(args: &[String]) -> String {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--crate-name"
            && let Some(name) = args.get(i + 1)
        {
            return name.clone();
        }
    }
    "unknown".into()
}
