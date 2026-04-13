use crate::simulation::types::*;
use crate::topology::validator::ValidationReport;
use colored::Colorize;

pub fn render_validation(report: &ValidationReport) -> String {
    let mut out = String::new();
    if report.is_valid() {
        out.push_str(&format!("{}\n", "Topology is valid.".green().bold()));
    } else {
        out.push_str(&format!("{}\n", "Topology validation failed.".red().bold()));
    }
    for err in &report.errors {
        out.push_str(&format!("  {} {err}\n", "ERROR:".red()));
    }
    for warn in &report.warnings {
        out.push_str(&format!("  {} {warn}\n", "WARN:".yellow()));
    }
    out
}

pub fn render_cascade(result: &CascadeResult) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} Cascade Failure Simulation: {}\n",
        ">>>".cyan().bold(),
        result.origin_component.cyan().bold()
    ));
    out.push_str(&format!("{}\n", "=".repeat(60)));

    out.push_str(&format!(
        "  Severity: {}\n",
        match result.severity {
            Severity::Critical => result.severity.to_string().red().bold(),
            Severity::Major => result.severity.to_string().red(),
            Severity::Moderate => result.severity.to_string().yellow(),
            Severity::Minimal => result.severity.to_string().green(),
        }
    ));

    let br = &result.blast_radius;
    out.push_str(&format!(
        "  Blast radius: {}/{} components ({:.1}%)\n",
        br.total_affected.to_string().red(),
        br.total_components,
        br.impact_percentage
    ));
    out.push_str(&format!(
        "  Est. recovery: {:.0}s\n\n",
        result.estimated_recovery_seconds
    ));

    out.push_str(&format!("{}\n", "Cascade path:".bold()));
    for step in &result.cascade_path {
        let indent = "  ".repeat(step.depth + 1);
        let state_str = match step.state {
            ComponentState::Failed => "FAILED".red().bold(),
            ComponentState::Degraded => "DEGRADED".yellow(),
            ComponentState::Healthy => "HEALTHY".green(),
        };
        out.push_str(&format!(
            "{}{} {} [{}] (p={:.2})\n",
            indent,
            if step.depth == 0 { "*" } else { "└" },
            step.component_name,
            state_str,
            step.propagation_probability
        ));
    }
    out
}

pub fn render_spof(result: &SpofResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("{} SPOF Analysis\n", ">>>".cyan().bold()));
    out.push_str(&format!("{}\n", "=".repeat(60)));
    out.push_str(&format!(
        "  Resilience score: {:.0}/100\n\n",
        result.resilience_score
    ));

    if result.single_points_of_failure.is_empty() {
        out.push_str(&format!(
            "  {}\n",
            "No single points of failure detected.".green()
        ));
    } else {
        out.push_str(&format!(
            "{} ({} found)\n",
            "Single Points of Failure:".red().bold(),
            result.single_points_of_failure.len()
        ));
        out.push_str(&format!(
            "  {:<6} {:<25} {:<5} {:<4} {}\n",
            "Score", "Component", "AP", "Red.", "At Risk"
        ));
        out.push_str(&format!("  {}\n", "-".repeat(58)));
        for spof in &result.single_points_of_failure {
            let ap = if spof.is_articulation_point {
                "yes"
            } else {
                "no"
            };
            out.push_str(&format!(
                "  {:<6.0} {:<25} {:<5} {:<4} {} components\n",
                spof.criticality_score,
                spof.component_name,
                ap,
                spof.redundancy,
                spof.components_at_risk.len()
            ));
        }

        out.push('\n');
        for spof in &result.single_points_of_failure {
            out.push_str(&format!("  {} {}\n", ">>".yellow(), spof.recommendation));
        }
    }

    if !result.bridges.is_empty() {
        out.push_str(&format!(
            "\n{} ({} found)\n",
            "Critical Edges (Bridges):".yellow().bold(),
            result.bridges.len()
        ));
        for bridge in &result.bridges {
            out.push_str(&format!("  {} → {}\n", bridge.from, bridge.to));
        }
    }

    out
}
