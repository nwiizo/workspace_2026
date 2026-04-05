use std::collections::HashSet;

use rustc_hir::def_id::LocalDefId;
use rustc_middle::mir::{BasicBlock, Body};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

use crate::analysis::{AnalysisPass, Diagnostic, DiagnosticKind, Severity, location_from_span};
use crate::config::{CostWeights, Thresholds};

pub struct LayoutAnalysis {
    thresholds: Thresholds,
    weights: CostWeights,
}

impl LayoutAnalysis {
    pub fn new(thresholds: &Thresholds, weights: &CostWeights) -> Self {
        Self {
            thresholds: thresholds.clone(),
            weights: weights.clone(),
        }
    }
}

impl<'tcx> AnalysisPass<'tcx> for LayoutAnalysis {
    fn name(&self) -> &'static str {
        "layout"
    }

    fn run_on_body(
        &self,
        tcx: TyCtxt<'tcx>,
        def_id: LocalDefId,
        body: &Body<'tcx>,
        _loop_blocks: &HashSet<BasicBlock>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let fn_name = tcx.def_path_str(def_id.to_def_id());

        // Track already-analyzed struct DefIds to avoid duplicate diagnostics
        let mut analyzed_structs: HashSet<DefId> = HashSet::new();

        for local_decl in body.local_decls.iter() {
            let local_ty = local_decl.ty;

            if let ty::Adt(adt_def, _) = local_ty.kind() {
                if !adt_def.is_struct() || !analyzed_structs.insert(adt_def.did()) {
                    continue;
                }

                let Ok(layout) = tcx
                    .layout_of(ty::TypingEnv::post_analysis(tcx, def_id).as_query_input(local_ty))
                else {
                    continue;
                };

                let size = layout.size.bytes() as usize;
                let span = local_decl.source_info.span;

                // Large struct warning
                if size > self.thresholds.large_struct_bytes {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::LargeStructMove,
                        severity: Severity::Warning,
                        message: format!(
                            "Large struct `{}` ({size} bytes) used as local in `{fn_name}`, consider `Box<T>`",
                            tcx.def_path_str(adt_def.did())
                        ),
                        suggestion: Some(format!(
                            "Wrap in `Box<{}>` to avoid large stack moves",
                            tcx.def_path_str(adt_def.did())
                        )),
                        location: location_from_span(tcx, span, &fn_name),
                        base_cost: self.weights.large_struct_move
                            * (size as f64 / self.thresholds.large_struct_bytes as f64),
                        in_loop: false,
                    });
                }

                // Padding waste detection (only for non-repr(C) structs)
                if !adt_def.repr().c() {
                    let fields: Vec<u64> = adt_def
                        .all_fields()
                        .filter_map(|f| {
                            let field_ty = tcx.type_of(f.did).instantiate_identity();
                            tcx.layout_of(
                                ty::TypingEnv::post_analysis(tcx, def_id).as_query_input(field_ty),
                            )
                            .ok()
                            .map(|l| l.size.bytes())
                        })
                        .collect();

                    let sum_field_sizes: u64 = fields.iter().sum();
                    let actual_size = layout.size.bytes();

                    if sum_field_sizes > 0 && actual_size > sum_field_sizes {
                        let waste = actual_size - sum_field_sizes;
                        let waste_pct = (waste as f64 / actual_size as f64) * 100.0;

                        if waste_pct > self.thresholds.padding_waste_percent {
                            diagnostics.push(Diagnostic {
                                kind: DiagnosticKind::PaddingWaste,
                                severity: Severity::Info,
                                message: format!(
                                    "Struct `{}` has {waste} bytes padding ({waste_pct:.1}% waste), actual size: {actual_size}B, field sum: {sum_field_sizes}B",
                                    tcx.def_path_str(adt_def.did())
                                ),
                                suggestion: Some(
                                    "Consider reordering fields by descending alignment to reduce padding".into()
                                ),
                                location: location_from_span(tcx, span, &fn_name),
                                base_cost: waste as f64 * self.weights.padding_waste_per_byte,
                                in_loop: false,
                            });
                        }
                    }
                }
            }
        }

        diagnostics
    }
}
