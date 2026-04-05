use rustc_driver::Callbacks;
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;

use crate::analysis::{self, AnalysisResult};
use crate::config::RustGuardConfig;

pub struct RustGuardCallbacks {
    config: RustGuardConfig,
    has_errors: bool,
}

impl RustGuardCallbacks {
    pub fn new(config: RustGuardConfig) -> Self {
        Self {
            config,
            has_errors: false,
        }
    }

    /// Whether any Error-severity findings were detected.
    pub fn has_errors(&self) -> bool {
        self.has_errors
    }
}

impl Callbacks for RustGuardCallbacks {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> rustc_driver::Compilation {
        let AnalysisResult { findings, summary } = analysis::run_analysis(tcx, &self.config);
        self.has_errors = summary.has_errors();

        let output = crate::output::render(&findings, &summary, self.config.output.format);
        match output {
            Ok(text) => {
                eprintln!("{text}");
            }
            Err(e) => {
                eprintln!("rustguard: output error: {e}");
            }
        }

        rustc_driver::Compilation::Stop
    }
}
