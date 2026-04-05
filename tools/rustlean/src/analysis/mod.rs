pub mod alloc;
pub mod clone;
pub mod layout;
pub mod loops;

use serde::Serialize;
use std::collections::HashSet;

use rustc_hir::def_id::LocalDefId;
use rustc_middle::mir::{BasicBlock, Body};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub severity: Severity,
    pub message: String,
    pub suggestion: Option<String>,
    pub location: Location,
    pub base_cost: f64,
    pub in_loop: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum DiagnosticKind {
    UnnecessaryClone,
    HeapAllocation,
    AllocationInLoop,
    VecReallocRisk,
    LargeStructMove,
    PaddingWaste,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub function: String,
}

/// Resolve a source location from a MIR basic block's terminator span.
/// Shared by all analysis passes.
pub fn resolve_location(
    tcx: TyCtxt<'_>,
    def_id: LocalDefId,
    body: &Body<'_>,
    bb: BasicBlock,
) -> Location {
    let span = body.basic_blocks[bb].terminator().source_info.span;
    let source_map = tcx.sess.source_map();
    let lo = source_map.lookup_char_pos(span.lo());
    let fn_name = tcx.def_path_str(def_id.to_def_id());

    Location {
        file: lo.file.name.prefer_local_unconditionally().to_string(),
        line: lo.line as u32,
        column: lo.col_display as u32,
        function: fn_name,
    }
}

/// Resolve a source location from a span and function name.
/// Used by layout analysis where BasicBlock is not available.
pub fn location_from_span(tcx: TyCtxt<'_>, span: Span, function: &str) -> Location {
    let source_map = tcx.sess.source_map();
    let lo = source_map.lookup_char_pos(span.lo());

    Location {
        file: lo.file.name.prefer_local_unconditionally().to_string(),
        line: lo.line as u32,
        column: lo.col_display as u32,
        function: function.to_string(),
    }
}

pub trait AnalysisPass<'tcx> {
    fn name(&self) -> &'static str;

    fn run_on_body(
        &self,
        tcx: TyCtxt<'tcx>,
        def_id: LocalDefId,
        body: &Body<'tcx>,
        loop_blocks: &HashSet<BasicBlock>,
    ) -> Vec<Diagnostic>;
}

impl std::fmt::Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnnecessaryClone => write!(f, "unnecessary-clone"),
            Self::HeapAllocation => write!(f, "heap-alloc"),
            Self::AllocationInLoop => write!(f, "alloc-in-loop"),
            Self::VecReallocRisk => write!(f, "vec-realloc"),
            Self::LargeStructMove => write!(f, "large-struct-move"),
            Self::PaddingWaste => write!(f, "padding-waste"),
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}
