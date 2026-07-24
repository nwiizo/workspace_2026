use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use cargo_metadata::MetadataCommand;
use design_gate_core::{RustFileWalkerOptions, relative_path, rust_files};
use rayon::prelude::*;

use crate::config::BoundaryConfig;
use crate::error::{BoundaryError, Result};
use crate::git::{GitInfo, Volatility};
use crate::lints::{FileContext, LayerContext, run_lints};
use crate::model::{BlindSpot, BlindSpotManifest, BoundaryReport, LayerInfo, Summary};
use crate::parser::{ParsedFile, parse_file};
use crate::scoring;

#[derive(Debug, Clone, Copy)]
pub struct AnalysisOptions {
    pub include_low: bool,
}

#[derive(Debug)]
struct SourceFile {
    parsed: ParsedFile,
    relative: PathBuf,
    module: String,
    layer: Option<LayerContext>,
    volatility: Volatility,
}

pub fn analyze_path(path: &Path, options: &AnalysisOptions) -> Result<BoundaryReport> {
    let root = analysis_root(path);
    let (project, metadata_error) = project_name(&root);
    let config = BoundaryConfig::discover(&root)?;
    let git = GitInfo::discover(&root)?;
    let rust_files = rust_files(&root, RustFileWalkerOptions::default())?;
    if rust_files.is_empty() {
        return Err(BoundaryError::NoRustFiles(root));
    }
    let parsed: Vec<ParsedFile> = rust_files
        .par_iter()
        .map(|file| parse_file(file))
        .collect::<Result<Vec<_>>>()?;

    let mut layer_evidence: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut layer_notes = Vec::new();
    let source_files: Vec<SourceFile> = parsed
        .into_iter()
        .map(|parsed| {
            let relative = relative_path(&root, &parsed.path);
            let parsed = normalize_parsed_file_paths(parsed, &relative);
            let module = module_path(&relative);
            let layer_match = config.layer_for_module(&module, &relative);
            if let Some(layer_match) = &layer_match {
                let evidence = format!(
                    "{} -> {} ({})",
                    relative.display(),
                    layer_match.name,
                    layer_match.evidence.join(", ")
                );
                layer_evidence
                    .entry(layer_match.name.clone())
                    .or_default()
                    .push(evidence);
                for item in &layer_match.evidence {
                    if item.contains("ambiguous layer match") {
                        layer_notes.push(format!("{}: {item}", relative.display()));
                    }
                }
            }
            let volatility = git.as_ref().map_or(Volatility::Unknown, |info| {
                info.volatility_for(&parsed.path)
            });
            SourceFile {
                parsed,
                relative,
                module,
                layer: layer_match.map(|layer| LayerContext {
                    name: layer.name,
                    rank: layer.rank,
                }),
                volatility,
            }
        })
        .collect();

    let contexts: Vec<FileContext<'_>> = source_files
        .iter()
        .map(|file| FileContext {
            parsed: &file.parsed,
            module: &file.module,
            layer: file.layer.clone(),
            volatility: file.volatility,
        })
        .collect();
    let all_issues = run_lints(&contexts, &config);
    let score = scoring::project_score(&all_issues);
    let grade = scoring::grade(score);
    let mut layers = config.layer_infos();
    attach_layer_evidence(&mut layers, layer_evidence);
    let blind_spots = blind_spots(
        &config,
        git.is_some(),
        &source_files,
        metadata_error,
        layer_notes,
        false,
    );
    let summary = Summary::from_issues(source_files.len(), &all_issues);
    Ok(BoundaryReport {
        project,
        root,
        score,
        grade,
        summary,
        issues: all_issues,
        layers,
        blind_spots,
        baseline: None,
        include_low: options.include_low,
        no_rust_files: false,
        gate: None,
    })
}

fn normalize_parsed_file_paths(mut parsed: ParsedFile, relative: &Path) -> ParsedFile {
    parsed.path = relative.to_path_buf();
    for import in &mut parsed.imports {
        import.location.file = relative.to_path_buf();
    }
    for reference in &mut parsed.path_refs {
        reference.location.file = relative.to_path_buf();
    }
    for item in &mut parsed.pub_items {
        item.location.file = relative.to_path_buf();
    }
    parsed
}

fn analysis_root(path: &Path) -> PathBuf {
    if path.is_file() {
        return path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    }
    path.to_path_buf()
}

fn project_name(root: &Path) -> (String, Option<String>) {
    let mut command = MetadataCommand::new();
    command.current_dir(root).no_deps();
    match command.exec() {
        Ok(metadata) => {
            if let Some(package) = metadata.root_package() {
                (package.name.to_string(), None)
            } else {
                (fallback_project_name(root), None)
            }
        }
        Err(error) => (
            fallback_project_name(root),
            Some(format!("cargo metadata failed: {error}")),
        ),
    }
}

fn fallback_project_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| "project".to_string(), ToString::to_string)
}

fn module_path(relative: &Path) -> String {
    let mut components: Vec<String> = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(text) => text.to_str().map(ToString::to_string),
            _ => None,
        })
        .collect();
    if components.first().is_some_and(|part| part == "src") {
        components.remove(0);
    }
    if let Some(last) = components.last_mut()
        && last.ends_with(".rs")
    {
        let trimmed = last.trim_end_matches(".rs").to_string();
        *last = trimmed;
    }
    if components.last().is_some_and(|part| part == "mod") {
        components.pop();
    }
    if components.len() == 1
        && components
            .first()
            .is_some_and(|part| part == "lib" || part == "main")
    {
        return "crate".to_string();
    }
    if components.is_empty() {
        "crate".to_string()
    } else {
        components
            .into_iter()
            .map(|part| part.replace('-', "_"))
            .collect::<Vec<_>>()
            .join("::")
    }
}

