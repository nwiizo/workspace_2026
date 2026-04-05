use std::collections::HashSet;

use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{self as hir, BlockCheckMode, UnsafeSource};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

use crate::config::rules::UnsafeRulesConfig;
use crate::diagnostics::{Category, Finding, Severity, SourceLocation, UnsafeReachInfo};

use super::call_graph::CallGraph;

struct UnsafeBlockVisitor {
    unsafe_spans: Vec<Span>,
}

impl<'tcx> Visitor<'tcx> for UnsafeBlockVisitor {
    fn visit_block(&mut self, block: &'tcx hir::Block<'tcx>) {
        if let BlockCheckMode::UnsafeBlock(UnsafeSource::UserProvided) = block.rules {
            self.unsafe_spans.push(block.span);
        }
        intravisit::walk_block(self, block);
    }
}

/// Collect DefIds of all functions that contain unsafe code (unsafe fn or unsafe blocks).
pub(crate) fn collect_unsafe_def_ids(tcx: TyCtxt<'_>) -> HashSet<DefId> {
    let mut result = HashSet::new();

    for local_def_id in tcx.hir_body_owners() {
        let def_id = local_def_id.to_def_id();
        let hir_id = tcx.local_def_id_to_hir_id(local_def_id);

        let is_unsafe_fn = tcx
            .hir_fn_sig_by_hir_id(hir_id)
            .is_some_and(|sig| sig.header.is_unsafe());

        let has_unsafe_block = tcx
            .hir_maybe_body_owned_by(local_def_id)
            .is_some_and(|body| {
                let mut visitor = UnsafeBlockVisitor {
                    unsafe_spans: Vec::new(),
                };
                visitor.visit_body(body);
                !visitor.unsafe_spans.is_empty()
            });

        if is_unsafe_fn || has_unsafe_block {
            result.insert(def_id);
        }
    }

    result
}

/// Analyze a single crate for unsafe blocks and unsafe functions.
/// Returns findings paired with their original spans (for suppression checking),
/// plus the set of DefIds containing unsafe code (for reuse in reach analysis).
pub(crate) fn analyze_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    config: &UnsafeRulesConfig,
) -> Vec<(Finding, Span)> {
    let mut findings = Vec::new();

    for local_def_id in tcx.hir_body_owners() {
        let def_id = local_def_id.to_def_id();
        let hir_id = tcx.local_def_id_to_hir_id(local_def_id);

        // Check if the function itself is `unsafe fn`
        if let Some(fn_sig) = tcx.hir_fn_sig_by_hir_id(hir_id)
            && fn_sig.header.is_unsafe()
        {
            let span = tcx.def_span(def_id);
            let fn_name = tcx.def_path_str(def_id);
            findings.push((
                Finding {
                    rule_id: "RG001",
                    severity: Severity::Info,
                    category: Category::UnsafeFunction,
                    message: format!("function `{fn_name}` is declared as `unsafe`"),
                    location: span_to_location(tcx, span),
                    related_locations: vec![],
                    suggestion: None,
                    unsafe_reach: None,
                    has_safety_comment: false,
                },
                span,
            ));
        }

        // Visit the HIR body to find unsafe blocks
        let Some(body) = tcx.hir_maybe_body_owned_by(local_def_id) else {
            continue;
        };
        let mut visitor = UnsafeBlockVisitor {
            unsafe_spans: Vec::new(),
        };
        visitor.visit_body(body);

        let fn_name = tcx.def_path_str(def_id);
        for span in visitor.unsafe_spans {
            let has_comment = has_safety_comment(tcx, span);
            let (severity, suggestion) = if config.require_safety_comment && !has_comment {
                (
                    Severity::Warning,
                    Some("add a `// SAFETY: ...` comment explaining why this is safe".to_string()),
                )
            } else {
                (Severity::Info, None)
            };

            findings.push((
                Finding {
                    rule_id: "RG002",
                    severity,
                    category: Category::UnsafeBlock,
                    message: format!("unsafe block in function `{fn_name}`"),
                    location: span_to_location(tcx, span),
                    related_locations: vec![],
                    suggestion,
                    unsafe_reach: None,
                    has_safety_comment: has_comment,
                },
                span,
            ));
        }
    }

    findings
}

