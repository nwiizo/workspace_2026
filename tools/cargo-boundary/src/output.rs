use std::io::{self, Write};

use crate::error::Result;
use crate::model::{BoundaryReport, GateReport, Issue, Severity};
use design_gate_core::localized_severity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Summary,
    Json,
    Ai,
    BlindSpots,
    Layers,
}

pub fn print_report(report: &BoundaryReport, mode: OutputMode, japanese: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    match mode {
        OutputMode::Json => {
            writeln!(lock, "{}", serde_json::to_string_pretty(report)?)?;
            return Ok(());
        }
        OutputMode::Ai => print_ai(report, japanese, &mut lock)?,
        OutputMode::Summary => {
            print_summary(report, japanese, &mut lock, true)?;
        }
        OutputMode::BlindSpots => {
            print_blind_spots(report, japanese, &mut lock)?;
            print_report_gate(report, &mut lock)?;
        }
        OutputMode::Layers => {
            print_layers(report, japanese, &mut lock)?;
            print_report_gate(report, &mut lock)?;
        }
        OutputMode::Human => print_human(report, japanese, &mut lock)?,
    }
    Ok(())
}

fn print_human<W: Write>(
    report: &BoundaryReport,
    japanese: bool,
    writer: &mut W,
) -> io::Result<()> {
    print_summary(report, japanese, writer, true)?;
    let issues = visible_issues(report);
    if issues.is_empty() {
        if japanese {
            writeln!(writer, "表示対象の境界 issue はありません。")?;
        } else {
            writeln!(writer, "No visible boundary issues found.")?;
        }
    } else {
        writeln!(writer)?;
        for issue in issues {
            print_issue(issue, japanese, writer)?;
        }
    }
    if let Some(diff) = &report.baseline {
        writeln!(writer)?;
        if japanese {
            writeln!(
                writer,
                "baseline {}: new {} / resolved {} / unchanged {}",
                diff.git_ref,
                diff.new_issues.len(),
                diff.resolved_issues.len(),
                diff.unchanged
            )?;
        } else {
            writeln!(
                writer,
                "Baseline {}: new {} / resolved {} / unchanged {}",
                diff.git_ref,
                diff.new_issues.len(),
                diff.resolved_issues.len(),
                diff.unchanged
            )?;
        }
    }
    Ok(())
}

fn print_summary<W: Write>(
    report: &BoundaryReport,
    japanese: bool,
    writer: &mut W,
    include_notes: bool,
) -> io::Result<()> {
    if report.no_rust_files {
        let note = localized_no_rust_files(japanese);
        writeln!(writer, "{note}")?;
    }
    if japanese {
        writeln!(writer, "Boundary: {}", report.project)?;
        writeln!(
            writer,
            "評価: {} | スコア: {:.1}/100 | ファイル: {} | issue: {}",
            report.grade, report.score, report.summary.analyzed_files, report.summary.issue_count
        )?;
        writeln!(writer, "内訳: {}", format_breakdown(report, japanese))?;
    } else {
        writeln!(writer, "Boundary: {}", report.project)?;
        writeln!(
            writer,
            "Grade: {} | Score: {:.1}/100 | Files: {} | Issues: {}",
            report.grade, report.score, report.summary.analyzed_files, report.summary.issue_count
        )?;
        writeln!(writer, "Breakdown: {}", format_breakdown(report, japanese))?;
    }
    print_report_gate(report, writer)?;
    let hidden = hidden_low_count(report);
    if hidden > 0 {
        if japanese {
            writeln!(
                writer,
                "hint: 低 severity issue {hidden} 件を非表示にしています。--all を使うと表示します。"
            )?;
        } else {
            writeln!(
                writer,
                "hint: {hidden} low-severity issues hidden, use --all"
            )?;
        }
    }
    if include_notes {
        for note in localized_notes(report, japanese) {
            writeln!(writer, "note: {note}")?;
        }
    }
    Ok(())
}

fn print_issue<W: Write>(issue: &Issue, japanese: bool, writer: &mut W) -> io::Result<()> {
    let first = issue.locations.first();
    let location = first
        .map(|loc| format!("{}:{}:{}", loc.file.display(), loc.line, loc.column))
        .unwrap_or_else(|| "<unknown>".to_string());
    writeln!(
        writer,
        "{}[{}] {} -> {} at {}",
        severity_label(issue.severity, japanese),
        issue_type_label(issue, japanese),
        issue.key.source,
        issue.key.target,
        location
    )?;
    if japanese {
        writeln!(writer, "  {}", issue.message_ja)?;
        writeln!(writer, "  修正: {}", issue.suggestion_ja)?;
    } else {
        writeln!(writer, "  {}", issue.message)?;
        writeln!(writer, "  fix: {}", issue.suggestion)?;
    }
    Ok(())
}