fn attach_layer_evidence(layers: &mut [LayerInfo], mut evidence: BTreeMap<String, Vec<String>>) {
    for layer in layers {
        if let Some(mut entries) = evidence.remove(&layer.name) {
            entries.sort();
            entries.truncate(8);
            layer.evidence = entries;
        }
    }
}

fn blind_spots(
    config: &BoundaryConfig,
    git_used: bool,
    files: &[SourceFile],
    metadata_error: Option<String>,
    layer_notes: Vec<String>,
    no_rust_files: bool,
) -> BlindSpotManifest {
    let mut blind_spots = vec![
        BlindSpot {
            id: "macro-and-cfg".to_string(),
            description: "Macro-expanded code and inactive cfg branches are not analyzed unless they exist as source text.".to_string(),
            description_ja: "macro 展開後のコードと無効な cfg 分岐は、ソーステキストとして存在しない限り解析しません。".to_string(),
        },
        BlindSpot {
            id: "type-resolution".to_string(),
            description: "The analyzer uses syntactic paths, calls, and imports; trait method dispatch, re-exports, and generated names may be missed.".to_string(),
            description_ja: "解析器は構文上の path・呼び出し・import を使います。trait method dispatch、re-export、生成名は見逃す場合があります。".to_string(),
        },
        BlindSpot {
            id: "pub-leak-approximation".to_string(),
            description: "pub-leak is approximate because method calls are not type-resolved; matching bare names can hide an actually unused public item.".to_string(),
            description_ja: "pub-leak は近似です。メソッド呼び出しは型解決しないため、同名識別子によって実際には未使用の pub item を見逃す場合があります。".to_string(),
        },
        BlindSpot {
            id: "runtime-coupling".to_string(),
            description: "Runtime ordering, timing, shared state, and protocol coupling are outside static AST analysis.".to_string(),
            description_ja: "実行時の順序、タイミング、共有状態、protocol coupling は静的 AST 解析の対象外です。".to_string(),
        },
    ];
    let mut notes = Vec::new();
    let mut notes_ja = Vec::new();
    if no_rust_files {
        notes.push("no Rust files found under this path".to_string());
        notes_ja.push("この path 配下に Rust ファイルが見つかりませんでした".to_string());
    }
    if config.used_heuristics {
        notes.push(
            "boundary.toml was not found; layers were inferred from directory and module names."
                .to_string(),
        );
        notes_ja.push(
            "boundary.toml が見つからなかったため、directory 名と module 名から層を推定しました。"
                .to_string(),
        );
        blind_spots.push(BlindSpot {
            id: "heuristic-layers".to_string(),
            description: "Layer assignment is heuristic. Add boundary.toml to make the design contract explicit.".to_string(),
            description_ja: "層の割り当ては heuristic です。boundary.toml を追加して設計上の契約を明示してください。".to_string(),
        });
    }
    if !git_used {
        notes.push(
            "git history was not available; volatility was treated as unknown and scores use depth x occurrences only."
                .to_string(),
        );
        notes_ja.push(
            "git 履歴を利用できなかったため、volatility は unknown として扱い、score は depth x occurrences のみで計算しました。"
                .to_string(),
        );
        blind_spots.push(BlindSpot {
            id: "volatility".to_string(),
            description: "Git log volatility could not be calculated, so severity excludes change-frequency amplification.".to_string(),
            description_ja: "git log の volatility を計算できないため、severity には変更頻度による増幅が含まれません。".to_string(),
        });
    }
    if let Some(error) = metadata_error {
        notes.push(error.clone());
        notes_ja.push(format!("{error}。crate 名は path 名から推定しました。"));
    }
    let parse_failures: usize = files.iter().map(|file| file.parsed.parse_error_count).sum();
    if parse_failures > 0 {
        notes.push(format!(
            "{parse_failures} parse error(s) were reported by ra_ap_syntax; syntactic extraction continued best-effort."
        ));
        notes_ja.push(format!(
            "ra_ap_syntax が {parse_failures} 件の parse error を報告しました。構文抽出は best-effort で継続しました。"
        ));
    }
    for note in layer_notes {
        notes.push(format!("ambiguous layer inference: {note}"));
        notes_ja.push(format!("層推定が曖昧です: {note}"));
    }
    let unlayered = files.iter().filter(|file| file.layer.is_none()).count();
    if unlayered > 0 {
        notes.push(format!(
            "{unlayered} file(s) did not match any declared or inferred layer."
        ));
        notes_ja.push(format!(
            "{unlayered} 個のファイルが宣言済みまたは推定された層に一致しませんでした。"
        ));
    }
    let relative_examples: Vec<String> = files
        .iter()
        .filter(|file| file.layer.is_none())
        .take(5)
        .map(|file| file.relative.display().to_string())
        .collect();
    if !relative_examples.is_empty() {
        notes.push(format!(
            "unlayered examples: {}",
            relative_examples.join(", ")
        ));
        notes_ja.push(format!("層未判定の例: {}", relative_examples.join(", ")));
    }
    BlindSpotManifest {
        blind_spots,
        notes,
        notes_ja,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_path_handles_mod_rs() {
        assert_eq!(module_path(Path::new("src/domain/mod.rs")), "domain");
        assert_eq!(
            module_path(Path::new("src/domain/order.rs")),
            "domain::order"
        );
        assert_eq!(module_path(Path::new("src/lib.rs")), "crate");
    }
}
