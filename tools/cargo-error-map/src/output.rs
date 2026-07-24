use std::collections::BTreeMap;
use std::io::{self, Write};

use serde::Serialize;

use crate::analyzer::Analysis;
use crate::baseline::BaselineDiff;
use crate::blind_spot::BlindSpotManifest;
use crate::issue::{Issue, Severity};
use design_gate_core::{localized, localized_severity, localized_severity_by_name};

pub use design_gate_core::GateReport;

#[derive(Debug, Clone, Copy)]
pub struct OutputOptions<'a> {
    pub all: bool,
    pub summary: bool,
    pub japanese: bool,
    pub blind_spots: bool,
    pub gate: Option<&'a GateReport>,
}

#[derive(Debug, Serialize)]
struct JsonReport<'a> {
    project: &'a str,
    grade: String,
    files_analyzed: usize,
    suppressed_issues: usize,
    issues: Vec<&'a Issue>,
    summary: BTreeMap<String, usize>,
    blind_spots: &'a BlindSpotManifest,
    baseline: Option<&'a BaselineDiff>,
    gate: Option<&'a GateReport>,
}

pub fn write_json<W: Write>(
    analysis: &Analysis,
    baseline: Option<&BaselineDiff>,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    let issues = filtered_issues(analysis, options.all);
    let report = JsonReport {
        project: &analysis.project,
        grade: analysis.grade.to_string(),
        files_analyzed: analysis.files_analyzed,
        suppressed_issues: analysis.suppressed_issues,
        summary: summary(&analysis.issues),
        issues,
        blind_spots: &analysis.blind_spots,
        baseline,
        gate: options.gate,
    };
    serde_json::to_writer_pretty(&mut *writer, &report)?;
    writeln!(writer)
}

pub fn write_text<W: Write>(
    analysis: &Analysis,
    baseline: Option<&BaselineDiff>,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    writeln!(writer, "Error Map: {}", analysis.project)?;
    if options.japanese {
        writeln!(
            writer,
            "評価: {} | 解析ファイル: {} | issue: {}",
            analysis.grade,
            analysis.files_analyzed,
            analysis.issues.len()
        )?;
        writeln!(
            writer,
            "内訳: {}",
            format_summary(&summary(&analysis.issues), true)
        )?;
    } else {
        writeln!(
            writer,
            "Grade: {} | Files: {} | Issues: {}",
            analysis.grade,
            analysis.files_analyzed,
            analysis.issues.len()
        )?;
        writeln!(
            writer,
            "Breakdown: {}",
            format_summary(&summary(&analysis.issues), false)
        )?;
    }
    if analysis.suppressed_issues > 0 {
        if options.japanese {
            writeln!(
                writer,
                "{} 件の issue を抑制しました",
                analysis.suppressed_issues
            )?;
        } else {
            writeln!(writer, "{} issues suppressed", analysis.suppressed_issues)?;
        }
    }
    if let Some(diff) = baseline {
        if options.japanese {
            writeln!(
                writer,
                "ベースライン: {} -> {} | 新規 {} | 解決 {} | 変更なし {}",
                diff.baseline_grade,
                diff.current_grade,
                diff.new_issues.len(),
                diff.resolved_issues.len(),
                diff.unchanged
            )?;
        } else {
            writeln!(
                writer,
                "Baseline: {} -> {} | new {} | resolved {} | unchanged {}",
                diff.baseline_grade,
                diff.current_grade,
                diff.new_issues.len(),
                diff.resolved_issues.len(),
                diff.unchanged
            )?;
        }
    }
    write_gate(options, writer)?;
    if options.summary {
        return write_blind_spot_pointer(analysis, options, writer);
    }
    writeln!(writer)?;
    let issues = filtered_issues(analysis, options.all);
    if issues.is_empty() {
        writeln!(
            writer,
            "{}",
            localized(
                options.japanese,
                "表示対象の issue はありません。",
                "No issues in the selected severity range."
            )
        )?;
    } else {
        for issue in issues {
            let severity = localized_severity(issue.severity, options.japanese);
            if options.japanese {
                writeln!(
                    writer,
                    "[{}] {} {}:{} ({})",
                    severity,
                    issue.issue_type().label_ja(),
                    issue.file.display(),
                    issue.line,
                    issue.layer.label_ja()
                )?;
            } else {
                writeln!(
                    writer,
                    "[{}] {} {}:{} ({})",
                    severity,
                    issue.issue_type(),
                    issue.file.display(),
                    issue.line,
                    issue.layer
                )?;
            }
            writeln!(writer, "  {}", issue.message)?;
            writeln!(
                writer,
                "  {}: {}",
                localized(options.japanese, "修正", "fix"),
                issue.remediation
            )?;
            writeln!(
                writer,
                "  {}: ({}, {}, {})",
                localized(options.japanese, "キー", "key"),
                issue.key.issue_type,
                issue.key.source,
                issue.key.target
            )?;
        }
    }
    write_blind_spot_pointer(analysis, options, writer)
}

