#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod callbacks;
mod mir_visitor;

use std::env;
use std::process::ExitCode;

use callbacks::{PassthroughCallback, ProbeCallback};

fn main() -> ExitCode {
    rustc_driver::install_ice_hook("https://github.com/nwiizo/workspace_2026/issues", |_| ());

    let mut args: Vec<String> = env::args().collect();

    // When used as RUSTC_WORKSPACE_WRAPPER, cargo passes:
    //   [driver_path, original_rustc_path, actual_args...]
    if args.len() > 1 && args[1].ends_with("rustc") {
        args.remove(1);
    }

    let should_analyze = should_analyze_crate(&args);

    let output_dir = env::var("RUSTPROBE_OUTPUT_DIR").unwrap_or_else(|_| {
        let target = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
        format!("{target}/rustprobe")
    });

    if !args.iter().any(|a| a == "--sysroot")
        && let Ok(output) = std::process::Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
    {
        let sysroot = String::from_utf8_lossy(&output.stdout).trim().to_string();
        args.push("--sysroot".into());
        args.push(sysroot);
    }

    let mut probe_cb = ProbeCallback::new(output_dir);
    let mut passthrough_cb = PassthroughCallback;
    let callbacks: &mut (dyn rustc_driver::Callbacks + Send) = if should_analyze {
        &mut probe_cb
    } else {
        &mut passthrough_cb
    };

    let result = rustc_driver::catch_fatal_errors(|| {
        rustc_driver::run_compiler(&args, callbacks);
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn should_analyze_crate(args: &[String]) -> bool {
    if args
        .iter()
        .any(|a| a == "--version" || a == "--print" || a == "-vV")
    {
        return false;
    }

    if let Some(pos) = args.iter().position(|a| a == "--crate-name")
        && let Some(name) = args.get(pos + 1)
        && name.starts_with("build_script_")
    {
        return false;
    }

    if let Some(pos) = args.iter().position(|a| a == "--crate-type")
        && let Some(ty) = args.get(pos + 1)
        && ty == "proc-macro"
    {
        return false;
    }

    true
}
