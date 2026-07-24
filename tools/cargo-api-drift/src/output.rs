use std::collections::BTreeMap;
use std::io::{self, Write};

use serde::Serialize;

use crate::analyzer::Analysis;
use crate::blind_spot;
use crate::issue::{ChangeKind, Classification, Issue, Severity};
use design_gate_core::{GateReport, localized, localized_severity, localized_severity_by_name};

#[derive(Debug, Clone, Copy)]
pub struct OutputOptions<'a> {
    pub all: bool,
    pub summary: bool,
    pub japanese: bool,
    pub gate: Option<&'a GateReport>,
}

#[derive(Debug, Serialize)]
struct JsonReport<'a> {
    project: &'a str,
    against: &'a str,
    grade: String,
    files_analyzed: usize,
    suppressed_issues: usize,
    issues: Vec<&'a Issue>,
    breakdown: BTreeMap<String, usize>,
    blind_spots: &'a design_gate_core::BlindSpotManifest,
    gate: Option<&'a GateReport>,
}

pub fn write_json<W: Write>(
    analysis: &Analysis,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    let issues = filtered_issues(analysis, options.all);
    let report = JsonReport {
        project: &analysis.project,
        against: &analysis.against,
        grade: analysis.grade.to_string(),
        files_analyzed: analysis.files_analyzed,
        suppressed_issues: analysis.suppressed_issues,
        breakdown: summary(issues.iter().copied()),
        issues,
        blind_spots: &analysis.blind_spots,
        gate: options.gate,
    };
    serde_json::to_writer_pretty(&mut *writer, &report)?;
    writeln!(writer)
}

pub fn write_text<W: Write>(
    analysis: &Analysis,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    let issues = filtered_issues(analysis, options.all);
    writeln!(writer, "API Drift: {}", analysis.project)?;
    if options.japanese {
        writeln!(
            writer,
            "評価: {} | 解析ファイル: {} | issue: {}",
            analysis.grade,
            analysis.files_analyzed,
            issues.len()
        )?;
        writeln!(
            writer,
            "内訳: {}",
            format_summary(&summary(issues.iter().copied()), true)
        )?;
        write_gate(options, writer)?;
        writeln!(
            writer,
            "比較対象: {} | issue {}",
            analysis.against,
            issues.len()
        )?;
    } else {
        writeln!(
            writer,
            "Grade: {} | Files: {} | Issues: {}",
            analysis.grade,
            analysis.files_analyzed,
            issues.len()
        )?;
        writeln!(
            writer,
            "Breakdown: {}",
            format_summary(&summary(issues.iter().copied()), false)
        )?;
        write_gate(options, writer)?;
        writeln!(
            writer,
            "Against: {} | issues {}",
            analysis.against,
            issues.len()
        )?;
    }
    if analysis.suppressed_issues > 0 {
        writeln!(writer, "{} issues suppressed", analysis.suppressed_issues)?;
    }
    if options.summary {
        return write_blind_spot_pointer(analysis, options, writer);
    }
    writeln!(writer)?;
    if issues.is_empty() {
        writeln!(
            writer,
            "{}",
            localized(
                options.japanese,
                "表示対象の API drift はありません。",
                "No API drift in the selected severity range."
            )
        )?;
    } else {
        for issue in issues {
            write_issue(issue, options.japanese, writer)?;
        }
    }
    write_blind_spot_pointer(analysis, options, writer)
}

pub fn write_ai<W: Write>(
    analysis: &Analysis,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    if options.japanese {
        writeln!(writer, "# cargo-api-drift API 変更レビュー")?;
        writeln!(writer, "比較対象: {}", analysis.against)?;
        writeln!(
            writer,
            "厳密な semver audit には cargo-semver-checks を併用してください。"
        )?;
    } else {
        writeln!(writer, "# cargo-api-drift API change review")?;
        writeln!(writer, "Against: {}", analysis.against)?;
        writeln!(writer, "Use cargo-semver-checks for strict semver audits.")?;
    }
    write_gate(options, writer)?;
    for issue in filtered_issues(analysis, options.all) {
        writeln!(
            writer,
            "\n## {} {} at {}:{}",
            issue.classification(),
            issue.change_kind,
            issue.file.display(),
            issue.line
        )?;
        writeln!(writer, "- severity: {}", issue.severity)?;
        writeln!(
            writer,
            "- stable_key: ({}, {}, {})",
            issue.key.classification, issue.key.source, issue.key.target
        )?;
        writeln!(writer, "- change: {}", issue.message)?;
        writeln!(writer, "- action: {}", issue.remediation)?;
    }
    writeln!(writer, "\n## Blind spot manifest")?;
    for blind in &analysis.blind_spots.blind_spots {
        writeln!(
            writer,
            "- {}: {}",
            blind.id,
            blind.localized_description(options.japanese)
        )?;
    }
    for note in analysis.blind_spots.localized_notes(options.japanese) {
        writeln!(writer, "- note: {note}")?;
    }
    Ok(())
}

pub fn write_blind_spots<W: Write>(japanese: bool, writer: &mut W) -> io::Result<()> {
    let manifest = blind_spot::build(0);
    writeln!(
        writer,
        "{}",
        localized(japanese, "Blind spots / 未解析領域", "Blind spots")
    )?;
    for blind in &manifest.blind_spots {
        writeln!(
            writer,
            "- {}: {}",
            blind.id,
            blind.localized_description(japanese)
        )?;
    }
    Ok(())
}

