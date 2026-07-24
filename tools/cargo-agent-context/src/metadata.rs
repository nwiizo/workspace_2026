use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use cargo_metadata::{DependencyKind, MetadataCommand, Package};
use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct ProjectAnalysis {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub name: String,
    pub edition: String,
    pub crate_kind: CrateKind,
    pub dependencies: DependencySummary,
    pub workspace_members: Vec<String>,
    pub build_commands: Vec<CommandInfo>,
    pub conventions: Vec<ConventionInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrateKind {
    Lib,
    Bin,
    LibAndBin,
    Workspace,
    Other(String),
}

impl CrateKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Lib => "lib",
            Self::Bin => "bin",
            Self::LibAndBin => "lib+bin",
            Self::Workspace => "workspace",
            Self::Other(value) => value,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DependencySummary {
    pub normal: Vec<String>,
    pub dev: Vec<String>,
    pub build: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub command: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ConventionInfo {
    pub path: String,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    lints: Option<Lints>,
    workspace: Option<WorkspaceToml>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceToml {
    lints: Option<Lints>,
}

#[derive(Debug, Deserialize)]
struct Lints {
    clippy: Option<toml::Value>,
}

pub fn analyze_project(path: &Path) -> Result<ProjectAnalysis> {
    let root = normalize_root(path)?;
    let manifest_path = root.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Err(Error::EmptyDirectory(root));
    }

    let mut command = MetadataCommand::new();
    command.manifest_path(&manifest_path);
    let metadata = command.exec().map_err(|source| Error::CargoMetadata {
        path: root.clone(),
        source,
    })?;

    let manifest_text = fs::read_to_string(&manifest_path).map_err(|source| Error::ReadFile {
        path: manifest_path.clone(),
        source,
    })?;
    let cargo_toml: CargoToml = toml::from_str(&manifest_text).map_err(|source| Error::Toml {
        path: manifest_path.clone(),
        source,
    })?;

    let workspace_members = workspace_member_names(&metadata);
    let root_package = metadata
        .root_package()
        .or_else(|| package_for_manifest(&metadata.packages, &manifest_path));
    let (name, edition, dependencies, crate_kind) = match root_package {
        Some(package) => (
            package.name.clone(),
            package.edition.to_string(),
            dependency_summary(package),
            crate_kind(package, workspace_members.len()),
        ),
        None => (
            root.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "workspace".to_string()),
            "unknown".to_string(),
            DependencySummary::default(),
            CrateKind::Workspace,
        ),
    };

    Ok(ProjectAnalysis {
        build_commands: build_commands(&root, &cargo_toml),
        conventions: convention_files(&root)?,
        root,
        manifest_path,
        name,
        edition,
        crate_kind,
        dependencies,
        workspace_members,
    })
}

fn normalize_root(path: &Path) -> Result<PathBuf> {
    let candidate = if path.is_file() {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        path.to_path_buf()
    };
    candidate.canonicalize().map_err(|source| Error::ReadFile {
        path: candidate,
        source,
    })
}

fn package_for_manifest<'a>(packages: &'a [Package], manifest_path: &Path) -> Option<&'a Package> {
    packages
        .iter()
        .find(|package| package.manifest_path.as_std_path() == manifest_path)
}

fn workspace_member_names(metadata: &cargo_metadata::Metadata) -> Vec<String> {
    let member_ids = metadata.workspace_members.iter().collect::<BTreeSet<_>>();
    let mut names = metadata
        .packages
        .iter()
        .filter(|package| member_ids.contains(&package.id))
        .map(|package| package.name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn dependency_summary(package: &Package) -> DependencySummary {
    let mut summary = DependencySummary::default();
    for dependency in &package.dependencies {
        match dependency.kind {
            DependencyKind::Development => summary.dev.push(dependency.name.clone()),
            DependencyKind::Build => summary.build.push(dependency.name.clone()),
            DependencyKind::Normal | DependencyKind::Unknown => {
                summary.normal.push(dependency.name.clone());
            }
        }
    }
    summary.normal.sort();
    summary.normal.dedup();
    summary.dev.sort();
    summary.dev.dedup();
    summary.build.sort();
    summary.build.dedup();
    summary
}

fn crate_kind(package: &Package, workspace_member_count: usize) -> CrateKind {
    if workspace_member_count > 1 {
        return CrateKind::Workspace;
    }
    let has_lib = package
        .targets
        .iter()
        .any(|target| target.kind.iter().any(|kind| kind == "lib"));
    let has_bin = package
        .targets
        .iter()
        .any(|target| target.kind.iter().any(|kind| kind == "bin"));
    match (has_lib, has_bin) {
        (true, true) => CrateKind::LibAndBin,
        (true, false) => CrateKind::Lib,
        (false, true) => CrateKind::Bin,
        (false, false) => CrateKind::Other(
            package
                .targets
                .iter()
                .flat_map(|target| target.kind.iter().map(ToString::to_string))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(","),
        ),
    }
}

fn build_commands(root: &Path, cargo_toml: &CargoToml) -> Vec<CommandInfo> {
    let mut commands = vec![
        CommandInfo {
            command: "cargo check".to_string(),
            reason: "Cargo.toml is present".to_string(),
        },
        CommandInfo {
            command: "cargo test".to_string(),
            reason: "Cargo.toml is present".to_string(),
        },
    ];
    if root.join("rustfmt.toml").is_file() || root.join(".rustfmt.toml").is_file() {
        commands.push(CommandInfo {
            command: "cargo fmt --check".to_string(),
            reason: "rustfmt configuration is present".to_string(),
        });
    }
    let has_clippy_config = root.join("clippy.toml").is_file()
        || root.join(".clippy.toml").is_file()
        || cargo_toml
            .lints
            .as_ref()
            .and_then(|lints| lints.clippy.as_ref())
            .is_some()
        || cargo_toml
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.lints.as_ref())
            .and_then(|lints| lints.clippy.as_ref())
            .is_some();
    if has_clippy_config {
        commands.push(CommandInfo {
            command: "cargo clippy --all-targets --all-features -- -D warnings".to_string(),
            reason: "clippy configuration or Cargo lints are present".to_string(),
        });
    }
    commands
}

fn convention_files(root: &Path) -> Result<Vec<ConventionInfo>> {
    let names = [
        "rustfmt.toml",
        ".rustfmt.toml",
        "clippy.toml",
        ".clippy.toml",
        "deny.toml",
        "CLAUDE.md",
        "AGENTS.md",
    ];
    let mut conventions = Vec::new();
    for name in names {
        let path = root.join(name);
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|source| Error::ReadFile {
            path: path.clone(),
            source,
        })?;
        conventions.push(ConventionInfo {
            path: name.to_string(),
            summary: first_lines(&text, 4),
        });
    }
    Ok(conventions)
}

fn first_lines(text: &str, max: usize) -> String {
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(max)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return "(empty file)".to_string();
    }
    for line in &mut lines {
        if let Some((end, _)) = line.char_indices().nth(120) {
            *line = &line[..end];
        }
    }
    lines.join(" / ")
}

#[cfg(test)]
mod tests {
    use super::first_lines;

    #[test]
    fn convention_summary_truncates_at_utf8_boundaries() {
        let input = "あ".repeat(121);
        let summary = first_lines(&input, 1);
        assert_eq!(summary.chars().count(), 120);
        assert!(summary.chars().all(|character| character == 'あ'));
    }
}
