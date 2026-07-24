mod cli;
mod error;
mod metadata;
mod report;
mod sibling;
mod source;

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;

use cli::Cli;
pub use error::{Error, Result};

pub fn main_entry<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match try_main(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            match err {
                Error::Usage(_) => ExitCode::from(design_gate_core::USAGE_ERROR_EXIT as u8),
                _ => ExitCode::from(1),
            }
        }
    }
}

pub fn try_main<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse(args)?;
    let analysis = metadata::analyze_project(&cli.path)?;
    let source = source::analyze_source(&analysis.root, cli.top)?;
    let risks = sibling::collect_sibling_reports(&analysis.root, cli.from.as_deref(), cli.run)?;
    let markdown = report::render(&analysis, &source, &risks, cli.format, cli.japanese)?;
    match cli.output {
        Some(path) => fs::write(&path, markdown).map_err(|source| Error::WriteFile {
            path: path.clone(),
            source,
        }),
        None => {
            let mut stdout = io::stdout().lock();
            stdout.write_all(markdown.as_bytes())?;
            Ok(())
        }
    }
}
