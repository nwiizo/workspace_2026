use std::process::{ExitCode, Termination};

fn main() -> ExitCode {
    cargo_agent_context::main_entry(std::env::args()).report()
}
