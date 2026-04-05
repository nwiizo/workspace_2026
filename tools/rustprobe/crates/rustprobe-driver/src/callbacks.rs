use std::sync::atomic::AtomicBool;

use rustc_driver::Compilation;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;

use rustprobe_analysis::ProbeData;

use crate::mir_visitor::MirAnalyzer;

static USING_INTERNAL_FEATURES: AtomicBool = AtomicBool::new(true);

pub struct PassthroughCallback;

impl rustc_driver::Callbacks for PassthroughCallback {}

pub struct ProbeCallback {
    output_dir: String,
}

impl ProbeCallback {
    pub fn new(output_dir: String) -> Self {
        Self { output_dir }
    }
}

impl rustc_driver::Callbacks for ProbeCallback {
    fn config(&mut self, config: &mut interface::Config) {
        config.using_internal_features = &USING_INTERNAL_FEATURES;
    }

    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> Compilation {
        let crate_name = tcx.crate_name(LOCAL_CRATE).to_string();
        let source_map = tcx.sess.source_map();
        let mut functions = Vec::new();

        for local_def_id in tcx.hir_body_owners() {
            if !tcx.is_mir_available(local_def_id) {
                continue;
            }

            let body = tcx.optimized_mir(local_def_id);
            let def_path = tcx.def_path_str(local_def_id.to_def_id());
            let name = tcx.item_name(local_def_id.to_def_id()).to_string();

            let (file, line_start, line_end) = resolve_span(source_map, body.span);

            let mut analyzer = MirAnalyzer::new(tcx);
            let probe = analyzer.analyze(
                body,
                format!("{crate_name}::{name}"),
                def_path,
                file,
                line_start,
                line_end,
            );
            functions.push(probe);
        }

        functions.sort_by(|a, b| {
            b.complexity_score
                .partial_cmp(&a.complexity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let data = ProbeData {
            crate_name: crate_name.clone(),
            functions,
        };

        if let Err(e) = write_probe_data(&self.output_dir, &crate_name, &data) {
            eprintln!("rustprobe: failed to write probe data: {e}");
        }

        Compilation::Continue
    }
}

fn resolve_span(
    source_map: &rustc_span::source_map::SourceMap,
    span: rustc_span::Span,
) -> (String, u32, u32) {
    let lo = source_map.lookup_char_pos(span.lo());
    let hi = source_map.lookup_char_pos(span.hi());
    let file = lo.file.name.prefer_local_unconditionally().to_string();
    (file, lo.line as u32, hi.line as u32)
}

fn write_probe_data(
    output_dir: &str,
    crate_name: &str,
    data: &ProbeData,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;
    let path = std::path::PathBuf::from(output_dir).join(format!("{crate_name}.json"));
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(path, json)?;
    Ok(())
}
