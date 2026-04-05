use colored::Colorize;

use crate::analysis::AnalysisSummary;
use crate::diagnostics::{Finding, Severity};

pub fn render(findings: &[Finding], summary: &AnalysisSummary) -> String {
    if findings.is_empty() && summary.suppressed_count == 0 {
        return "No findings.".to_string();
    }

    let mut output = String::new();

    for finding in findings {
        let severity_str = match finding.severity {
            Severity::Error => "error".red().bold().to_string(),
            Severity::Warning => "warning".yellow().bold().to_string(),
            Severity::Info => "info".blue().bold().to_string(),
        };

        output.push_str(&format!(
            "{severity_str}[{}]: {}\n",
            finding.category, finding.message,
        ));
        output.push_str(&format!("  {} {}\n", "-->".blue().bold(), finding.location,));

        if let Some(snippet) = &finding.location.snippet {
            for line in snippet.lines() {
                output.push_str(&format!("  {} {line}\n", "|".blue().bold()));
            }
        }

        if let Some(suggestion) = &finding.suggestion {
            output.push_str(&format!("  {} {suggestion}\n", "help:".green().bold(),));
        }

        if let Some(reach) = &finding.unsafe_reach {
            if !reach.call_chain.is_empty() {
                output.push_str(&format!(
                    "  {} call chain: {}\n",
                    "note:".cyan().bold(),
                    reach.call_chain.join(" -> "),
                ));
            }
            if !reach.affected_functions.is_empty() {
                output.push_str(&format!(
                    "  {} affects {} function(s)\n",
                    "note:".cyan().bold(),
                    reach.affected_functions.len(),
                ));
            }
        }

        output.push('\n');
    }

    // Enhanced summary
    output.push_str(&format!("{}\n", "=== Summary ===".bold()));

    let errors = summary.by_severity.get("error").copied().unwrap_or(0);
    let warnings = summary.by_severity.get("warning").copied().unwrap_or(0);
    let infos = summary.by_severity.get("info").copied().unwrap_or(0);

    output.push_str(&format!(
        "  Findings: {} total ({} error, {} warning, {} info)\n",
        summary.total_findings, errors, warnings, infos,
    ));

    if summary.suppressed_count > 0 {
        output.push_str(&format!(
            "  Suppressed: {} (via rustguard::allow)\n",
            summary.suppressed_count,
        ));
    }

    if summary.unsafe_fn_count > 0 || summary.unsafe_block_count > 0 {
        output.push_str(&format!(
            "  Unsafe: {} function(s), {} block(s)\n",
            summary.unsafe_fn_count, summary.unsafe_block_count,
        ));
    }

    if summary.unsafe_reach_max_depth > 0 {
        output.push_str(&format!(
            "  Max unsafe reach: {} function(s) deep\n",
            summary.unsafe_reach_max_depth,
        ));
    }

    let total_blocks = summary.safety_comment_present + summary.safety_comment_missing;
    if total_blocks > 0 {
        output.push_str(&format!(
            "  SAFETY comments: {}/{} ({:.0}% coverage)\n",
            summary.safety_comment_present,
            total_blocks,
            summary.safety_comment_coverage(),
        ));
    }

    output
}