pub fn write_changelog<W: Write>(
    analysis: &Analysis,
    gate: Option<&GateReport>,
    writer: &mut W,
) -> io::Result<()> {
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    for issue in filtered_issues(analysis, true) {
        let line = format!(
            "- **{}** `{}`: {}",
            issue.classification(),
            issue.key.source,
            issue.message
        );
        match changelog_bucket(issue.change_kind) {
            "Added" => added.push(line),
            "Removed" => removed.push(line),
            _ => changed.push(line),
        }
    }
    writeln!(writer, "## [Unreleased]")?;
    if let Some(gate) = gate {
        writeln!(
            writer,
            "\ncheck: {} (fail-on={}, {} issue(s) at/above threshold)",
            if gate.passed { "PASS" } else { "FAIL" },
            gate.fail_on,
            gate.failing
        )?;
    }
    write_bucket("Added", &added, writer)?;
    write_bucket("Changed", &changed, writer)?;
    write_bucket("Removed", &removed, writer)?;
    Ok(())
}

fn write_bucket<W: Write>(name: &str, entries: &[String], writer: &mut W) -> io::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    writeln!(writer, "\n### {name}")?;
    for entry in entries {
        writeln!(writer, "{entry}")?;
    }
    Ok(())
}

fn changelog_bucket(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added
        | ChangeKind::FieldAdded
        | ChangeKind::VariantAdded
        | ChangeKind::TraitMethodAdded => "Added",
        ChangeKind::Removed
        | ChangeKind::FieldRemoved
        | ChangeKind::VariantRemoved
        | ChangeKind::ReExportRemoved => "Removed",
        ChangeKind::SignatureChanged
        | ChangeKind::CfgChanged
        | ChangeKind::BoundAdded
        | ChangeKind::BoundRemoved
        | ChangeKind::FieldTypeChanged
        | ChangeKind::TraitMethodDefaultRemoved
        | ChangeKind::DeriveRemoved
        | ChangeKind::ReprChanged
        | ChangeKind::OrderChanged => "Changed",
    }
}

fn write_issue<W: Write>(issue: &Issue, japanese: bool, writer: &mut W) -> io::Result<()> {
    if japanese {
        writeln!(
            writer,
            "[{}] {} {} {}:{}",
            localized_severity(issue.severity, true),
            issue.classification().label_ja(),
            issue.change_kind,
            issue.file.display(),
            issue.line
        )?;
    } else {
        writeln!(
            writer,
            "[{}] {} {} {}:{}",
            localized_severity(issue.severity, false),
            issue.classification(),
            issue.change_kind,
            issue.file.display(),
            issue.line
        )?;
    }
    writeln!(writer, "  {}", issue.message)?;
    writeln!(
        writer,
        "  {} {}",
        localized(japanese, "修正:", "fix:"),
        issue.remediation
    )?;
    writeln!(
        writer,
        "  {} ({}, {}, {})",
        localized(japanese, "キー:", "key:"),
        issue.key.classification,
        issue.key.source,
        issue.key.target
    )
}

fn filtered_issues(analysis: &Analysis, all: bool) -> Vec<&Issue> {
    analysis
        .issues
        .iter()
        .filter(|issue| all || issue.severity >= Severity::Medium)
        .collect()
}

fn summary<'a>(issues: impl Iterator<Item = &'a Issue>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for issue in issues {
        *counts
            .entry(issue.classification().to_string())
            .or_insert(0) += 1;
    }
    counts
}

fn format_summary(counts: &BTreeMap<String, usize>, japanese: bool) -> String {
    if counts.is_empty() {
        return localized(japanese, "なし", "none").to_string();
    }
    counts
        .iter()
        .map(|(classification, count)| {
            let label = match classification.as_str() {
                "breaking" => localized(japanese, "破壊的変更", "breaking"),
                "risky" => localized(japanese, "リスク変更", "risky"),
                "safe" => localized(japanese, "安全な変更", "safe"),
                other => other,
            };
            format!("{label}={count}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_blind_spot_pointer<W: Write>(
    _analysis: &Analysis,
    options: OutputOptions<'_>,
    writer: &mut W,
) -> io::Result<()> {
    if options.japanese {
        writeln!(
            writer,
            "\n未解析領域の詳細は `--blind-spots` で表示できます。厳密な semver audit には cargo-semver-checks を使ってください。"
        )
    } else {
        writeln!(
            writer,
            "\nUse `--blind-spots` for limitations. Use cargo-semver-checks for strict semver audits."
        )
    }
}

fn write_gate<W: Write>(options: OutputOptions<'_>, writer: &mut W) -> io::Result<()> {
    let Some(gate) = options.gate else {
        return Ok(());
    };
    let fail_on_name = gate.fail_on.to_string();
    let fail_on = localized_severity_by_name(&fail_on_name, options.japanese);
    if options.japanese {
        writeln!(
            writer,
            "check: {} (fail-on={}, threshold 以上の issue: {})",
            if gate.passed { "PASS" } else { "FAIL" },
            fail_on,
            gate.failing
        )
    } else {
        writeln!(
            writer,
            "check: {} (fail-on={}, {} issue(s) at/above threshold)",
            if gate.passed { "PASS" } else { "FAIL" },
            fail_on,
            gate.failing
        )
    }
}

#[allow(dead_code)]
fn _classification_for_docs(_: Classification) {}
