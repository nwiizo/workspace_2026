use std::collections::BTreeMap;
use std::io::{self, Write};

use serde::Serialize;

use crate::analyzer::Analysis;
use crate::baseline::BaselineDiff;
use crate::blind_spot::BlindSpotManifest;
use crate::issue::{Issue, Severity};

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
    runtime: &'static str,
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
        runtime: analysis.runtime.id(),
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
    writeln!(writer, "Async Smell: {}", analysis.project)?;
    if options.japanese {
        writeln!(
            writer,
            "評価: {} | runtime: {} | 解析ファイル: {} | issue: {}",
            analysis.grade,
            analysis.runtime.id(),
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
            "Grade: {} | Runtime: {} | Files: {} | Issues: {}",
            analysis.grade,
            analysis.runtime.id(),
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
        writeln!(
            writer,
            "{}",
            localized_count(
                options.japanese,
                analysis.suppressed_issues,
                "件の issue を抑制しました",
                "issues suppressed"
            )
        )?;
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
                    issue.risk.label_ja()
                )?;
            } else {
                writeln!(
                    writer,
                    "[{}] {} {}:{} ({})",
                    severity,
                    issue.issue_type(),
                    issue.file.display(),
                    issue.line,
                    issue.risk
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
    write_hidden_low_hint(analysis, options, writer)?;
    write_blind_spot_pointer(analysis, options, writer)
}

pub fn write_ai<W: Write>(
    analysis: &Analysis,
    baseline: Option<&BaselineDiff>,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    if options.japanese {
        writeln!(writer, "# cargo-async-smell 修正指示")?;
        writeln!(writer, "評価: {}", analysis.grade)?;
        writeln!(writer, "優先順に以下を修正してください。")?;
    } else {
        writeln!(writer, "# cargo-async-smell repair plan")?;
        writeln!(writer, "Grade: {}", analysis.grade)?;
        writeln!(writer, "Fix these findings in priority order.")?;
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
            writeln!(writer, "- risk: {}", issue.risk.label_ja())?;
            writeln!(
                writer,
                "- stable_key: ({}, {}, {})",
                issue.key.issue_type, issue.key.source, issue.key.target
            )?;
            writeln!(writer, "- 問題: {}", issue.message)?;
            writeln!(writer, "- 修正: {}", issue.remediation)?;
            writeln!(
                writer,
                "- 検証: `cargo async-smell --check` と関連する Rust テストを再実行"
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
            writeln!(writer, "- risk: {}", issue.risk)?;
            writeln!(
                writer,
                "- stable_key: ({}, {}, {})",
                issue.key.issue_type, issue.key.source, issue.key.target
            )?;
            writeln!(writer, "- problem: {}", issue.message)?;
            writeln!(writer, "- repair: {}", issue.remediation)?;
            writeln!(
                writer,
                "- validation: rerun `cargo async-smell --check` and relevant Rust tests"
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

fn write_hidden_low_hint<W: Write>(
    analysis: &Analysis,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    let hidden = hidden_low_count(analysis, options.all);
    if hidden == 0 {
        return Ok(());
    }
    if options.japanese {
        writeln!(
            writer,
            "hint: 低 severity issue {hidden} 件を非表示にしています。--all を使うと表示します。"
        )
    } else {
        writeln!(
            writer,
            "hint: {hidden} low-severity issues hidden, use --all"
        )
    }
}

fn hidden_low_count(analysis: &Analysis, all: bool) -> usize {
    if all {
        return 0;
    }
    analysis
        .issues
        .iter()
        .filter(|issue| issue.severity == Severity::Low)
        .count()
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

fn localized<'a>(japanese: bool, ja: &'a str, en: &'a str) -> &'a str {
    if japanese { ja } else { en }
}

fn localized_count(japanese: bool, count: usize, ja_suffix: &str, en_suffix: &str) -> String {
    if japanese {
        format!("{count} {ja_suffix}")
    } else {
        format!("{count} {en_suffix}")
    }
}

fn localized_severity(severity: Severity, japanese: bool) -> &'static str {
    if !japanese {
        return match severity {
            Severity::Low => "Low",
            Severity::Medium => "Medium",
            Severity::High => "High",
            Severity::Critical => "Critical",
        };
    }
    match severity {
        Severity::Low => "低",
        Severity::Medium => "中",
        Severity::High => "高",
        Severity::Critical => "致命的",
    }
}

fn localized_severity_by_name(severity: &str, japanese: bool) -> &str {
    if !japanese {
        return severity;
    }
    match severity {
        "Low" => "低",
        "Medium" => "中",
        "High" => "高",
        "Critical" => "致命的",
        _ => severity,
    }
}