pub fn write_ai<W: Write>(
    analysis: &Analysis,
    baseline: Option<&BaselineDiff>,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    if options.japanese {
        writeln!(writer, "# cargo-error-map 修正指示")?;
        writeln!(writer, "評価: {}", analysis.grade)?;
        writeln!(writer, "優先順に以下を修正してください。")?;
    } else {
        writeln!(writer, "# cargo-error-map repair plan")?;
        writeln!(writer, "Grade: {}", analysis.grade)?;
        writeln!(writer, "Fix these findings in priority order.")?;
    }
    if analysis.suppressed_issues > 0 {
        if options.japanese {
            writeln!(
                writer,
                "{} 件の issue を抑制しました",
                analysis.suppressed_issues
            )?;
        } else {
            writeln!(writer, "{} issues suppressed", analysis.suppressed_issues)?;
        }
    }
    write_gate(options, writer)?;
    for issue in filtered_issues(analysis, options.all) {
        if options.japanese {
            writeln!(
                writer,
                "\n## {} at {}:{}",
                issue.issue_type().label_ja(),
                issue.file.display(),
                issue.line
            )?;
            writeln!(
                writer,
                "- 重要度: {}",
                localized_severity(issue.severity, true)
            )?;
            writeln!(
                writer,
                "- stable_key: ({}, {}, {})",
                issue.key.issue_type, issue.key.source, issue.key.target
            )?;
            writeln!(writer, "- 問題: {}", issue.message)?;
            writeln!(writer, "- 修正: {}", issue.remediation)?;
            writeln!(
                writer,
                "- 検証: `cargo error-map --check` と関連する Rust テストを再実行"
            )?;
        } else {
            writeln!(
                writer,
                "\n## {} at {}:{}",
                issue.issue_type(),
                issue.file.display(),
                issue.line
            )?;
            writeln!(writer, "- severity: {}", issue.severity)?;
            writeln!(
                writer,
                "- stable_key: ({}, {}, {})",
                issue.key.issue_type, issue.key.source, issue.key.target
            )?;
            writeln!(writer, "- problem: {}", issue.message)?;
            writeln!(writer, "- repair: {}", issue.remediation)?;
            writeln!(
                writer,
                "- validation: rerun `cargo error-map --check` and relevant Rust tests"
            )?;
        }
    }
    if let Some(diff) = baseline {
        if options.japanese {
            writeln!(
                writer,
                "\n## ベースライン差分\n新規: {}, 解決: {}, 変更なし: {}",
                diff.new_issues.len(),
                diff.resolved_issues.len(),
                diff.unchanged
            )?;
        } else {
            writeln!(
                writer,
                "\n## Baseline diff\nnew: {}, resolved: {}, unchanged: {}",
                diff.new_issues.len(),
                diff.resolved_issues.len(),
                diff.unchanged
            )?;
        }
    }
    writeln!(
        writer,
        "\n## {}",
        localized(
            options.japanese,
            "未解析領域マニフェスト",
            "Blind spot manifest"
        )
    )?;
    for blind in &analysis.blind_spots.blind_spots {
        let description = if options.japanese {
            &blind.description_ja
        } else {
            &blind.description
        };
        writeln!(writer, "- {}: {}", blind.id, description)?;
    }
    for note in analysis.blind_spots.localized_notes(options.japanese) {
        writeln!(writer, "- note: {note}")?;
    }
    Ok(())
}

pub fn write_blind_spots<W: Write>(
    manifest: &BlindSpotManifest,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    writeln!(
        writer,
        "{}",
        localized(options.japanese, "Blind spots / 未解析領域", "Blind spots")
    )?;
    write_gate(options, writer)?;
    for blind in &manifest.blind_spots {
        let description = if options.japanese {
            &blind.description_ja
        } else {
            &blind.description
        };
        writeln!(writer, "- {}: {}", blind.id, description)?;
    }
    for note in manifest.localized_notes(options.japanese) {
        writeln!(writer, "- note: {note}")?;
    }
    Ok(())
}

fn filtered_issues(analysis: &Analysis, all: bool) -> Vec<&Issue> {
    analysis
        .issues
        .iter()
        .filter(|issue| all || issue.severity >= Severity::Medium)
        .collect()
}

fn summary(issues: &[Issue]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for issue in issues {
        *counts.entry(issue.severity.to_string()).or_insert(0) += 1;
    }
    counts
}

fn format_summary(counts: &BTreeMap<String, usize>, japanese: bool) -> String {
    if counts.is_empty() {
        return localized(japanese, "なし", "none").to_string();
    }
    counts
        .iter()
        .map(|(severity, count)| {
            format!("{}={count}", localized_severity_by_name(severity, japanese))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_blind_spot_pointer<W: Write>(
    analysis: &Analysis,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    if options.blind_spots {
        writeln!(writer)?;
        write_blind_spots(&analysis.blind_spots, options, writer)
    } else if options.japanese {
        writeln!(
            writer,
            "\n未解析領域の詳細は `--blind-spots` で表示できます。"
        )
    } else {
        writeln!(writer, "\nUse `--blind-spots` to show the full manifest.")
    }
}

fn write_gate<W: Write>(options: OutputOptions<'_>, writer: &mut W) -> io::Result<()> {
    let Some(gate) = options.gate else {
        return Ok(());
    };
    if options.japanese {
        writeln!(
            writer,
            "check: {} (fail-on={}, threshold 以上の issue: {})",
            if gate.passed { "PASS" } else { "FAIL" },
            gate.fail_on,
            gate.failing
        )
    } else {
        writeln!(
            writer,
            "check: {} (fail-on={}, {} issue(s) at/above threshold)",
            if gate.passed { "PASS" } else { "FAIL" },
            gate.fail_on,
            gate.failing
        )
    }
}
