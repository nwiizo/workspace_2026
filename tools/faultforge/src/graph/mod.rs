pub(crate) mod builder;

use crate::error::{FaultForgeError, Result};
use crate::topology::model::{Component, Dependency, Topology};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;

/// System dependency graph wrapping petgraph.
#[derive(Debug)]
pub struct SystemGraph {
    graph: DiGraph<Component, Dependency>,
    index: HashMap<String, NodeIndex>,
}

impl SystemGraph {
    /// Build a SystemGraph from a validated Topology.
    pub fn from_topology(topology: &Topology) -> Result<Self> {
        let mut graph = DiGraph::new();
        let mut index = HashMap::new();

        for comp in &topology.components {
            let idx = graph.add_node(comp.clone());
            index.insert(comp.id.clone(), idx);
        }

        for dep in &topology.dependencies {
            let from_idx = index
                .get(&dep.from)
                .ok_or_else(|| FaultForgeError::Topology {
                    message: format!("dependency source '{}' not found", dep.from),
                })?;
            let to_idx = index
                .get(&dep.to)
                .ok_or_else(|| FaultForgeError::Topology {
                    message: format!("dependency target '{}' not found", dep.to),
                })?;
            graph.add_edge(*from_idx, *to_idx, dep.clone());
        }

        Ok(Self { graph, index })
    }

    /// Look up a component by ID.
    pub fn component(&self, id: &str) -> Option<&Component> {
        self.index
            .get(id)
            .and_then(|&idx| self.graph.node_weight(idx))
    }

    /// Get the NodeIndex for a component ID.
    pub fn node_index(&self, id: &str) -> Option<NodeIndex> {
        self.index.get(id).copied()
    }

    /// Get components that depend ON this component (incoming edges in dependency direction).
    /// If A→B means "A depends on B", then dependents(B) returns [A].
    pub fn dependents(&self, id: &str) -> Vec<(&str, &Dependency)> {
        let Some(&idx) = self.index.get(id) else {
            return Vec::new();
        };
        self.graph
            .edges_directed(idx, Direction::Incoming)
            .map(|e| {
                let source_comp = &self.graph[e.source()];
                (source_comp.id.as_str(), e.weight())
            })
            .collect()
    }

    /// Get components that this component depends on (outgoing edges).
    pub fn dependencies(&self, id: &str) -> Vec<(&str, &Dependency)> {
        let Some(&idx) = self.index.get(id) else {
            return Vec::new();
        };
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .map(|e| {
                let target_comp = &self.graph[e.target()];
                (target_comp.id.as_str(), e.weight())
            })
            .collect()
    }

    /// Iterate all components.
    pub fn all_components(&self) -> impl Iterator<Item = &Component> {
        self.graph.node_weights()
    }

    /// Number of components.
    pub fn component_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Number of dependency edges.
    pub fn dependency_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get all component IDs.
    pub fn component_ids(&self) -> Vec<&str> {
        self.index.keys().map(|s| s.as_str()).collect()
    }

    /// Access inner petgraph for advanced algorithms.
    pub(crate) fn inner(&self) -> &DiGraph<Component, Dependency> {
        &self.graph
    }

    /// Get index map for algorithms.
    pub(crate) fn index_map(&self) -> &HashMap<String, NodeIndex> {
        &self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::loader::parse_yaml;
    use std::path::PathBuf;

    fn build_test_graph() -> SystemGraph {
        let yaml = r#"
name: test
components:
  - id: gateway
    name: Gateway
    type: gateway
  - id: service
    name: Service
    type: service
  - id: db
    name: Database
    type: database
dependencies:
  - from: gateway
    to: service
    type: sync
    criticality: critical
  - from: service
    to: db
    type: sync
    criticality: high
"#;
        let topology = parse_yaml(yaml, &PathBuf::from("test.yaml")).unwrap();
        SystemGraph::from_topology(&topology).unwrap()
    }

    #[test]
    fn graph_construction() {
        let graph = build_test_graph();
        assert_eq!(graph.component_count(), 3);
        assert_eq!(graph.dependency_count(), 2);
    }

    #[test]
    fn dependents_lookup() {
        let graph = build_test_graph();
        // service depends on db, so db's dependents include service
        let deps = graph.dependents("db");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "service");
    }

    #[test]
    fn dependencies_lookup() {
        let graph = build_test_graph();
        let deps = graph.dependencies("service");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, "db");
    }
}
