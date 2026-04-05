use std::collections::HashSet;

use rustc_hir::def_id::LocalDefId;
use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{BasicBlock, Body, Local, Operand, Place, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};

use crate::analysis::{AnalysisPass, Diagnostic, DiagnosticKind, Severity, resolve_location};
use crate::config::CostWeights;

pub struct CloneAnalysis {
    weights: CostWeights,
}

impl CloneAnalysis {
    pub fn new(weights: &CostWeights) -> Self {
        Self {
            weights: weights.clone(),
        }
    }

    fn is_clone_call(tcx: TyCtxt<'_>, func: &Operand<'_>) -> bool {
        if let Operand::Constant(constant) = func
            && let ty::FnDef(def_id, _) = *constant.const_.ty().kind()
        {
            let path = tcx.def_path_str(def_id);
            if path.contains("::clone") {
                // Check if this is actually the Clone trait method
                if let Some(trait_id) = tcx.trait_of_assoc(def_id) {
                    let trait_path = tcx.def_path_str(trait_id);
                    return trait_path.contains("Clone");
                }
                // Fallback: path-based detection
                // Note: Copy types' .clone() may be optimized away in MIR and won't appear here
                return path.ends_with("::clone");
            }
        }
        false
    }

    /// Check if the local variable backing the source place is used in any
    /// successor block after the clone call. Uses the MIR Visitor API for
    /// type-safe place comparison instead of Debug string matching.
    fn is_source_used_after(body: &Body<'_>, clone_bb: BasicBlock) -> bool {
        let terminator = body.basic_blocks[clone_bb].terminator();
        if let TerminatorKind::Call { args, .. } = &terminator.kind
            && let Some(first_arg) = args.first()
        {
            let source_local = match &first_arg.node {
                Operand::Copy(place) | Operand::Move(place) => Some(place.local),
                _ => None,
            };

            if let Some(local) = source_local {
                // Collect all reachable successor blocks
                let mut visited = HashSet::new();
                let mut worklist: Vec<BasicBlock> = terminator.successors().collect();

                while let Some(next_bb) = worklist.pop() {
                    if !visited.insert(next_bb) {
                        continue;
                    }

                    let block = &body.basic_blocks[next_bb];

                    // Use PlaceUseFinder to check if the local is used in this block
                    let mut finder = PlaceUseFinder {
                        target: local,
                        found_use: false,
                    };

                    // Visit statements
                    for stmt in &block.statements {
                        finder.visit_statement(
                            stmt,
                            rustc_middle::mir::Location {
                                block: next_bb,
                                statement_index: 0,
                            },
                        );
                        if finder.found_use {
                            return true;
                        }
                    }

                    // Visit terminator (skip Drop — dropping is not a "use")
                    let term = block.terminator();
                    if !matches!(term.kind, TerminatorKind::Drop { .. }) {
                        finder.visit_terminator(
                            term,
                            rustc_middle::mir::Location {
                                block: next_bb,
                                statement_index: block.statements.len(),
                            },
                        );
                        if finder.found_use {
                            return true;
                        }
                    }

                    worklist.extend(term.successors());
                }
            }
        }
        false
    }
}

/// MIR Visitor that checks if a specific Local is used (read/copied/moved).
struct PlaceUseFinder {
    target: Local,
    found_use: bool,
}

impl<'tcx> Visitor<'tcx> for PlaceUseFinder {
    fn visit_place(
        &mut self,
        place: &Place<'tcx>,
        context: PlaceContext,
        _location: rustc_middle::mir::Location,
    ) {
        if place.local == self.target && context.is_use() {
            self.found_use = true;
        }
    }
}

impl<'tcx> AnalysisPass<'tcx> for CloneAnalysis {
    fn name(&self) -> &'static str {
        "clone"
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
                if !Self::is_clone_call(tcx, func) {
                    continue;
                }

                let in_loop = loop_blocks.contains(&bb);
                let location = resolve_location(tcx, def_id, body, bb);
                let source_used = Self::is_source_used_after(body, bb);

                if !source_used {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::UnnecessaryClone,
                        severity: if in_loop {
                            Severity::Error
                        } else {
                            Severity::Warning
                        },
                        message: format!(
                            "Unnecessary `.clone()` in `{}`: source is not used after clone, move suffices",
                            location.function
                        ),
                        suggestion: Some("Remove `.clone()` to use move semantics".into()),
                        location,
                        base_cost: self.weights.clone_heap,
                        in_loop,
                    });
                }
            }
        }

        diagnostics
    }
}
