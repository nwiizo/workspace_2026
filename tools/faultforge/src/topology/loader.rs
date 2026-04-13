use crate::error::{FaultForgeError, Result};
use crate::topology::model::Topology;
use std::path::Path;

/// Load a topology from a YAML file.
pub fn load_yaml(path: &Path) -> Result<Topology> {
    let content = std::fs::read_to_string(path).map_err(|source| FaultForgeError::FileRead {
        path: path.to_path_buf(),
        source,
    })?;
    parse_yaml(&content, path)
}

/// Parse a topology from a YAML string.
pub fn parse_yaml(content: &str, path: &Path) -> Result<Topology> {
    serde_yml::from_str(content).map_err(|source| FaultForgeError::YamlParse {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_minimal_topology() {
        let yaml = r#"
name: "test"
components:
  - id: svc-a
    name: "Service A"
    type: service
dependencies: []
"#;
        let topology = parse_yaml(yaml, &PathBuf::from("test.yaml")).unwrap();
        assert_eq!(topology.name, "test");
        assert_eq!(topology.components.len(), 1);
        assert_eq!(topology.components[0].redundancy, 1);
    }

    #[test]
    fn parse_with_dependencies() {
        let yaml = r#"
name: "test"
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
    criticality: critical
    has_fallback: false
"#;
        let topology = parse_yaml(yaml, &PathBuf::from("test.yaml")).unwrap();
        assert_eq!(topology.dependencies.len(), 1);
        assert_eq!(topology.dependencies[0].from, "a");
        assert_eq!(topology.dependencies[0].to, "b");
    }
}
