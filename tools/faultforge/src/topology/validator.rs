use crate::topology::model::Topology;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Validate a topology for structural correctness.
pub fn validate(topology: &Topology) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 1. Check for empty components.
    if topology.components.is_empty() {
        errors.push("topology must have at least one component".to_string());
        return ValidationReport { errors, warnings };
    }

    // 2. Check for duplicate component IDs.
    let mut seen_ids = HashSet::new();
    for comp in &topology.components {
        if !seen_ids.insert(&comp.id) {
            errors.push(format!("duplicate component ID: '{}'", comp.id));
        }
    }

    let component_ids: HashSet<&str> = topology.components.iter().map(|c| c.as_id()).collect();

    // 3. Validate dependencies reference existing components.
    for dep in &topology.dependencies {
        if !component_ids.contains(dep.from.as_str()) {
            errors.push(format!(
                "dependency references unknown component '{}' (from)",
                dep.from
            ));
        }
        if !component_ids.contains(dep.to.as_str()) {
            errors.push(format!(
                "dependency references unknown component '{}' (to)",
                dep.to
            ));
        }
        // 4. No self-referential dependencies.
        if dep.from == dep.to {
            errors.push(format!(
                "self-referential dependency: '{}' → '{}'",
                dep.from, dep.to
            ));
        }
    }

    // 5. Validate probability ranges.
    for comp in &topology.components {
        if !(0.0..=1.0).contains(&comp.failure_probability) {
            errors.push(format!(
                "component '{}' has invalid failure_probability: {} (must be 0.0-1.0)",
                comp.id, comp.failure_probability
            ));
        }
        if comp.recovery_time_seconds < 0.0 {
            errors.push(format!(
                "component '{}' has negative recovery_time_seconds: {}",
                comp.id, comp.recovery_time_seconds
            ));
        }
    }

    // 6. Warn on orphan components (no dependencies).
    let referenced: HashSet<&str> = topology
        .dependencies
        .iter()
        .flat_map(|d| [d.from.as_str(), d.to.as_str()])
        .collect();
    for comp in &topology.components {
        if !referenced.contains(comp.id.as_str()) {
            warnings.push(format!(
                "component '{}' has no dependencies (orphan)",
                comp.id
            ));
        }
    }

    // 7. Detect cycles (warning, not error — cycles exist in real systems).
    if let Some(cycle) = detect_cycle(topology) {
        warnings.push(format!("dependency cycle detected: {}", cycle.join(" → ")));
    }

    ValidationReport { errors, warnings }
}

trait AsId {
    fn as_id(&self) -> &str;
}

impl AsId for crate::topology::model::Component {
    fn as_id(&self) -> &str {
        &self.id
    }
}

fn detect_cycle(topology: &Topology) -> Option<Vec<String>> {
    // Build adjacency list.
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for dep in &topology.dependencies {
        adj.entry(dep.from.as_str())
            .or_default()
            .push(dep.to.as_str());
    }

    // BFS-based cycle detection using coloring.
    let mut color: HashMap<&str, u8> = HashMap::new(); // 0=white, 1=gray, 2=black
    let mut parent: HashMap<&str, &str> = HashMap::new();

    for comp in &topology.components {
        color.insert(comp.id.as_str(), 0);
    }

    for comp in &topology.components {
        if color[comp.id.as_str()] == 0 {
            if let Some(cycle) = dfs_cycle(comp.id.as_str(), &adj, &mut color, &mut parent) {
                return Some(cycle);
            }
        }
    }
    None
}

fn dfs_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    color: &mut HashMap<&'a str, u8>,
    parent: &mut HashMap<&'a str, &'a str>,
) -> Option<Vec<String>> {
    color.insert(node, 1); // gray

    if let Some(neighbors) = adj.get(node) {
        for &next in neighbors {
            match color.get(next).copied().unwrap_or(0) {
                0 => {
                    parent.insert(next, node);
                    if let Some(cycle) = dfs_cycle(next, adj, color, parent) {
                        return Some(cycle);
                    }
                }
                1 => {
                    // Found a cycle — reconstruct path.
                    let mut path = VecDeque::new();
                    path.push_front(next.to_string());
                    let mut current = node;
                    while current != next {
                        path.push_front(current.to_string());
                        current = parent.get(current).copied().unwrap_or(next);
                    }
                    path.push_front(next.to_string());
                    return Some(path.into());
                }
                _ => {} // black, already processed
            }
        }
    }

    color.insert(node, 2); // black
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::loader::parse_yaml;
    use std::path::PathBuf;

    fn validate_yaml(yaml: &str) -> ValidationReport {
        let topology = parse_yaml(yaml, &PathBuf::from("test.yaml")).unwrap();
        validate(&topology)
    }

    #[test]
    fn valid_topology() {
        let report = validate_yaml(
            r#"
name: test
components:
  - id: a
    name: A
    type: service
  - id: b
    name: B
    type: database
dependencies:
  - from: a
    to: b
    type: sync
    criticality: high
"#,
        );
        assert!(report.is_valid(), "errors: {:?}", report.errors);
    }

    #[test]
    fn duplicate_id() {
        let report = validate_yaml(
            r#"
name: test
components:
  - id: a
    name: A
    type: service
  - id: a
    name: A2
    type: cache
dependencies: []
"#,
        );
        assert!(!report.is_valid());
        assert!(report.errors[0].contains("duplicate"));
    }

    #[test]
    fn unknown_dependency_target() {
        let report = validate_yaml(
            r#"
name: test
components:
  - id: a
    name: A
    type: service
dependencies:
  - from: a
    to: nonexistent
    type: sync
    criticality: low
"#,
        );
        assert!(!report.is_valid());
        assert!(report.errors[0].contains("nonexistent"));
    }

    #[test]
    fn self_referential() {
        let report = validate_yaml(
            r#"
name: test
components:
  - id: a
    name: A
    type: service
dependencies:
  - from: a
    to: a
    type: sync
    criticality: low
"#,
        );
        assert!(!report.is_valid());
        assert!(report.errors[0].contains("self-referential"));
    }

    #[test]
    fn orphan_warning() {
        let report = validate_yaml(
            r#"
name: test
components:
  - id: a
    name: A
    type: service
  - id: orphan
    name: Orphan
    type: cache
dependencies:
  - from: a
    to: a
    type: sync
    criticality: low
"#,
        );
        assert!(report.warnings.iter().any(|w| w.contains("orphan")));
    }
}
