use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::{Error, Result};
use crate::parser::FunctionInfo;

#[derive(Debug, Clone, Default)]
pub struct CoverageMap {
    by_id: HashMap<String, f64>,
    by_name: HashMap<String, f64>,
}

impl CoverageMap {
    pub fn from_reachability(functions: &[FunctionInfo]) -> Self {
        let mut called = HashSet::new();
        for function in functions.iter().filter(|function| function.is_test) {
            for callee in &function.callees {
                called.insert(callee.clone());
            }
        }
        let mut bare_name_counts = HashMap::new();
        for function in functions.iter().filter(|function| !function.is_test) {
            *bare_name_counts
                .entry(function.name.as_str())
                .or_insert(0usize) += 1;
        }
        let mut by_id = HashMap::new();
        for function in functions {
            if function.is_test {
                continue;
            }
            let covered = called.contains(&function.qualified_name)
                || (bare_name_counts
                    .get(function.name.as_str())
                    .copied()
                    .unwrap_or(0)
                    == 1
                    && called.contains(&function.name));
            by_id.insert(function.id.clone(), if covered { 100.0 } else { 0.0 });
        }
        Self {
            by_id,
            by_name: HashMap::new(),
        }
    }

    pub fn from_llvm_cov(root: &Path, path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::MissingCoverage(path.to_path_buf()));
        }
        let source = fs::read_to_string(path).map_err(|source| Error::ReadFile {
            path: path.to_path_buf(),
            source,
        })?;
        let json: Value = serde_json::from_str(&source).map_err(|source| Error::CoverageJson {
            path: path.to_path_buf(),
            source,
        })?;
        let mut map = Self::default();
        collect_functions(root, &json, &mut map);
        Ok(map)
    }

    #[cfg(test)]
    pub fn coverage_for(&self, function: &FunctionInfo) -> f64 {
        self.coverage_match_for(function).unwrap_or(0.0)
    }

    pub fn coverage_match_for(&self, function: &FunctionInfo) -> Option<f64> {
        self.by_id
            .get(&function.id)
            .copied()
            .or_else(|| self.by_name.get(&function.qualified_name).copied())
            .or_else(|| {
                self.by_name
                    .iter()
                    .find(|(name, _)| {
                        name.ends_with(&format!("::{}", function.qualified_name))
                            || name.contains(&function.id)
                    })
                    .map(|(_, coverage)| *coverage)
            })
            .or_else(|| {
                (function.name == function.qualified_name)
                    .then(|| self.by_name.get(&function.name).copied())
                    .flatten()
            })
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty() && self.by_name.is_empty()
    }
}

fn collect_functions(root: &Path, value: &Value, map: &mut CoverageMap) {
    match value {
        Value::Object(object) => {
            if let Some(functions) = object.get("functions").and_then(Value::as_array) {
                for function in functions {
                    collect_function_entry(root, function, map);
                }
            }
            for child in object.values() {
                collect_functions(root, child, map);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_functions(root, child, map);
            }
        }
        _ => {}
    }
}

fn collect_function_entry(root: &Path, value: &Value, map: &mut CoverageMap) {
    let Some(object) = value.as_object() else {
        return;
    };
    let Some(name) = object.get("name").and_then(Value::as_str) else {
        return;
    };
    let coverage = explicit_coverage(value)
        .or_else(|| region_coverage(value))
        .or_else(|| count_coverage(value))
        .unwrap_or(0.0);
    map.by_name.insert(name.to_string(), coverage);
    if let Some(filenames) = object.get("filenames").and_then(Value::as_array) {
        for file in filenames.iter().filter_map(Value::as_str) {
            let rel = normalize_path(root, file);
            let simple_name = name.rsplit("::").next().unwrap_or(name);
            map.by_id.insert(format!("{rel}:{simple_name}"), coverage);
            map.by_id.insert(format!("{rel}:{name}"), coverage);
        }
    }
}

fn explicit_coverage(value: &Value) -> Option<f64> {
    value
        .get("coverage")
        .or_else(|| value.get("percent"))
        .or_else(|| value.get("coverage_percent"))
        .and_then(Value::as_f64)
}

fn count_coverage(value: &Value) -> Option<f64> {
    let count = value.get("count")?.as_i64()?;
    Some(if count > 0 { 100.0 } else { 0.0 })
}

fn region_coverage(value: &Value) -> Option<f64> {
    let regions = value.get("regions")?.as_array()?;
    let mut total = 0usize;
    let mut covered = 0usize;
    for region in regions {
        let Some(items) = region.as_array() else {
            continue;
        };
        total += 1;
        if items.get(4).and_then(Value::as_i64).unwrap_or(0) > 0 {
            covered += 1;
        }
    }
    if total == 0 || covered == 0 {
        None
    } else {
        Some(covered as f64 * 100.0 / total as f64)
    }
}

fn normalize_path(root: &Path, path: &str) -> String {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let normalized = path.canonicalize().unwrap_or(path);
        if let Ok(relative) = normalized.strip_prefix(&root) {
            return relative.to_string_lossy().replace('\\', "/");
        }
        return normalized.to_string_lossy().replace('\\', "/");
    }
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    components.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_ap_syntax::Edition;

    #[test]
    fn reachability_does_not_cover_same_named_methods_by_bare_name() {
        let root = Path::new("/tmp/demo");
        let file = root.join("src/lib.rs");
        let parsed = crate::parser::parse_source(
            root,
            &file,
            r#"
struct AlphaRunner;
struct BetaRunner;

impl AlphaRunner {
    fn run(&self) {}
}

impl BetaRunner {
    fn run(&self) {}
}

#[test]
fn covers_alpha() {
    AlphaRunner::run();
}
"#,
            Edition::Edition2024,
        );
        let coverage = CoverageMap::from_reachability(&parsed.functions);
        let alpha = parsed
            .functions
            .iter()
            .find(|function| function.qualified_name == "AlphaRunner::run")
            .expect("alpha");
        let beta = parsed
            .functions
            .iter()
            .find(|function| function.qualified_name == "BetaRunner::run")
            .expect("beta");
        assert_eq!(coverage.coverage_for(alpha), 100.0);
        assert_eq!(coverage.coverage_for(beta), 0.0);
    }

    #[test]
    fn region_zero_falls_back_to_function_count() {
        let root = Path::new("/tmp/demo");
        let mut map = CoverageMap::default();
        collect_function_entry(
            root,
            &serde_json::json!({
                "name": "covered",
                "filenames": ["/tmp/demo/src/lib.rs"],
                "count": 7,
                "regions": [[1, 1, 2, 1, 0, 0, 0, 0, 0]]
            }),
            &mut map,
        );
        assert_eq!(map.by_id.get("src/lib.rs:covered"), Some(&100.0));
    }

    #[test]
    fn workspace_crate_relative_paths_do_not_collapse_to_src_suffix() {
        let root = Path::new("/repo");
        let mut map = CoverageMap::default();
        collect_function_entry(
            root,
            &serde_json::json!({
                "name": "same",
                "filenames": ["/repo/crates/other/src/lib.rs"],
                "count": 1
            }),
            &mut map,
        );
        assert!(map.by_id.contains_key("crates/other/src/lib.rs:same"));
        assert!(!map.by_id.contains_key("src/lib.rs:same"));
    }
}
