use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use rustc_driver::Compilation;
use rustc_hir::def::DefKind;
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;

use crate::analysis::alloc::AllocAnalysis;
use crate::analysis::clone::CloneAnalysis;
use crate::analysis::layout::LayoutAnalysis;
use crate::analysis::loops::detect_loop_blocks;
use crate::analysis::{AnalysisPass, Diagnostic, DiagnosticKind};
use crate::config::RustLeanConfig;

pub struct RustLeanCallbacks {
    pub config: RustLeanConfig,
    pub diagnostics: Arc<Mutex<Vec<Diagnostic>>>,
}

impl RustLeanCallbacks {
    pub fn new(config: RustLeanConfig) -> Self {
        Self {
            config,
            diagnostics: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn take_diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics
            .lock()
            .map(|mut d| std::mem::take(&mut *d))
            .unwrap_or_default()
    }
}

impl rustc_driver::Callbacks for RustLeanCallbacks {
    fn after_analysis<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        let passes: Vec<Box<dyn AnalysisPass<'tcx>>> = vec![
            Box::new(CloneAnalysis::new(&self.config.cost_weights)),
            Box::new(AllocAnalysis::new(&self.config.cost_weights)),
            Box::new(LayoutAnalysis::new(
                &self.config.thresholds,
                &self.config.cost_weights,
            )),
        ];

        let ignore_patterns = &self.config.ignore_paths;
        let mut all_diagnostics = Vec::new();

        for local_def_id in tcx.hir_body_owners() {
            // Skip constants — optimized_mir panics for const items
            let def_kind = tcx.def_kind(local_def_id);
            if matches!(
                def_kind,
                DefKind::Const { .. }
                    | DefKind::AssocConst { .. }
                    | DefKind::AnonConst
                    | DefKind::InlineConst
                    | DefKind::Static { .. }
            ) {
                continue;
            }

            if !tcx.is_mir_available(local_def_id) {
                continue;
            }

            let body = tcx.optimized_mir(local_def_id);
            let loop_blocks = detect_loop_blocks(body);

            for pass in &passes {
                let mut diags = pass.run_on_body(tcx, local_def_id, body, &loop_blocks);
                all_diagnostics.append(&mut diags);
            }
        }

        // Filter by ignore_paths
        if !ignore_patterns.is_empty() {
            all_diagnostics.retain(|d| {
                !ignore_patterns
                    .iter()
                    .any(|pat| d.location.file.contains(pat.trim_matches('*')))
            });
        }

        // Deduplicate layout diagnostics (same kind + same message across functions)
        let mut seen_layout: HashSet<(DiagnosticKind, String)> = HashSet::new();
        all_diagnostics.retain(|d| {
            if matches!(
                d.kind,
                DiagnosticKind::LargeStructMove | DiagnosticKind::PaddingWaste
            ) {
                seen_layout.insert((d.kind, d.message.clone()))
            } else {
                true
            }
        });

        if let Ok(mut locked) = self.diagnostics.lock() {
            locked.append(&mut all_diagnostics);
        }

        Compilation::Continue
    }
}
