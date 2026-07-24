use std::collections::HashMap;

use serde::Serialize;

use crate::parser::FunctionInfo;

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub carries_question: bool,
    pub has_context: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ErrorGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug)]
pub(crate) struct FunctionIndex<'a> {
    functions: &'a [FunctionInfo],
    by_name: HashMap<&'a str, Vec<usize>>,
    by_file_name: HashMap<(&'a str, &'a str), Vec<usize>>,
    by_module_name: HashMap<(&'a str, &'a str), Vec<usize>>,
    used_bare_name_fallback: bool,
}

impl<'a> FunctionIndex<'a> {
    pub(crate) fn new(functions: &'a [FunctionInfo]) -> Self {
        let mut by_name = HashMap::new();
        let mut by_file_name = HashMap::new();
        let mut by_module_name = HashMap::new();
        for (idx, function) in functions.iter().enumerate() {
            by_name
                .entry(function.name.as_str())
                .or_insert_with(Vec::new)
                .push(idx);
            by_file_name
                .entry((function.rel_path.as_str(), function.name.as_str()))
                .or_insert_with(Vec::new)
                .push(idx);
            by_module_name
                .entry((function.module_path.as_str(), function.name.as_str()))
                .or_insert_with(Vec::new)
                .push(idx);
        }
        Self {
            functions,
            by_name,
            by_file_name,
            by_module_name,
            used_bare_name_fallback: false,
        }
    }

    pub(crate) fn fan_in(&mut self) -> HashMap<String, usize> {
        let mut fan_in = HashMap::new();
        for idx in 0..self.functions.len() {
            for target_idx in self.resolve_indices(idx) {
                *fan_in
                    .entry(self.functions[target_idx].id.clone())
                    .or_insert(0) += 1;
            }
        }
        fan_in
    }

    pub(crate) fn resolve_indices(&mut self, caller_idx: usize) -> Vec<usize> {
        let caller = &self.functions[caller_idx];
        let mut indices = Vec::new();
        for callee in &caller.callees {
            let key = (caller.rel_path.as_str(), callee.as_str());
            if let Some(targets) = self.by_file_name.get(&key) {
                indices.extend(targets);
                continue;
            }
            let key = (caller.module_path.as_str(), callee.as_str());
            if let Some(targets) = self.by_module_name.get(&key) {
                indices.extend(targets);
                continue;
            }
            if let Some(targets) = self.by_name.get(callee.as_str()) {
                self.used_bare_name_fallback = true;
                indices.extend(targets);
            }
        }
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    pub(crate) fn used_bare_name_fallback(&self) -> bool {
        self.used_bare_name_fallback
    }
}

impl ErrorGraph {
    pub(crate) fn from_index(index: &mut FunctionIndex<'_>) -> Self {
        let mut nodes: Vec<String> = index.functions.iter().map(|func| func.id.clone()).collect();
        nodes.sort();
        nodes.dedup();
        let mut edges = Vec::new();
        for idx in 0..index.functions.len() {
            let source = &index.functions[idx];
            for target_idx in index.resolve_indices(idx) {
                let target = &index.functions[target_idx];
                edges.push(GraphEdge {
                    source: source.id.clone(),
                    target: target.id.clone(),
                    carries_question: source.has_question,
                    has_context: source.has_context,
                });
            }
        }
        edges.sort_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then_with(|| a.target.cmp(&b.target))
        });
        edges.dedup_by(|a, b| a.source == b.source && a.target == b.target);
        Self { nodes, edges }
    }

    pub fn render_text(&self, japanese: bool) -> String {
        let mut out = String::new();
        if japanese {
            out.push_str("エラー伝播グラフ\n");
        } else {
            out.push_str("Error propagation graph\n");
        }
        for edge in &self.edges {
            let marker = if edge.has_context {
                "context"
            } else if edge.carries_question {
                "?"
            } else {
                "call"
            };
            out.push_str(&format!(
                "{} -> {} [{}]\n",
                edge.source, edge.target, marker
            ));
        }
        if self.edges.is_empty() {
            if japanese {
                out.push_str("(crate 内関数 edge は検出されませんでした)\n");
            } else {
                out.push_str("(no intra-crate function edges detected)\n");
            }
        }
        out
    }

    pub fn render_dot(&self, japanese: bool) -> String {
        let mut out = if japanese {
            String::from("// エラー伝播グラフ\n")
        } else {
            String::new()
        };
        out.push_str("digraph error_map {\n");
        for node in &self.nodes {
            out.push_str(&format!("  \"{}\";\n", escape_dot(node)));
        }
        for edge in &self.edges {
            let label = if edge.has_context {
                "context"
            } else if edge.carries_question {
                "?"
            } else {
                "call"
            };
            out.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                escape_dot(&edge.source),
                escape_dot(&edge.target),
                label
            ));
        }
        out.push_str("}\n");
        out
    }
}

fn escape_dot(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
