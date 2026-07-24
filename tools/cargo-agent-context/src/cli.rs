use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Markdown,
    AgentsMd,
    ClaudeMd,
}

#[derive(Debug, Parser)]
#[command(
    name = "cargo-agent-context",
    about = "Summarize Rust repository context for AI coding agents",
    version
)]
pub struct Cli {
    /// Path to a Rust crate or workspace root.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    pub format: OutputFormat,

    /// Write markdown to a file instead of stdout.
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Directory containing pre-generated sibling JSON reports.
    #[arg(long, value_name = "DIR", conflicts_with = "run")]
    pub from: Option<PathBuf>,

    /// Run sibling tools with --json when available.
    #[arg(long, conflicts_with = "from")]
    pub run: bool,

    /// Limit public API rows.
    #[arg(long, default_value_t = 30)]
    pub top: usize,

    /// Japanese output.
    #[arg(long, visible_alias = "jp")]
    pub japanese: bool,
}

impl Cli {
    pub fn parse<I, T>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let normalized = design_gate_core::absorb_cargo_subcommand(
            args.into_iter()
                .map(|arg| arg.into().to_string_lossy().into_owned()),
            "agent-context",
        );
        Self::try_parse_from(normalized).map_err(|err| Error::Usage(err.to_string()))
    }
}
