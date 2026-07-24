use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::{BoundaryError, Result};
use crate::model::{LayerInfo, LayerSource};

#[derive(Debug, Clone)]
pub struct BoundaryConfig {
    pub layers: Vec<LayerRule>,
    pub allowed: Vec<AllowRule>,
    pub forbidden_imports: Vec<ForbiddenImportRule>,
    pub used_heuristics: bool,
}

#[derive(Debug, Clone)]
pub struct LayerRule {
    pub name: String,
    pub rank: usize,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AllowRule {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForbiddenImportRule {
    pub layer: String,
    #[serde(default, alias = "imports")]
    pub crates: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    layers: Vec<RawLayer>,
    #[serde(default, alias = "allowed")]
    allow: Vec<AllowRule>,
    #[serde(default)]
    forbidden_imports: Vec<ForbiddenImportRule>,
}

#[derive(Debug, Deserialize)]
struct RawLayer {
    name: String,
    #[serde(default)]
    paths: Vec<String>,
    rank: Option<usize>,
}

impl BoundaryConfig {
    pub fn discover(root: &Path) -> Result<Self> {
        let config_path = root.join("boundary.toml");
        if config_path.is_file() {
            return Self::from_file(&config_path);
        }
        Ok(Self::heuristic())
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let source = fs::read_to_string(path).map_err(|source| BoundaryError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let raw: RawConfig = toml::from_str(&source).map_err(|source| BoundaryError::Toml {
            path: path.to_path_buf(),
            source,
        })?;
        let layers = raw
            .layers
            .into_iter()
            .enumerate()
            .map(|(index, layer)| LayerRule {
                name: layer.name,
                rank: layer.rank.unwrap_or(index),
                paths: layer.paths,
            })
            .collect();
        Ok(Self {
            layers,
            allowed: raw.allow,
            forbidden_imports: raw.forbidden_imports,
            used_heuristics: false,
        })
    }

    pub fn heuristic() -> Self {
        Self {
            layers: vec![
                LayerRule {
                    name: "domain".to_string(),
                    rank: 0,
                    paths: vec![
                        "domain".to_string(),
                        "core".to_string(),
                        "entity".to_string(),
                        "entities".to_string(),
                        "model".to_string(),
                        "models".to_string(),
                    ],
                },
                LayerRule {
                    name: "application".to_string(),
                    rank: 1,
                    paths: vec![
                        "app".to_string(),
                        "application".to_string(),
                        "usecase".to_string(),
                        "usecases".to_string(),
                        "service".to_string(),
                        "services".to_string(),
                    ],
                },
                LayerRule {
                    name: "infrastructure".to_string(),
                    rank: 2,
                    paths: vec![
                        "infra".to_string(),
                        "infrastructure".to_string(),
                        "adapter".to_string(),
                        "adapters".to_string(),
                        "repository".to_string(),
                        "repositories".to_string(),
                        "persistence".to_string(),
                        "db".to_string(),
                    ],
                },
                LayerRule {
                    name: "presentation".to_string(),
                    rank: 3,
                    paths: vec![
                        "presentation".to_string(),
                        "handler".to_string(),
                        "handlers".to_string(),
                        "api".to_string(),
                        "controller".to_string(),
                        "controllers".to_string(),
                        "route".to_string(),
                        "routes".to_string(),
                        "web".to_string(),
                    ],
                },
            ],
            allowed: Vec::new(),
            forbidden_imports: Vec::new(),
            used_heuristics: true,
        }
    }

    pub fn layer_for_module(&self, module: &str, relative_file: &Path) -> Option<LayerMatch> {
        let module_parts: Vec<&str> = module.split("::").filter(|part| !part.is_empty()).collect();
        let directory_parts: Vec<String> = relative_file
            .parent()
            .map(|parent| {
                parent
                    .components()
                    .filter_map(|component| component.as_os_str().to_str())
                    .filter(|part| *part != "src")
                    .map(|part| part.replace('-', "_"))
                    .collect()
            })
            .unwrap_or_default();
        let directory_refs: Vec<&str> = directory_parts.iter().map(String::as_str).collect();
        if let Some(best) = self.best_layer_match(&directory_refs, "directory segment") {
            return Some(best);
        }
        self.best_layer_match(&module_parts, "module segment")
    }

    fn best_layer_match(&self, parts: &[&str], evidence_kind: &str) -> Option<LayerMatch> {
        let mut matches = Vec::new();
        for layer in &self.layers {
            for pattern in &layer.paths {
                for (index, part) in parts.iter().enumerate() {
                    if part == pattern {
                        matches.push((
                            index,
                            LayerMatch {
                                name: layer.name.clone(),
                                rank: layer.rank,
                                evidence: vec![format!("{evidence_kind} `{pattern}`")],
                            },
                        ));
                    }
                }
            }
        }
        let max_index = matches.iter().map(|(index, _)| *index).max()?;
        let mut deepest: Vec<LayerMatch> = matches
            .into_iter()
            .filter_map(|(index, layer_match)| (index == max_index).then_some(layer_match))
            .collect();
        deepest.sort_by(|left, right| left.rank.cmp(&right.rank).then(left.name.cmp(&right.name)));
        let mut selected = deepest.remove(0);
        if !deepest.is_empty() {
            let mut layers = vec![selected.name.clone()];
            layers.extend(deepest.into_iter().map(|layer| layer.name));
            layers.sort();
            layers.dedup();
            selected.evidence.push(format!(
                "ambiguous layer match among {}; selected {}",
                layers.join(", "),
                selected.name
            ));
        }
        Some(selected)
    }

    pub fn layer_for_path_string(&self, path: &str) -> Option<LayerMatch> {
        let normalized = normalize_path(path);
        let parts: Vec<&str> = normalized
            .split("::")
            .filter(|part| !part.is_empty() && *part != "crate" && *part != "self")
            .collect();
        self.best_layer_match(&parts, "path segment")
    }

    pub fn is_allowed(
        &self,
        source: &str,
        target: &str,
        source_rank: usize,
        target_rank: usize,
    ) -> bool {
        if source == target {
            return true;
        }
        let rank_ok = source_rank >= target_rank;
        let explicit_allow = self
            .allowed
            .iter()
            .any(|rule| rule.from == source && rule.to == target);
        rank_ok || explicit_allow
    }

    pub fn layer_infos(&self) -> Vec<LayerInfo> {
        self.layers
            .iter()
            .map(|layer| LayerInfo {
                name: layer.name.clone(),
                rank: layer.rank,
                paths: layer.paths.clone(),
                source: if self.used_heuristics {
                    LayerSource::Heuristic
                } else {
                    LayerSource::Config
                },
                evidence: Vec::new(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct LayerMatch {
    pub name: String,
    pub rank: usize,
    pub evidence: Vec<String>,
}

fn normalize_path(path: &str) -> String {
    path.replace("super::", "")
}