fn print_ai<W: Write>(report: &BoundaryReport, japanese: bool, writer: &mut W) -> io::Result<()> {
    if japanese {
        writeln!(writer, "# cargo-boundary 修正計画")?;
    } else {
        writeln!(writer, "# cargo-boundary repair plan")?;
    }
    print_summary(report, japanese, writer, false)?;
    writeln!(writer)?;
    for (index, issue) in visible_issues(report).iter().enumerate() {
        writeln!(
            writer,
            "## {}. {} {}",
            index + 1,
            severity_label(issue.severity, japanese),
            issue_type_label(issue, japanese)
        )?;
        writeln!(
            writer,
            "- key: ({}, {}, {})",
            issue.key.issue_type, issue.key.source, issue.key.target
        )?;
        if japanese {
            writeln!(writer, "- 問題: {}", issue.message_ja)?;
            writeln!(writer, "- 対応: {}", issue.suggestion_ja)?;
        } else {
            writeln!(writer, "- problem: {}", issue.message)?;
            writeln!(writer, "- action: {}", issue.suggestion)?;
        }
        for location in issue.locations.iter().take(5) {
            writeln!(
                writer,
                "- location: {}:{}:{} `{}`",
                location.file.display(),
                location.line,
                location.column,
                location.snippet
            )?;
        }
    }
    writeln!(writer)?;
    if japanese {
        writeln!(writer, "## blind spot manifest")?;
    } else {
        writeln!(writer, "## Blind spot manifest")?;
    }
    for blind_spot in &report.blind_spots.blind_spots {
        let description = if japanese {
            &blind_spot.description_ja
        } else {
            &blind_spot.description
        };
        writeln!(writer, "- {}: {}", blind_spot.id, description)?;
    }
    for note in localized_notes(report, japanese) {
        writeln!(writer, "- note: {note}")?;
    }
    Ok(())
}

fn print_blind_spots<W: Write>(
    report: &BoundaryReport,
    japanese: bool,
    writer: &mut W,
) -> io::Result<()> {
    if japanese {
        writeln!(writer, "blind spot manifest:")?;
    } else {
        writeln!(writer, "Blind spot manifest:")?;
    }
    for blind_spot in &report.blind_spots.blind_spots {
        let description = if japanese {
            &blind_spot.description_ja
        } else {
            &blind_spot.description
        };
        writeln!(writer, "- {}: {}", blind_spot.id, description)?;
    }
    for note in localized_notes(report, japanese) {
        writeln!(writer, "- note: {note}")?;
    }
    Ok(())
}

fn print_layers<W: Write>(
    report: &BoundaryReport,
    japanese: bool,
    writer: &mut W,
) -> io::Result<()> {
    if japanese {
        writeln!(writer, "層構造:")?;
    } else {
        writeln!(writer, "Layers:")?;
    }
    for layer in &report.layers {
        writeln!(
            writer,
            "- {} (rank {}, {:?}) paths: {}",
            layer.name,
            layer.rank,
            layer.source,
            layer.paths.join(", ")
        )?;
        for evidence in &layer.evidence {
            writeln!(writer, "  - {evidence}")?;
        }
    }
    Ok(())
}

fn print_gate<W: Write>(gate: &GateReport, writer: &mut W) -> io::Result<()> {
    let status = if gate.passed { "PASS" } else { "FAIL" };
    writeln!(
        writer,
        "check: {status} (fail-on={}, {} issue(s) at/above threshold)",
        gate.fail_on, gate.failing
    )
}

fn print_report_gate<W: Write>(report: &BoundaryReport, writer: &mut W) -> io::Result<()> {
    if let Some(gate) = &report.gate {
        print_gate(gate, writer)?;
    }
    Ok(())
}

fn format_breakdown(report: &BoundaryReport, japanese: bool) -> String {
    [
        (Severity::Critical, report.summary.critical),
        (Severity::High, report.summary.high),
        (Severity::Medium, report.summary.medium),
        (Severity::Low, report.summary.low),
    ]
    .into_iter()
    .map(|(severity, count)| format!("{}={count}", localized_severity(severity, japanese)))
    .collect::<Vec<_>>()
    .join(", ")
}

fn visible_issues(report: &BoundaryReport) -> Vec<&Issue> {
    report
        .issues
        .iter()
        .filter(|issue| report.include_low || issue.severity >= Severity::Medium)
        .collect()
}

fn hidden_low_count(report: &BoundaryReport) -> usize {
    if report.include_low {
        0
    } else {
        report
            .issues
            .iter()
            .filter(|issue| issue.severity == Severity::Low)
            .count()
    }
}

fn localized_notes(report: &BoundaryReport, japanese: bool) -> &[String] {
    if japanese {
        &report.blind_spots.notes_ja
    } else {
        &report.blind_spots.notes
    }
}

fn localized_no_rust_files(japanese: bool) -> &'static str {
    if japanese {
        "この path 配下に Rust ファイルが見つかりませんでした"
    } else {
        "no Rust files found under this path"
    }
}

fn severity_label(severity: Severity, japanese: bool) -> &'static str {
    localized_severity(severity, japanese)
}

fn issue_type_label(issue: &Issue, japanese: bool) -> String {
    if japanese {
        match issue.key.issue_type {
            crate::model::IssueType::LayerViolation => "層違反".to_string(),
            crate::model::IssueType::InternalCrossing => "internal越境".to_string(),
            crate::model::IssueType::PubLeak => "pub漏れ".to_string(),
            crate::model::IssueType::ForbiddenImport => "禁止import".to_string(),
        }
    } else {
        issue.key.issue_type.to_string()
    }
}
