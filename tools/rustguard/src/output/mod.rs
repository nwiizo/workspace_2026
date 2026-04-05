pub mod json;
pub mod sarif;
pub mod text;

use crate::analysis::AnalysisSummary;
use crate::config::OutputFormat;
use crate::diagnostics::Finding;
use crate::error::Result;

pub fn render(
    findings: &[Finding],
    summary: &AnalysisSummary,
    format: OutputFormat,
) -> Result<String> {
    match format {
        OutputFormat::Text => Ok(text::render(findings, summary)),
        OutputFormat::Json => json::render(findings, summary),
        OutputFormat::Sarif => sarif::render(findings, summary),
    }
}
