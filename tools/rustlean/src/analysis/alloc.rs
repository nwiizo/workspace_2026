use std::collections::HashSet;

use rustc_hir::def_id::LocalDefId;
use rustc_middle::mir::{BasicBlock, Body, Operand, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};

use crate::analysis::{AnalysisPass, Diagnostic, DiagnosticKind, Severity, resolve_location};
use crate::config::CostWeights;

#[derive(Debug, Clone, Copy)]
enum AllocKind {
    BoxNew,
    VecNew,
    VecPush,
    StringAlloc,
    FormatMacro,
    RawAlloc,
}

impl AllocKind {
    /// The diagnostic kind when this allocation is NOT inside a loop.
    fn non_loop_kind(self) -> DiagnosticKind {
        match self {
            Self::VecPush => DiagnosticKind::VecReallocRisk,
            _ => DiagnosticKind::HeapAllocation,
        }
    }

    fn base_cost(self, weights: &CostWeights) -> f64 {
        match self {
            Self::VecPush => weights.vec_push,
            Self::StringAlloc => weights.string_alloc,
            Self::FormatMacro => weights.format_macro,
            Self::BoxNew | Self::VecNew | Self::RawAlloc => weights.heap_alloc,
        }
    }

    fn description(self, fn_name: &str) -> String {
        match self {
            Self::BoxNew => format!("Heap allocation via `Box::new` in `{fn_name}`"),
            Self::VecNew => format!("Heap allocation via `Vec::new` in `{fn_name}`"),
            Self::VecPush => format!("`Vec::push` in `{fn_name}` may trigger reallocation"),
            Self::StringAlloc => format!("String allocation in `{fn_name}`"),
            Self::FormatMacro => {
                format!("`format!` macro allocates a new String in `{fn_name}`")
            }
            Self::RawAlloc => format!("Low-level heap allocation in `{fn_name}`"),
        }
    }

    fn suggestion(self) -> Option<String> {
        match self {
            Self::VecPush => {
                Some("Consider `Vec::with_capacity()` if the size is known in advance".into())
            }
            Self::VecNew => {
                Some("If used in a loop, consider hoisting the allocation outside".into())
            }
            Self::StringAlloc => Some("Consider using `&str` if ownership is not needed".into()),
            Self::FormatMacro => {
                Some("Consider `write!` to an existing buffer if called repeatedly".into())
            }
            Self::BoxNew | Self::RawAlloc => None,
        }
    }
}

pub struct AllocAnalysis {
    weights: CostWeights,
}

impl AllocAnalysis {
    pub fn new(weights: &CostWeights) -> Self {
        Self {
            weights: weights.clone(),
        }
    }

    fn classify_alloc_call(tcx: TyCtxt<'_>, func: &Operand<'_>) -> Option<AllocKind> {
        if let Operand::Constant(constant) = func
            && let ty::FnDef(def_id, _) = *constant.const_.ty().kind()
        {
            let path = tcx.def_path_str(def_id);

            if path.contains("Box") && path.contains("new") {
                return Some(AllocKind::BoxNew);
            }
            if path.contains("Vec") {
                // Skip with_capacity — it's a best practice, not a problem
                if path.contains("new") && !path.contains("with_capacity") {
                    return Some(AllocKind::VecNew);
                }
                if path.contains("push") {
                    return Some(AllocKind::VecPush);
                }
            }
            if path.contains("String::from")
                || path.contains("to_string")
                || path.contains("to_owned")
            {
                return Some(AllocKind::StringAlloc);
            }
            if path.contains("fmt::format") {
                return Some(AllocKind::FormatMacro);
            }
            if path.contains("__rust_alloc") || path.contains("exchange_malloc") {
                return Some(AllocKind::RawAlloc);
            }
        }
        None
    }
}

impl<'tcx> AnalysisPass<'tcx> for AllocAnalysis {
    fn name(&self) -> &'static str {
        "alloc"
    }

    fn run_on_body(
        &self,
        tcx: TyCtxt<'tcx>,
        def_id: LocalDefId,
        body: &Body<'tcx>,
        loop_blocks: &HashSet<BasicBlock>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (bb, block_data) in body.basic_blocks.iter_enumerated() {
            let terminator = block_data.terminator();

            if let TerminatorKind::Call { func, .. } = &terminator.kind {
                let Some(alloc_kind) = Self::classify_alloc_call(tcx, func) else {
                    continue;
                };

                let in_loop = loop_blocks.contains(&bb);
                let location = resolve_location(tcx, def_id, body, bb);

                let kind = if in_loop {
                    DiagnosticKind::AllocationInLoop
                } else {
                    alloc_kind.non_loop_kind()
                };

                diagnostics.push(Diagnostic {
                    kind,
                    severity: if in_loop {
                        Severity::Error
                    } else {
                        Severity::Info
                    },
                    message: alloc_kind.description(&location.function),
                    suggestion: alloc_kind.suggestion(),
                    location,
                    base_cost: alloc_kind.base_cost(&self.weights),
                    in_loop,
                });
            }
        }

        diagnostics
    }
}
