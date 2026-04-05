use crate::scan::candidate::{RiskLevel, ScanReport};
use colored::Colorize;

/// Print a human-readable scan report.
pub fn print_report(report: &ScanReport) {
    println!("{} Report", report.job_name.cyan().bold());
    println!("{}", "=".repeat(60));
    println!(
        "  関数スキャン数: {}  |  評価数: {}  |  適用可能: {}  |  所要時間: {}ms",
        report.functions_scanned,
        report.triples_evaluated,
        report.applicable_count,
        report.duration_ms,
    );
    println!();

    if report.candidates.is_empty() {
        println!("  候補なし");
        return;
    }

    let safe = report.safe_candidates();
    let moderate = report.moderate_candidates();
    let risky = report.risky_candidates();

    if !safe.is_empty() {
        println!(
            "{} score >= 80  ({} candidates)",
            "[Safe]".green().bold(),
            safe.len()
        );
        println!("{}", "-".repeat(60).dimmed());
        print_table_header();
        for c in &safe {
            print_candidate_row(c);
        }
        println!();
    }

    if !moderate.is_empty() {
        println!(
            "{} 50 <= score < 80  ({} candidates)",
            "[Warn]".yellow().bold(),
            moderate.len()
        );
        println!("{}", "-".repeat(60).dimmed());
        print_table_header();
        for c in &moderate {
            print_candidate_row(c);
        }
        println!();
    }

    if !risky.is_empty() {
        println!(
            "{} score < 50  ({} candidates)",
            "[Risk]".red().bold(),
            risky.len()
        );
        println!("{}", "-".repeat(60).dimmed());
        print_table_header();
        for c in &risky {
            print_candidate_row(c);
        }
        println!();
    }

    // Action hints.
    if let Some(best) = report.candidates.first() {
        println!("{}", "Next steps:".bold());
        println!(
            "  Preview:  cargo rustmorph preview -f \"{}\" -i {} -t {} --path <project>",
            best.function_name,
            best.param_index,
            best.transform.cli_name(),
        );
    }
}

fn print_table_header() {
    println!(
        "  {:<6} {:<40} {:<10} {:<20} {:>5} {:>5}",
        "Score", "Function", "Param", "Transform", "Sites", "Files"
    );
}

fn print_candidate_row(c: &crate::scan::candidate::ScanCandidate) {
    let icon = match c.risk_level() {
        RiskLevel::Safe => " +".green(),
        RiskLevel::Moderate => " ~".yellow(),
        RiskLevel::Risky => " !".red(),
    };
    println!(
        "  {}{:<4} {:<40} {:<10} {:<20} {:>5} {:>5}",
        icon,
        c.safety_score.total,
        truncate(&c.function_name, 40),
        truncate(&c.param_name, 10),
        c.transform.to_string(),
        c.affected_sites,
        c.affected_files,
    );
}

fn truncate(s: &str, max: usize) -> String {
    if max < 4 || s.len() <= max {
        return s.to_string();
    }
    let target = max - 3;
    // Find a safe char boundary for slicing.
    let boundary = s
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= target)
        .last()
        .unwrap_or(0);
    format!("{}...", &s[..boundary])
}
