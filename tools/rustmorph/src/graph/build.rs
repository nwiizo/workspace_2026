use crate::analysis::ProjectAnalysis;
use crate::error::Result;
use crate::graph::{DepGraph, NodeKind};
use std::path::Path;

/// Builds a `DepGraph` from a Rust project directory.
pub struct GraphBuilder;

impl GraphBuilder {
    /// Analyze a project directory and build the ownership dependency graph.
    pub fn build(project_root: &Path) -> Result<DepGraph> {
        let analysis = ProjectAnalysis::analyze(project_root)?;
        let mut graph = DepGraph::new();

        // Add all functions as nodes (consume the Vec to avoid cloning).
        for sig in analysis.functions {
            let kind = if sig.impl_target.is_some() {
                NodeKind::Method
            } else {
                NodeKind::Function
            };
            graph.add_function(sig, kind);
        }

        // Add all call sites as edges.
        for call in &analysis.call_sites {
            graph.add_call(call);
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulate::{ImpactAnalyzer, Transform};
    use tempfile::TempDir;

    const FIXTURE: &str = r#"
fn process(data: &str) -> String {
    data.to_string()
}

fn caller() {
    let result = process(&"hello");
    consume(result);
}

fn consume(s: String) {
    drop(s);
}

struct Service {
    name: String,
}

impl Service {
    fn handle(&self, input: &str) -> String {
        process(input)
    }
}
"#;

    fn build_fixture_graph() -> (TempDir, DepGraph) {
        let dir = TempDir::new().expect("failed to create temp dir");
        let sample = dir.path().join("sample.rs");
        std::fs::write(&sample, FIXTURE).expect("failed to write fixture");
        let graph = GraphBuilder::build(dir.path()).expect("failed to build graph");
        (dir, graph)
    }

    // --- Step 4: Integration test with fixture ---

    #[test]
    fn fixture_graph_has_nodes_and_edges() {
        let (_dir, graph) = build_fixture_graph();

        // 4 functions: process, caller, consume, Service::handle
        assert_eq!(graph.node_count(), 4);
        assert!(
            graph.edge_count() > 0,
            "Expected edges but got {}",
            graph.edge_count()
        );
    }

    #[test]
    fn fixture_caller_calls_process_and_consume() {
        let (_dir, graph) = build_fixture_graph();

        let caller_idx = graph
            .find_function("sample::caller")
            .expect("caller not found");
        let callees = graph.callees(caller_idx);

        let callee_names: Vec<_> = callees
            .iter()
            .filter_map(|(idx, _)| graph.get_signature(*idx).map(|s| s.short_name.as_str()))
            .collect();
        assert!(
            callee_names.contains(&"process"),
            "caller should call process, got: {callee_names:?}"
        );
        assert!(
            callee_names.contains(&"consume"),
            "caller should call consume, got: {callee_names:?}"
        );
    }

    #[test]
    fn fixture_method_calls_process() {
        let (_dir, graph) = build_fixture_graph();

        let process_idx = graph
            .find_function("sample::process")
            .expect("process not found");
        let callers = graph.callers(process_idx);

        let caller_names: Vec<_> = callers
            .iter()
            .filter_map(|(idx, _)| graph.get_signature(*idx).map(|s| s.short_name.as_str()))
            .collect();
        assert!(
            caller_names.contains(&"handle"),
            "Service::handle should call process, got: {caller_names:?}"
        );
    }

    // --- Step 5: Self-analysis test ---

    #[test]
    fn self_analysis_produces_edges() {
        let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let graph = GraphBuilder::build(project_root).expect("failed to self-analyze");

        assert!(
            graph.node_count() > 50,
            "Expected many functions, got {}",
            graph.node_count()
        );
        assert!(
            graph.edge_count() > 0,
            "Expected non-zero edges from self-analysis, got 0"
        );
    }

    // --- Step 6: Preview E2E test ---

    #[test]
    fn preview_end_to_end() {
        let (_dir, graph) = build_fixture_graph();

        let analyzer = ImpactAnalyzer::new(&graph);

        // process takes &str (param 0 is "data: &str"), try RefToOwned
        let impact = analyzer
            .analyze("sample::process", 0, &Transform::RefToOwned)
            .expect("impact analysis should succeed for process param 0 RefToOwned");

        assert!(
            !impact.changes.is_empty(),
            "Expected changes at call sites, got 0"
        );
        assert!(impact.safety_score.total > 0, "Safety score should be > 0");
    }
}
