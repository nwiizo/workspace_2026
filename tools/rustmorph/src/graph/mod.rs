mod build;

pub use build::GraphBuilder;

use crate::types::{ArgUsage, CallSite, FunctionSignature, SpanInfo};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A node in the ownership dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepNode {
    pub kind: NodeKind,
    pub signature: FunctionSignature,
}

/// The kind of a graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Function,
    Method,
    Closure,
}

/// An edge in the ownership dependency graph — represents how a caller passes
/// data to a callee at a specific call site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepEdge {
    pub kind: EdgeKind,
    /// Which parameter index this edge refers to.
    pub param_index: usize,
    /// The call site where this edge originates.
    pub call_site: SpanInfo,
    /// The raw expression text of the argument.
    pub arg_expr: String,
}

/// The kind of data flow along an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Ownership transfer (move).
    Owns,
    /// Shared borrow (`&T`).
    Borrows,
    /// Mutable borrow (`&mut T`).
    MutBorrows,
    /// Clone before passing.
    Clones,
}

impl From<ArgUsage> for EdgeKind {
    fn from(usage: ArgUsage) -> Self {
        match usage {
            ArgUsage::Move => Self::Owns,
            ArgUsage::Borrow => Self::Borrows,
            ArgUsage::BorrowMut => Self::MutBorrows,
            ArgUsage::Clone => Self::Clones,
        }
    }
}

/// Edge kind counts for summary display.
#[derive(Debug, Default)]
pub struct EdgeKindCounts {
    pub owns: usize,
    pub borrows: usize,
    pub mut_borrows: usize,
    pub clones: usize,
}

/// The ownership dependency graph.
#[derive(Debug)]
pub struct DepGraph {
    graph: DiGraph<DepNode, DepEdge>,
    /// Full qualified name → NodeIndex.
    index: HashMap<String, NodeIndex>,
    /// Short name / suffix → list of full qualified names (for fallback resolution).
    short_name_index: HashMap<String, Vec<String>>,
}

impl DepGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index: HashMap::new(),
            short_name_index: HashMap::new(),
        }
    }

    /// Add a function node to the graph.
    pub(crate) fn add_function(&mut self, sig: FunctionSignature, kind: NodeKind) -> NodeIndex {
        let original_name = sig.name.clone();
        // Disambiguate the index key if a function with the same name already exists
        // (e.g., multiple trait impls: Display::fmt and Debug::fmt).
        let key = if self.index.contains_key(&original_name) {
            let mut counter = 2;
            loop {
                let candidate = format!("{original_name}#{counter}");
                if !self.index.contains_key(&candidate) {
                    break candidate;
                }
                counter += 1;
            }
        } else {
            original_name.clone()
        };
        let idx = self.graph.add_node(DepNode {
            kind,
            signature: sig,
        });
        self.index.insert(key, idx);

        // Register suffixes from the ORIGINAL name for fallback resolution.
        // e.g. "mod::Struct::method" → register "method", "Struct::method"
        let parts: Vec<&str> = original_name.split("::").collect();
        for i in 1..parts.len() {
            let suffix = parts[i..].join("::");
            self.short_name_index
                .entry(suffix)
                .or_default()
                .push(original_name.clone());
        }

        idx
    }

    /// Resolve a name to a NodeIndex, trying exact match first, then suffix fallback.
    fn resolve_name(&self, name: &str) -> Option<NodeIndex> {
        // 1. Exact match.
        if let Some(&idx) = self.index.get(name) {
            return Some(idx);
        }
        // 2. Short name / suffix fallback (only if unambiguous).
        if let Some(candidates) = self.short_name_index.get(name) {
            if candidates.len() == 1 {
                return self.index.get(&candidates[0]).copied();
            }
        }
        None
    }

    /// Add a call edge from caller to callee.
    pub(crate) fn add_call(&mut self, call_site: &CallSite) {
        let caller_idx = self.resolve_name(&call_site.caller);
        let callee_idx = self.resolve_name(&call_site.callee);

        if let (Some(caller), Some(callee)) = (caller_idx, callee_idx) {
            for (i, arg) in call_site.args.iter().enumerate() {
                self.graph.add_edge(
                    caller,
                    callee,
                    DepEdge {
                        kind: arg.usage.into(),
                        param_index: i,
                        call_site: call_site.span.clone(),
                        arg_expr: arg.expr.clone(),
                    },
                );
            }
        }
    }

    /// Look up a function node by name (exact match, then fallback).
    pub fn find_function(&self, name: &str) -> Option<NodeIndex> {
        self.resolve_name(name)
    }

    /// Get the function signature for a node.
    pub fn get_signature(&self, idx: NodeIndex) -> Option<&FunctionSignature> {
        self.graph.node_weight(idx).map(|n| &n.signature)
    }

    /// Get all callers of a given function (incoming edges).
    pub fn callers(&self, idx: NodeIndex) -> Vec<(NodeIndex, &DepEdge)> {
        self.graph
            .edges_directed(idx, Direction::Incoming)
            .map(|e| (e.source(), e.weight()))
            .collect()
    }

    /// Get all callees from a given function (outgoing edges).
    pub fn callees(&self, idx: NodeIndex) -> Vec<(NodeIndex, &DepEdge)> {
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .map(|e| (e.target(), e.weight()))
            .collect()
    }

    /// Iterate all nodes with their indices.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeIndex, &DepNode)> {
        self.graph
            .node_indices()
            .map(move |idx| (idx, &self.graph[idx]))
    }

    /// Get all function names in the graph (from node signatures, not index keys).
    pub fn function_names(&self) -> Vec<&str> {
        self.graph
            .node_weights()
            .map(|n| n.signature.name.as_str())
            .collect()
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Count edges by kind.
    pub fn edge_kind_counts(&self) -> EdgeKindCounts {
        let mut counts = EdgeKindCounts::default();
        for edge in self.graph.edge_weights() {
            match edge.kind {
                EdgeKind::Owns => counts.owns += 1,
                EdgeKind::Borrows => counts.borrows += 1,
                EdgeKind::MutBorrows => counts.mut_borrows += 1,
                EdgeKind::Clones => counts.clones += 1,
            }
        }
        counts
    }
}

impl Default for DepGraph {
    fn default() -> Self {
        Self::new()
    }
}
