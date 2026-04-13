use crate::graph::SystemGraph;
use crate::simulation::cascade::CascadeEngine;
use crate::simulation::types::*;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;

/// SPOF detection engine using Tarjan's algorithm.
pub struct SpofEngine<'g> {
    graph: &'g SystemGraph,
}

impl<'g> SpofEngine<'g> {
    pub fn new(graph: &'g SystemGraph) -> Self {
        Self { graph }
    }

    /// Run full SPOF analysis.
    pub fn analyze(&self) -> SpofResult {
        let index_map = self.graph.index_map();

        // Reverse map: NodeIndex → component ID.
        let rev_map: HashMap<NodeIndex, &str> = index_map
            .iter()
            .map(|(id, &idx)| (idx, id.as_str()))
            .collect();

        // Find articulation points using Tarjan's algorithm on undirected view.
        let (art_points, bridges) = self.tarjan_ap_and_bridges(&rev_map);

        // Build SPOF entries with criticality scoring.
        let mut spof_entries: Vec<SpofEntry> = Vec::new();

        for &ap_idx in &art_points {
            let comp_id = rev_map[&ap_idx];
            let comp = self.graph.component(comp_id).expect("component exists");

            // Simulate cascade to determine components at risk.
            let cascade = CascadeEngine::new(self.graph, 0.5);
            let at_risk: Vec<String> = cascade
                .simulate(comp_id)
                .map(|r| {
                    r.cascade_path
                        .iter()
                        .filter(|s| s.component_id != comp_id)
                        .map(|s| s.component_id.clone())
                        .collect()
                })
                .unwrap_or_default();

            let dependents_count = self.graph.dependents(comp_id).len();
            let total = self.graph.component_count();

            let impact_ratio = at_risk.len() as f64 / total.max(1) as f64;
            let redundancy_factor = if comp.redundancy > 1 {
                0.5 / comp.redundancy as f64
            } else {
                1.0
            };
            let ap_factor = 1.0; // It IS an articulation point.
            let dep_ratio = dependents_count as f64 / total.max(1) as f64;

            let score = (impact_ratio * 40.0
                + redundancy_factor * 30.0
                + ap_factor * 20.0
                + dep_ratio * 10.0)
                .min(100.0);

            let recommendation = if comp.redundancy <= 1 {
                format!(
                    "Add redundancy to '{}' — it is a single point of failure with {} components at risk",
                    comp.name,
                    at_risk.len()
                )
            } else {
                format!(
                    "'{}' is an articulation point despite redundancy={} — consider architectural changes",
                    comp.name, comp.redundancy
                )
            };

            spof_entries.push(SpofEntry {
                component_id: comp_id.to_string(),
                component_name: comp.name.clone(),
                criticality_score: score,
                components_at_risk: at_risk,
                is_articulation_point: true,
                redundancy: comp.redundancy,
                recommendation,
            });
        }

        // Also flag non-AP components with redundancy=1 and high dependent count.
        for comp in self.graph.all_components() {
            if art_points.iter().any(|&idx| rev_map[&idx] == comp.id) {
                continue; // Already included as AP.
            }
            let dependents_count = self.graph.dependents(&comp.id).len();
            if comp.redundancy <= 1 && dependents_count >= 2 {
                let cascade = CascadeEngine::new(self.graph, 0.5);
                let at_risk: Vec<String> = cascade
                    .simulate(&comp.id)
                    .map(|r| {
                        r.cascade_path
                            .iter()
                            .filter(|s| s.component_id != comp.id)
                            .map(|s| s.component_id.clone())
                            .collect()
                    })
                    .unwrap_or_default();

                if !at_risk.is_empty() {
                    let score = (at_risk.len() as f64 / self.graph.component_count().max(1) as f64
                        * 40.0
                        + 30.0) // redundancy=1 penalty
                        .min(100.0);

                    spof_entries.push(SpofEntry {
                        component_id: comp.id.clone(),
                        component_name: comp.name.clone(),
                        criticality_score: score,
                        components_at_risk: at_risk,
                        is_articulation_point: false,
                        redundancy: comp.redundancy,
                        recommendation: format!(
                            "Add redundancy to '{}' — {} dependents with no failover",
                            comp.name, dependents_count
                        ),
                    });
                }
            }
        }

        spof_entries.sort_by(|a, b| {
            b.criticality_score
                .partial_cmp(&a.criticality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Build bridge entries.
        let bridge_entries: Vec<BridgeEntry> = bridges
            .iter()
            .map(|&(from_idx, to_idx)| {
                let from_id = rev_map[&from_idx];
                let to_id = rev_map[&to_idx];
                BridgeEntry {
                    from: from_id.to_string(),
                    to: to_id.to_string(),
                    criticality_score: 50.0, // Base score for bridges.
                }
            })
            .collect();

        // Compute resilience score (100 = fully resilient, 0 = fragile).
        let total = self.graph.component_count() as f64;
        let spof_penalty = spof_entries.len() as f64 / total.max(1.0) * 50.0;
        let bridge_penalty =
            bridge_entries.len() as f64 / self.graph.dependency_count().max(1) as f64 * 30.0;
        let redundancy_bonus = self
            .graph
            .all_components()
            .filter(|c| c.redundancy > 1)
            .count() as f64
            / total.max(1.0)
            * 20.0;
        let resilience =
            (100.0 - spof_penalty - bridge_penalty + redundancy_bonus).clamp(0.0, 100.0);

        SpofResult {
            single_points_of_failure: spof_entries,
            bridges: bridge_entries,
            resilience_score: resilience,
        }
    }

    /// Tarjan's algorithm for articulation points and bridges.
    /// Operates on the undirected view of the directed graph.
    fn tarjan_ap_and_bridges(
        &self,
        rev_map: &HashMap<NodeIndex, &str>,
    ) -> (Vec<NodeIndex>, Vec<(NodeIndex, NodeIndex)>) {
        let mut disc = HashMap::new();
        let mut low = HashMap::new();
        let mut parent: HashMap<NodeIndex, Option<NodeIndex>> = HashMap::new();
        let mut aps = Vec::new();
        let mut bridges = Vec::new();
        let mut timer = 0u32;

        for &idx in rev_map.keys() {
            if !disc.contains_key(&idx) {
                self.tarjan_dfs(
                    idx,
                    None,
                    &mut disc,
                    &mut low,
                    &mut parent,
                    &mut aps,
                    &mut bridges,
                    &mut timer,
                );
            }
        }

        aps.sort();
        aps.dedup();
        (aps, bridges)
    }

    #[allow(clippy::too_many_arguments)]
    fn tarjan_dfs(
        &self,
        u: NodeIndex,
        par: Option<NodeIndex>,
        disc: &mut HashMap<NodeIndex, u32>,
        low: &mut HashMap<NodeIndex, u32>,
        parent: &mut HashMap<NodeIndex, Option<NodeIndex>>,
        aps: &mut Vec<NodeIndex>,
        bridges: &mut Vec<(NodeIndex, NodeIndex)>,
        timer: &mut u32,
    ) {
        let inner = self.graph.inner();
        disc.insert(u, *timer);
        low.insert(u, *timer);
        parent.insert(u, par);
        *timer += 1;
        let mut child_count = 0u32;

        // Get undirected neighbors (both incoming and outgoing edges).
        let neighbors: Vec<NodeIndex> = inner
            .edges_directed(u, Direction::Outgoing)
            .map(|e| e.target())
            .chain(
                inner
                    .edges_directed(u, Direction::Incoming)
                    .map(|e| e.source()),
            )
            .collect();

        for v in neighbors {
            if !disc.contains_key(&v) {
                child_count += 1;
                self.tarjan_dfs(v, Some(u), disc, low, parent, aps, bridges, timer);

                let low_u = low[&u].min(low[&v]);
                low.insert(u, low_u);

                // u is an articulation point if:
                // 1. u is root and has 2+ children
                // 2. u is not root and low[v] >= disc[u]
                if par.is_none() && child_count > 1 {
                    aps.push(u);
                }
                if par.is_some() && low[&v] >= disc[&u] {
                    aps.push(u);
                }

                // Bridge: low[v] > disc[u]
                if low[&v] > disc[&u] {
                    bridges.push((u, v));
                }
            } else if Some(v) != par {
                let low_u = low[&u].min(disc[&v]);
                low.insert(u, low_u);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::loader::parse_yaml;
    use std::path::PathBuf;

    #[test]
    fn star_topology_center_is_spof() {
        let yaml = r#"
name: star
components:
  - id: center
    name: Center Hub
    type: service
    redundancy: 1
  - id: leaf1
    name: Leaf 1
    type: service
  - id: leaf2
    name: Leaf 2
    type: service
  - id: leaf3
    name: Leaf 3
    type: service
dependencies:
  - from: leaf1
    to: center
    type: sync
    criticality: critical
  - from: leaf2
    to: center
    type: sync
    criticality: critical
  - from: leaf3
    to: center
    type: sync
    criticality: critical
"#;
        let topology = parse_yaml(yaml, &PathBuf::from("test.yaml")).unwrap();
        let graph = SystemGraph::from_topology(&topology).unwrap();
        let result = SpofEngine::new(&graph).analyze();

        // center should be flagged as SPOF (3 dependents, redundancy=1)
        let center_spof = result
            .single_points_of_failure
            .iter()
            .find(|s| s.component_id == "center");
        assert!(
            center_spof.is_some(),
            "center should be a SPOF, found: {:?}",
            result
                .single_points_of_failure
                .iter()
                .map(|s| &s.component_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn resilience_score_range() {
        let yaml = r#"
name: test
components:
  - id: a
    name: A
    type: service
    redundancy: 3
  - id: b
    name: B
    type: service
    redundancy: 2
dependencies:
  - from: a
    to: b
    type: sync
    criticality: medium
"#;
        let topology = parse_yaml(yaml, &PathBuf::from("test.yaml")).unwrap();
        let graph = SystemGraph::from_topology(&topology).unwrap();
        let result = SpofEngine::new(&graph).analyze();

        assert!(
            (0.0..=100.0).contains(&result.resilience_score),
            "resilience score {} out of range",
            result.resilience_score
        );
    }
}
