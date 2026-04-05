use std::collections::HashMap;

use serde::Serialize;

use crate::analysis::AnalysisSummary;
use crate::diagnostics::Finding;
use crate::error::Result;

#[derive(Serialize)]
struct JsonReport<'a> {
    tool: ToolInfo,
    summary: JsonSummary<'a>,
    findings: &'a [Finding],
}

#[derive(Serialize)]
struct ToolInfo {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct JsonSummary<'a> {
    total: usize,
    suppressed: usize,
    errors: usize,
    warnings: usize,
    infos: usize,
    by_category: &'a HashMap<String, usize>,
    unsafe_functions: usize,
    unsafe_blocks: usize,
    unsafe_reach_max_depth: usize,
    safety_comment_coverage_percent: f64,
}

pub fn render(findings: &[Finding], summary: &AnalysisSummary) -> Result<String> {
    let report = JsonReport {
        tool: ToolInfo {
            name: "rustguard",
            version: env!("CARGO_PKG_VERSION"),
        },
        summary: JsonSummary {
            total: summary.total_findings,
            suppressed: summary.suppressed_count,
            errors: summary.by_severity.get("error").copied().unwrap_or(0),
            warnings: summary.by_severity.get("warning").copied().unwrap_or(0),
            infos: summary.by_severity.get("info").copied().unwrap_or(0),
            by_category: &summary.by_category,
            unsafe_functions: summary.unsafe_fn_count,
            unsafe_blocks: summary.unsafe_block_count,
            unsafe_reach_max_depth: summary.unsafe_reach_max_depth,
            safety_comment_coverage_percent: summary.safety_comment_coverage(),
        },
        findings,
    };
    let json = serde_json::to_string_pretty(&report)?;
    Ok(json)
}
