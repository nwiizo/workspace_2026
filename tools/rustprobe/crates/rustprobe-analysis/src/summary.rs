use std::fmt::Write;

use crate::probe_data::ProbeData;

const TOP_N: usize = 10;

pub fn generate_summary(data: &[ProbeData]) -> String {
    let mut out = String::new();
    for probe in data {
        write_crate_summary(&mut out, probe);
    }
    out
}

fn write_crate_summary(out: &mut String, data: &ProbeData) {
    writeln!(out, "RustProbe: {}", data.crate_name).expect("write");
    writeln!(out, "{}", "─".repeat(60.min(10 + data.crate_name.len()))).expect("write");
    writeln!(out).expect("write");

    let has_ownership_cost: Vec<_> = data
        .functions
        .iter()
        .filter(|f| f.clone_count > 0 || f.drop_count > 3 || (f.move_count > 5 && f.has_loops))
        .collect();

    if !has_ownership_cost.is_empty() {
        writeln!(out, "Ownership Cost Hotspots:").expect("write");
        writeln!(out).expect("write");
        for func in has_ownership_cost.iter().take(TOP_N) {
            write_hotspot(out, func);
        }
    }

    writeln!(out, "Functions ({} analyzed):", data.function_count()).expect("write");
    writeln!(out).expect("write");
    writeln!(
        out,
        "  {:<48} {:>6} {:>6} {:>6}",
        "Function", "clones", "moves", "drops"
    )
    .expect("write");
    writeln!(
        out,
        "  {:<48} {:>6} {:>6} {:>6}",
        "────────", "──────", "──────", "──────"
    )
    .expect("write");

    for func in data.functions.iter().take(TOP_N) {
        let location = format!("{}:{}", func.file, func.line_start);
        let label = format!(
            "{} ({})",
            truncate_name(&func.name, 30),
            truncate_name(&location, 15)
        );
        let loop_marker = if func.has_loops { " loop" } else { "" };

        writeln!(
            out,
            "  {:<48} {:>6} {:>6} {:>6}  {}",
            label, func.clone_count, func.move_count, func.drop_count, loop_marker
        )
        .expect("write");
    }

    let remaining = data.functions.len().saturating_sub(TOP_N);
    if remaining > 0 {
        writeln!(out, "  ... and {remaining} more").expect("write");
    }
    writeln!(out).expect("write");

    let total_clones: usize = data.functions.iter().map(|f| f.clone_count).sum();
    let total_moves: usize = data.functions.iter().map(|f| f.move_count).sum();
    let total_drops: usize = data.functions.iter().map(|f| f.drop_count).sum();

    writeln!(
        out,
        "Totals: {total_clones} clones, {total_moves} moves, {total_drops} drops across {} functions",
        data.function_count()
    )
    .expect("write");
    writeln!(out).expect("write");
}

fn write_hotspot(out: &mut String, func: &crate::probe_data::FunctionProbe) {
    let location = format!("{}:{}-{}", func.file, func.line_start, func.line_end);
    writeln!(out, "  {} ({})", func.name, location).expect("write");

    let mut reasons = Vec::new();

    if func.clone_count > 0 {
        let in_loop = if func.has_loops { " (inside loop)" } else { "" };
        reasons.push(format!("{} clone call(s){}", func.clone_count, in_loop));
    }

    if func.drop_count > 3 {
        reasons.push(format!(
            "{} drop sites — consider reducing scope or using references",
            func.drop_count
        ));
    }

    if func.move_count > 5 && func.has_loops {
        reasons.push(format!(
            "{} moves in a loop — values are being transferred repeatedly",
            func.move_count
        ));
    }

    for reason in &reasons {
        writeln!(out, "    -> {reason}").expect("write");
    }

    let ownership_targets: Vec<_> = func
        .call_targets
        .iter()
        .filter(|t| {
            t.contains("clone")
                || t.contains("drop")
                || t.contains("to_string")
                || t.contains("to_owned")
                || t.contains("Vec::<T>::new")
                || t.contains("Vec::<T, A>::push")
                || t.contains("String::from")
        })
        .collect();

    if !ownership_targets.is_empty() {
        let targets_str = ownership_targets
            .iter()
            .map(|t| short_target(t))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(out, "    calls: {targets_str}").expect("write");
    }

    writeln!(out).expect("write");
}

/// Shortens a fully qualified call target to its last two `::` segments.
fn short_target(target: &str) -> &str {
    if let Some(pos) = target.rfind("::")
        && let Some(pos2) = target[..pos].rfind("::")
    {
        return &target[pos2 + 2..];
    }
    target
}

fn truncate_name(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("..{}", &s[s.len().saturating_sub(max_len - 2)..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe_data::{FunctionProbe, ProbeData};
    use std::collections::HashMap;

    #[test]
    fn summary_contains_hotspots() {
        let data = ProbeData {
            crate_name: "my_app".to_string(),
            functions: vec![
                FunctionProbe {
                    name: "my_app::parser::parse".to_string(),
                    def_path: "my_app::parser::parse".to_string(),
                    file: "src/parser.rs".to_string(),
                    line_start: 10,
                    line_end: 50,
                    basic_block_count: 15,
                    statement_count: 45,
                    terminator_kinds: HashMap::from([("Call".to_string(), 5)]),
                    call_targets: vec![
                        "std::clone::Clone::clone".to_string(),
                        "std::vec::Vec::<T, A>::push".to_string(),
                    ],
                    has_loops: true,
                    move_count: 3,
                    clone_count: 8,
                    drop_count: 5,
                    complexity_score: 42.3,
                },
                FunctionProbe {
                    name: "my_app::main".to_string(),
                    def_path: "my_app::main".to_string(),
                    file: "src/main.rs".to_string(),
                    line_start: 1,
                    line_end: 10,
                    basic_block_count: 3,
                    statement_count: 9,
                    terminator_kinds: HashMap::new(),
                    call_targets: vec![],
                    has_loops: false,
                    move_count: 1,
                    clone_count: 0,
                    drop_count: 1,
                    complexity_score: 4.5,
                },
            ],
        };

        let summary = generate_summary(&[data]);
        assert!(summary.contains("Ownership Cost Hotspots"));
        assert!(summary.contains("parser::parse"));
        assert!(summary.contains("clone call(s)"));
        assert!(summary.contains("(inside loop)"));
        assert!(summary.contains("Totals:"));
    }

    #[test]
    fn summary_no_hotspots_when_clean() {
        let data = ProbeData {
            crate_name: "clean_app".to_string(),
            functions: vec![FunctionProbe {
                name: "clean_app::main".to_string(),
                def_path: "main".to_string(),
                file: "src/main.rs".to_string(),
                line_start: 1,
                line_end: 5,
                basic_block_count: 2,
                statement_count: 3,
                terminator_kinds: HashMap::new(),
                call_targets: vec![],
                has_loops: false,
                move_count: 0,
                clone_count: 0,
                drop_count: 0,
                complexity_score: 2.0,
            }],
        };

        let summary = generate_summary(&[data]);
        assert!(!summary.contains("Ownership Cost Hotspots"));
        assert!(summary.contains("Functions (1 analyzed)"));
    }

    #[test]
    fn truncate_preserves_tail() {
        assert_eq!(truncate_name("short", 10), "short");
        assert_eq!(truncate_name("very_long_name", 10), "..ong_name");
    }

    #[test]
    fn short_target_extracts_last_segments() {
        assert_eq!(short_target("std::clone::Clone::clone"), "Clone::clone");
    }
}