/// Analyze unsafe reach: trace which functions transitively call unsafe code
/// via the intra-crate call graph.
pub(crate) fn analyze_unsafe_reach<'tcx>(
    tcx: TyCtxt<'tcx>,
    config: &UnsafeRulesConfig,
    call_graph: &CallGraph,
    unsafe_def_ids: &HashSet<DefId>,
) -> Vec<(Finding, Span)> {
    let mut findings = Vec::new();
    let max_depth = config.max_unsafe_reach.unwrap_or(10);
    let warn_threshold = config.warn_unsafe_reach.unwrap_or(5);

    for &unsafe_def_id in unsafe_def_ids {
        let reachable = call_graph.callers_within_depth(unsafe_def_id, max_depth);
        if reachable.is_empty() {
            continue;
        }

        let unsafe_name = tcx.def_path_str(unsafe_def_id);
        let unsafe_span = tcx.def_span(unsafe_def_id);

        let affected: Vec<String> = reachable
            .iter()
            .map(|(def_id, _)| tcx.def_path_str(*def_id))
            .collect();

        let mut call_chain = vec![unsafe_name.clone()];
        for (def_id, _) in reachable.iter().take(5) {
            call_chain.push(tcx.def_path_str(*def_id));
        }

        let severity = match config.max_unsafe_reach {
            Some(max) if reachable.len() > max => Severity::Error,
            _ if reachable.len() > warn_threshold => Severity::Warning,
            _ => Severity::Info,
        };

        findings.push((
            Finding {
                rule_id: "RG003",
                severity,
                category: Category::UnsafeReach,
                message: format!(
                    "unsafe code in `{unsafe_name}` is reachable from {} function(s)",
                    reachable.len()
                ),
                location: span_to_location(tcx, unsafe_span),
                related_locations: reachable
                    .iter()
                    .take(10)
                    .map(|(def_id, _)| span_to_location(tcx, tcx.def_span(*def_id)))
                    .collect(),
                suggestion: None,
                unsafe_reach: Some(UnsafeReachInfo {
                    unsafe_location: span_to_location(tcx, unsafe_span),
                    affected_functions: affected,
                    call_chain,
                }),
                has_safety_comment: false,
            },
            unsafe_span,
        ));
    }

    findings
}

pub(crate) fn span_to_location(tcx: TyCtxt<'_>, span: Span) -> SourceLocation {
    let source_map = tcx.sess.source_map();
    let lo = source_map.lookup_char_pos(span.lo());
    let hi = source_map.lookup_char_pos(span.hi());

    let snippet = source_map.span_to_snippet(span).ok().map(|s| {
        let trimmed = s.trim();
        if trimmed.len() <= 200 {
            trimmed.to_string()
        } else {
            // Use floor_char_boundary to avoid panicking on multi-byte UTF-8
            let boundary = trimmed.floor_char_boundary(200);
            format!("{}...", &trimmed[..boundary])
        }
    });

    SourceLocation {
        file: lo
            .file
            .name
            .prefer_local_unconditionally()
            .to_string()
            .into(),
        line: lo.line,
        column: lo.col_display + 1,
        end_line: Some(hi.line),
        end_column: Some(hi.col_display + 1),
        snippet,
    }
}

fn get_preceding_source(tcx: TyCtxt<'_>, span: Span, bytes_back: u32) -> Option<String> {
    let source_map = tcx.sess.source_map();
    let lo = span.lo();
    let search_start = rustc_span::BytePos(lo.0.saturating_sub(bytes_back));
    let search_span = span.with_lo(search_start).with_hi(lo);
    source_map.span_to_snippet(search_span).ok()
}

/// Iterate preceding lines in reverse, stopping at the first blank line.
fn preceding_lines_until_blank(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .rev()
        .take_while(|line| !line.trim().is_empty())
        .map(|line| line.trim())
}

fn has_safety_comment(tcx: TyCtxt<'_>, span: Span) -> bool {
    let Some(source) = get_preceding_source(tcx, span, 300) else {
        return false;
    };
    for trimmed in preceding_lines_until_blank(&source) {
        let upper = trimmed.to_uppercase();
        if upper.contains("// SAFETY:") || upper.contains("/// SAFETY") {
            return true;
        }
    }
    false
}

/// Check if a span is preceded by a `// rustguard::allow(RULE_ID)` comment.
/// Stops at blank lines to avoid cross-function suppression.
pub(crate) fn is_suppressed(tcx: TyCtxt<'_>, span: Span, rule_id: &str) -> bool {
    let Some(source) = get_preceding_source(tcx, span, 300) else {
        return false;
    };
    for trimmed in preceding_lines_until_blank(&source) {
        if let Some(rest) = trimmed.strip_prefix("// rustguard::allow(")
            && let Some(rules_str) = rest.strip_suffix(')')
        {
            let rules: Vec<&str> = rules_str.split(',').map(|s| s.trim()).collect();
            if rules.iter().any(|r| *r == rule_id || *r == "*") {
                return true;
            }
        }
    }
    false
}
