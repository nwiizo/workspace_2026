use std::collections::{HashMap, HashSet, VecDeque};

use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::mir::{Body, TerminatorKind};
use rustc_middle::ty::TyCtxt;

/// A simple intra-crate call graph built from MIR Call terminators.
pub struct CallGraph {
    /// Maps caller -> set of callees (reserved for Phase 2: callee-direction analysis)
    #[expect(dead_code)]
    pub(crate) callees: HashMap<DefId, HashSet<DefId>>,
    /// Maps callee -> set of callers (reverse graph)
    pub(crate) callers: HashMap<DefId, HashSet<DefId>>,
}

/// DefKinds for which it is safe to query `optimized_mir`.
fn has_optimizable_mir(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(
        tcx.def_kind(def_id),
        DefKind::Fn | DefKind::AssocFn | DefKind::Closure
    )
}

impl CallGraph {
    pub fn build(tcx: TyCtxt<'_>) -> Self {
        let mut callees: HashMap<DefId, HashSet<DefId>> = HashMap::new();
        let mut callers: HashMap<DefId, HashSet<DefId>> = HashMap::new();

        for local_def_id in tcx.hir_body_owners() {
            let def_id = local_def_id.to_def_id();

            if !tcx.is_mir_available(def_id) || !has_optimizable_mir(tcx, def_id) {
                continue;
            }

            let body: &Body<'_> = tcx.optimized_mir(def_id);

            for bb_data in body.basic_blocks.iter() {
                if let Some(terminator) = &bb_data.terminator
                    && let TerminatorKind::Call { func, .. } = &terminator.kind
                    && let Some(callee_def_id) = resolve_call_target(func)
                {
                    callees.entry(def_id).or_default().insert(callee_def_id);
                    callers.entry(callee_def_id).or_default().insert(def_id);
                }
            }
        }

        Self { callees, callers }
    }

    /// BFS from `start` following reverse edges (callers).
    /// Returns all functions reachable within `max_depth` hops.
    pub fn callers_within_depth(&self, start: DefId, max_depth: usize) -> Vec<(DefId, usize)> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        visited.insert(start);
        queue.push_back((start, 0usize));

        while let Some((current, depth)) = queue.pop_front() {
            if depth > 0 {
                result.push((current, depth));
            }
            if depth >= max_depth {
                continue;
            }
            if let Some(callers_set) = self.callers.get(&current) {
                for &caller in callers_set {
                    if visited.insert(caller) {
                        queue.push_back((caller, depth + 1));
                    }
                }
            }
        }

        result
    }
}

/// Resolve a call target to a local DefId only.
fn resolve_call_target(func: &rustc_middle::mir::Operand<'_>) -> Option<DefId> {
    use rustc_middle::mir::Operand;
    use rustc_middle::ty::TyKind;

    match func {
        Operand::Constant(constant) => {
            let ty = constant.const_.ty();
            match ty.kind() {
                TyKind::FnDef(def_id, _) if def_id.is_local() => Some(*def_id),
                _ => None,
            }
        }
        Operand::Copy(_) | Operand::Move(_) | Operand::RuntimeChecks(_) => None,
    }
}
