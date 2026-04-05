pub mod call_graph;
pub mod unsafe_impact;

use std::collections::HashMap;

use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

use crate::config::RustGuardConfig;
use crate::diagnostics::{Category, Finding, Severity};

/// Summary statistics for the analysis run.
#[derive(Debug, Clone, Default)]
pub struct AnalysisSummary {
    pub total_findings: usize,
    pub suppressed_count: usize,
    pub by_severity: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
    pub unsafe_fn_count: usize,
    pub unsafe_block_count: usize,
    pub unsafe_reach_max_depth: usize,
    pub safety_comment_present: usize,
    pub safety_comment_missing: usize,
}

impl AnalysisSummary {
    pub fn safety_comment_coverage(&self) -> f64 {
        let total = self.safety_comment_present + self.safety_comment_missing;
        if total == 0 {
            100.0
        } else {
            (self.safety_comment_present as f64 / total as f64) * 100.0
        }
    }

    pub fn has_errors(&self) -> bool {
        self.by_severity.get("error").copied().unwrap_or(0) > 0
    }
}

pub struct AnalysisResult {
    pub findings: Vec<Finding>,
    pub summary: AnalysisSummary,
}

/// A finding with its original span preserved for suppression checking.
struct SpannedFinding {
    finding: Finding,
    span: Span,
}

/// Run all enabled analyses on the current crate.
pub fn run_analysis<'tcx>(tcx: TyCtxt<'tcx>, config: &RustGuardConfig) -> AnalysisResult {
    let mut spanned: Vec<SpannedFinding> = Vec::new();

    if config.rules.r#unsafe.enabled {
        spanned.extend(
            unsafe_impact::analyze_crate(tcx, &config.rules.r#unsafe)
                .into_iter()
                .map(|(f, s)| SpannedFinding {
                    finding: f,
                    span: s,
                }),
        );

        let cg = call_graph::CallGraph::build(tcx);
        spanned.extend(
            unsafe_impact::analyze_unsafe_reach(tcx, &config.rules.r#unsafe, &cg)
                .into_iter()
                .map(|(f, s)| SpannedFinding {
                    finding: f,
                    span: s,
                }),
        );
    }

    // Filter out suppressed findings
    let pre_filter_count = spanned.len();
    spanned.retain(|sf| !unsafe_impact::is_suppressed(tcx, sf.span, &sf.finding.rule_id));
    let suppressed_count = pre_filter_count - spanned.len();

    let mut findings: Vec<Finding> = spanned.into_iter().map(|sf| sf.finding).collect();

    // Sort by file, then line
    findings.sort_by(|a, b| {
        a.location
            .file
            .cmp(&b.location.file)
            .then(a.location.line.cmp(&b.location.line))
    });

    // Build summary
    let mut summary = AnalysisSummary {
        total_findings: findings.len(),
        suppressed_count,
        ..Default::default()
    };

    for f in &findings {
        *summary
            .by_severity
            .entry(f.severity.to_string())
            .or_default() += 1;
        *summary
            .by_category
            .entry(f.category.to_string())
            .or_default() += 1;

        match f.category {
            Category::UnsafeFunction => summary.unsafe_fn_count += 1,
            Category::UnsafeBlock => {
                summary.unsafe_block_count += 1;
                // After severity fix: Info = has SAFETY comment, Warning = missing
                if f.severity == Severity::Info {
                    summary.safety_comment_present += 1;
                } else {
                    summary.safety_comment_missing += 1;
                }
            }
            Category::UnsafeReach => {
                if let Some(ref reach) = f.unsafe_reach {
                    let depth = reach.affected_functions.len();
                    if depth > summary.unsafe_reach_max_depth {
                        summary.unsafe_reach_max_depth = depth;
                    }
                }
            }
            _ => {}
        }
    }

    AnalysisResult { findings, summary }
}
