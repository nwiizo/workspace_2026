use crate::error::{FaultForgeError, Result};
use crate::graph::SystemGraph;
use crate::simulation::types::*;
use crate::topology::model::{Criticality, DependencyType};
use std::collections::{HashSet, VecDeque};

/// LTS-based cascade failure simulation engine.
pub struct CascadeEngine<'g> {
    graph: &'g SystemGraph,
    threshold: f64,
}

impl<'g> CascadeEngine<'g> {
    pub fn new(graph: &'g SystemGraph, threshold: f64) -> Self {
        Self { graph, threshold }
    }

    /// Simulate cascade failure starting from the given component.
    pub fn simulate(&self, component_id: &str) -> Result<CascadeResult> {
        let origin = self
            .graph
            .component(component_id)
            .ok_or_else(|| FaultForgeError::ComponentNotFound(component_id.to_string()))?;

        let mut cascade_path = Vec::new();
        let mut failed: HashSet<String> = HashSet::new();
        let mut degraded: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        // Initial state: origin component fails.
        failed.insert(component_id.to_string());
        cascade_path.push(CascadeStep {
            component_id: component_id.to_string(),
            component_name: origin.name.clone(),
            depth: 0,
            state: ComponentState::Failed,
            failure_mode: FailureMode::Origin,
            propagation_probability: 1.0,
        });
        queue.push_back((component_id.to_string(), 0));

        // BFS: propagate failure to dependents (reverse direction).
        while let Some((current_id, depth)) = queue.pop_front() {
            let dependents = self.graph.dependents(&current_id);

            for (dependent_id, dep) in dependents {
                if failed.contains(dependent_id) {
                    continue;
                }

                let prob = self.compute_propagation_probability(dep);
                let (state, mode) = self.determine_state(dep, prob);

                let comp = self.graph.component(dependent_id);
                let comp_name = comp.map_or("unknown", |c| &c.name);

                cascade_path.push(CascadeStep {
                    component_id: dependent_id.to_string(),
                    component_name: comp_name.to_string(),
                    depth: depth + 1,
                    state,
                    failure_mode: mode,
                    propagation_probability: prob,
                });

                match state {
                    ComponentState::Failed => {
                        failed.insert(dependent_id.to_string());
                        queue.push_back((dependent_id.to_string(), depth + 1));
                    }
                    ComponentState::Degraded => {
                        degraded.insert(dependent_id.to_string());
                        // Degraded components don't propagate full failure.
                    }
                    ComponentState::Healthy => {}
                }
            }
        }

        let total = self.graph.component_count();
        let affected_count = failed.len() + degraded.len();

        let directly_affected: Vec<String> = cascade_path
            .iter()
            .filter(|s| s.depth == 1 && s.state == ComponentState::Failed)
            .map(|s| s.component_id.clone())
            .collect();

        let transitively_affected: Vec<String> = cascade_path
            .iter()
            .filter(|s| s.depth > 1 && s.state == ComponentState::Failed)
            .map(|s| s.component_id.clone())
            .collect();

        let impact_pct = (affected_count as f64 / total as f64) * 100.0;

        let estimated_recovery = cascade_path
            .iter()
            .filter(|s| s.state == ComponentState::Failed)
            .filter_map(|s| self.graph.component(&s.component_id))
            .map(|c| c.recovery_time_seconds)
            .fold(0.0f64, f64::max);

        Ok(CascadeResult {
            origin_component: component_id.to_string(),
            cascade_path,
            blast_radius: BlastRadius {
                directly_affected,
                transitively_affected,
                total_affected: affected_count,
                total_components: total,
                impact_percentage: impact_pct,
            },
            estimated_recovery_seconds: estimated_recovery,
            severity: Severity::from_impact(impact_pct),
        })
    }

    fn compute_propagation_probability(&self, dep: &crate::topology::model::Dependency) -> f64 {
        let base = dep.criticality.weight();

        // Sync dependencies propagate more strongly.
        let type_factor = match dep.dependency_type {
            DependencyType::Sync => 1.0,
            DependencyType::Async => 0.5,
            DependencyType::EventDriven => 0.4,
            DependencyType::Batch => 0.3,
        };

        // Fallback/retry reduce probability.
        let mitigation = if dep.has_fallback {
            0.3
        } else if dep.has_retry {
            0.8
        } else {
            1.0
        };

        (base * type_factor * mitigation).min(1.0)
    }

    fn determine_state(
        &self,
        dep: &crate::topology::model::Dependency,
        prob: f64,
    ) -> (ComponentState, FailureMode) {
        if prob < self.threshold {
            return (ComponentState::Degraded, FailureMode::Degraded);
        }

        match dep.dependency_type {
            DependencyType::Sync if dep.criticality >= Criticality::High && !dep.has_fallback => {
                (ComponentState::Failed, FailureMode::DirectDependency)
            }
            DependencyType::Sync => (ComponentState::Failed, FailureMode::CascadePropagation),
            _ => (ComponentState::Degraded, FailureMode::Degraded),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::loader::parse_yaml;
    use std::path::PathBuf;

    fn simulate_yaml(yaml: &str, component: &str) -> CascadeResult {
        let topology = parse_yaml(yaml, &PathBuf::from("test.yaml")).unwrap();
        let graph = SystemGraph::from_topology(&topology).unwrap();
        CascadeEngine::new(&graph, 0.5).simulate(component).unwrap()
    }

    #[test]
    fn linear_cascade() {
        let result = simulate_yaml(
            r#"
name: test
components:
  - id: a
    name: A
    type: gateway
  - id: b
    name: B
    type: service
  - id: c
    name: C
    type: database
    recovery_time_seconds: 300
dependencies:
  - from: a
    to: b
    type: sync
    criticality: critical
  - from: b
    to: c
    type: sync
    criticality: critical
"#,
            "c",
        );
        // c fails → b depends on c (sync/critical) → a depends on b
        assert!(result.blast_radius.total_affected >= 2);
        assert_eq!(result.origin_component, "c");
    }

    #[test]
    fn fallback_limits_cascade() {
        let result = simulate_yaml(
            r#"
name: test
components:
  - id: main-db
    name: Main DB
    type: database
  - id: service
    name: Service
    type: service
  - id: cache
    name: Cache
    type: cache
dependencies:
  - from: service
    to: main-db
    type: sync
    criticality: critical
    has_fallback: true
  - from: service
    to: cache
    type: async
    criticality: low
"#,
            "main-db",
        );
        // service has fallback to main-db, so it should be degraded not failed
        let service_step = result
            .cascade_path
            .iter()
            .find(|s| s.component_id == "service");
        assert!(service_step.is_some());
        assert_eq!(
            service_step.unwrap().state,
            ComponentState::Degraded,
            "service should be degraded due to fallback"
        );
    }

    #[test]
    fn unknown_component_error() {
        let topology = parse_yaml(
            r#"
name: test
components:
  - id: a
    name: A
    type: service
dependencies: []
"#,
            &PathBuf::from("test.yaml"),
        )
        .unwrap();
        let graph = SystemGraph::from_topology(&topology).unwrap();
        let result = CascadeEngine::new(&graph, 0.5).simulate("nonexistent");
        assert!(result.is_err());
    }
}
