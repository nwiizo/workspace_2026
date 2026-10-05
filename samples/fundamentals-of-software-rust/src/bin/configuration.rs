use fundamentals_of_software_rust::config::AppConfig;
use std::process::ExitCode;

fn main() -> ExitCode {
    match AppConfig::from_env() {
        Ok(config) => {
            println!("Configured port: {}", config.port);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Configuration error: {error}");
            ExitCode::FAILURE
        }
    }
}
