use crate::graph::{DepGraph, EdgeKind};
use crate::scan::candidate::{ScanCandidate, ScanReport};
use crate::scan::config::{ScanConfig, ScanJob};
use crate::scan::filter::{
    is_transform_type_compatible, is_useful_transform, should_skip_function, should_skip_param,
    transforms_for_job,
};
use crate::simulate::ImpactAnalyzer;
use std::collections::HashSet;
use std::time::Instant;

/// The scan engine: discovers refactoring opportunities across a project.
pub struct ScanEngine<'g> {
    graph: &'g DepGraph,
    config: ScanConfig,
}

impl<'g> ScanEngine<'g> {
    pub fn new(graph: &'g DepGraph, config: ScanConfig) -> Self {
        Self { graph, config }
    }

    /// Run the scan and produce a report.
    pub fn scan(&self) -> ScanReport {
        let start = Instant::now();
        let analyzer = ImpactAnalyzer::new(self.graph);
        let transforms = transforms_for_job(self.config.job);

        let mut candidates = Vec::new();
        let mut functions_scanned = 0usize;
        let mut triples_evaluated = 0usize;
        let mut applicable_count = 0usize;

        // For CloneAudit, pre-compute which functions have clone edges.
        let clone_targets = if self.config.job == ScanJob::CloneAudit {
            self.find_clone_targets()
        } else {
            HashSet::new()
        };

        for (_idx, node) in self.graph.nodes() {
            if should_skip_function(node, &self.config) {
                continue;
            }

            // CloneAudit: only scan functions that are called with .clone().
            if self.config.job == ScanJob::CloneAudit
                && !clone_targets.contains(&node.signature.name)
            {
                continue;
            }

            functions_scanned += 1;
            let sig = &node.signature;

            for (param_idx, param) in sig.params.iter().enumerate() {
                if should_skip_param(param, &self.config) {
                    continue;
                }

                for transform in transforms {
                    triples_evaluated += 1;

                    // Filter cost-increasing / rarely-desired transforms.
                    if !is_useful_transform(transform, self.config.job) {
                        continue;
                    }

                    // Check ownership compatibility.
                    if param.type_info.ownership != transform.source_ownership() {
                        continue;
                    }

                    // Check type compatibility (String→&str only for String, etc.)
                    if !is_transform_type_compatible(param, transform) {
                        continue;
                    }

                    // Run impact analysis.
                    let impact = match analyzer.analyze(&sig.name, param_idx, transform) {
                        Some(impact) => impact,
                        None => continue,
                    };

                    applicable_count += 1;

                    // Skip candidates with no affected call sites — the
                    // transform is trivially "safe" but provides no value.
                    if impact.affected_count() == 0 {
                        continue;
                    }

                    // Filter by minimum score.
                    if impact.safety_score.total < self.config.min_score {
                        continue;
                    }

                    candidates.push(ScanCandidate {
                        function_name: sig.name.clone(),
                        short_name: sig.short_name.clone(),
                        param_index: param_idx,
                        param_name: param.name.clone(),
                        current_ownership: param.type_info.ownership,
                        transform: transform.clone(),
                        affected_sites: impact.affected_count(),
                        affected_files: impact.affected_files,
                        safety_score: impact.safety_score,
                    });
                }
            }
        }

        // Sort by safety score descending.
        candidates.sort_by(|a, b| b.safety_score.total.cmp(&a.safety_score.total));

        // Truncate if max_candidates is set.
        if let Some(max) = self.config.max_candidates {
            candidates.truncate(max);
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        ScanReport {
            job_name: self.config.job.to_string(),
            candidates,
            functions_scanned,
            triples_evaluated,
            applicable_count,
            duration_ms,
        }
    }

    /// Find functions that receive arguments via `.clone()`.
    fn find_clone_targets(&self) -> HashSet<String> {
        let mut targets = HashSet::new();
        for (idx, node) in self.graph.nodes() {
            for (_caller_idx, edge) in self.graph.callers(idx) {
                if edge.kind == EdgeKind::Clones {
                    targets.insert(node.signature.name.clone());
                    break;
                }
            }
        }
        targets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GraphBuilder;
    use tempfile::TempDir;

    const FIXTURE: &str = r#"
fn process(data: &str) -> String {
    data.to_string()
}

fn caller() {
    let s = String::from("hello");
    process(&s);
    consume(s.clone());
}

fn consume(s: String) {
    drop(s);
}

fn take_vec(v: Vec<u8>) {
    drop(v);
}

fn test_helper() {
    process(&"test");
}
"#;

    fn build_fixture() -> (TempDir, DepGraph) {
        let dir = TempDir::new().expect("tempdir");
        std::fs::write(dir.path().join("sample.rs"), FIXTURE).expect("write");
        let graph = GraphBuilder::build(dir.path()).expect("build");
        (dir, graph)
    }

    #[test]
    fn full_scan_finds_candidates() {
        let (_dir, graph) = build_fixture();
        let config = ScanConfig::default();
        let engine = ScanEngine::new(&graph, config);
        let report = engine.scan();

        assert!(report.functions_scanned > 0);
        assert!(report.applicable_count > 0);
        // All candidates must have at least 1 affected call site.
        for c in &report.candidates {
            assert!(
                c.affected_sites > 0,
                "candidate {} should have affected sites > 0",
                c.function_name
            );
        }
    }

    #[test]
    fn full_scan_skips_test_functions() {
        let (_dir, graph) = build_fixture();
        let config = ScanConfig {
            skip_test_functions: true,
            ..ScanConfig::default()
        };
        let engine = ScanEngine::new(&graph, config);
        let report = engine.scan();

        for c in &report.candidates {
            assert!(
                !c.function_name.contains("test_"),
                "test function should be skipped: {}",
                c.function_name
            );
        }
    }

    #[test]
    fn api_slim_only_finds_owned_types() {
        let (_dir, graph) = build_fixture();
        let config = ScanConfig {
            job: ScanJob::ApiSlim,
            ..ScanConfig::default()
        };
        let engine = ScanEngine::new(&graph, config);
        let report = engine.scan();

        for c in &report.candidates {
            assert!(
                matches!(
                    c.transform,
                    crate::simulate::Transform::StringToStr
                        | crate::simulate::Transform::VecToSlice
                        | crate::simulate::Transform::BoxToInline
                ),
                "api-slim should only have slim transforms, got: {}",
                c.transform
            );
        }
    }

    #[test]
    fn min_score_filters_candidates() {
        let (_dir, graph) = build_fixture();
        let config = ScanConfig {
            min_score: 95,
            ..ScanConfig::default()
        };
        let engine = ScanEngine::new(&graph, config);
        let report = engine.scan();

        for c in &report.candidates {
            assert!(
                c.safety_score.total >= 95,
                "candidate score {} should be >= 95",
                c.safety_score.total
            );
        }
    }

    #[test]
    fn self_analysis_scan() {
        let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let graph = GraphBuilder::build(project_root).expect("build");
        let config = ScanConfig::default();
        let engine = ScanEngine::new(&graph, config);
        let report = engine.scan();

        assert!(report.functions_scanned > 30);
        assert!(report.applicable_count > 0);
    }
}
