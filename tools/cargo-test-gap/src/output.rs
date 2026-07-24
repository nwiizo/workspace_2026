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
    pub top: usize,
}

#[derive(Debug, Serialize)]
struct JsonReport<'a> {
    project: &'a str,
    grade: String,
    files_analyzed: usize,
    suppressed_issues: usize,
    total_candidates: usize,
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
    let issues = analysis.issues.iter().collect();
    let report = JsonReport {
        project: &analysis.project,
        grade: analysis.grade.to_string(),
        files_analyzed: analysis.files_analyzed,
        suppressed_issues: analysis.suppressed_issues,
        total_candidates: analysis.issues.len(),
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
    writeln!(writer, "Test Gap: {}", analysis.project)?;
    if options.japanese {
        writeln!(
            writer,
            "評価: {} | 解析ファイル: {} | 候補: {}",
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
            "Grade: {} | Files: {} | Candidates: {}",
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
        writeln!(
            writer,
            "{}",
            localized(
                options.japanese,
                &format!("{} 件の issue を抑制しました", analysis.suppressed_issues),
                &format!("{} issues suppressed", analysis.suppressed_issues)
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
    write_hidden_low_hint(analysis, options, writer)?;
    if options.summary {
        return write_blind_spot_pointer(analysis, options, writer);
    }
    writeln!(writer)?;
    let visible_total = visible_issue_count(analysis, options.all);
    let issues = ranked_issues(analysis, options.all, options.top);
    if issues.is_empty() && visible_total > 0 && options.top == 0 {
        writeln!(
            writer,
            "{}",
            localized(
                options.japanese,
                &format!("0 / {visible_total} 件の候補を表示 (--top 0)。"),
                &format!("0 of {visible_total} candidates shown (--top 0).")
            )
        )?;
    } else if issues.is_empty() {
        writeln!(
            writer,
            "{}",
            localized(
                options.japanese,
                "表示対象の候補はありません。",
                "No candidates in the selected severity range."
            )
        )?;
    } else {
        for (idx, issue) in issues.iter().enumerate() {
            let severity = localized_severity(issue.severity, options.japanese);
            writeln!(
                writer,
                "{}. [{}] {} {}:{} risk={:.2}",
                idx + 1,
                severity,
                issue.function,
                issue.file.display(),
                issue.line,
                issue.risk
            )?;
            writeln!(
                writer,
                "   factors: churn={:.1}, complexity={}, exposure={:.1}, coverage={:.1}%",
                issue.churn, issue.complexity, issue.exposure, issue.coverage
            )?;
            writeln!(
                writer,
                "   key: ({}, {}, {})",
                issue.key.issue_type, issue.key.source, issue.key.target
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
    writeln!(
        writer,
        "{}",
        localized(
            options.japanese,
            "# cargo-test-gap テスト優先順位",
            "# cargo-test-gap test priority plan"
        )
    )?;
    writeln!(writer, "Grade: {}", analysis.grade)?;
    write_gate(options, writer)?;
    for issue in filtered_issues(analysis, options.all) {
        writeln!(
            writer,
            "\n## {} at {}:{}",
            issue.function,
            issue.file.display(),
            issue.line
        )?;
        writeln!(writer, "- severity: {}", issue.severity)?;
        writeln!(writer, "- risk: {:.2}", issue.risk)?;
        writeln!(
            writer,
            "- factors: churn={:.1}, complexity={}, exposure={:.1}, coverage={:.1}%",
            issue.churn, issue.complexity, issue.exposure, issue.coverage
        )?;
        writeln!(
            writer,
            "- stable_key: ({}, {}, {})",
            issue.key.issue_type, issue.key.source, issue.key.target
        )?;
        writeln!(writer, "- action: {}", issue.remediation)?;
    }
    if let Some(diff) = baseline {
        writeln!(
            writer,
            "\n## Baseline diff\nnew: {}, resolved: {}, unchanged: {}",
            diff.new_issues.len(),
            diff.resolved_issues.len(),
            diff.unchanged
        )?;
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

fn ranked_issues(analysis: &Analysis, all: bool, top: usize) -> Vec<&Issue> {
    filtered_issues(analysis, all)
        .into_iter()
        .take(top)
        .collect()
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
    [
        Severity::Critical,
        Severity::High,
        Severity::Medium,
        Severity::Low,
    ]
    .into_iter()
    .map(|severity| {
        let severity_name = severity.to_string();
        let count = counts.get(&severity_name).copied().unwrap_or(0);
        format!(
            "{}={count}",
            localized_severity_by_name(&severity_name, japanese)
        )
    })
    .collect::<Vec<_>>()
    .join(", ")
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

fn visible_issue_count(analysis: &Analysis, all: bool) -> usize {
    filtered_issues(analysis, all).len()
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
