use std::collections::HashMap;

use serde::Serialize;

use crate::analysis::Diagnostic;
use crate::config::CostWeights;

#[derive(Debug, Clone, Serialize)]
pub struct FunctionScore {
    pub name: String,
    pub score: f64,
    pub diagnostic_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectScore {
    pub total_score: f64,
    pub normalized_score: f64,
    pub function_scores: Vec<FunctionScore>,
    pub summary: Summary,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub total_functions_analyzed: usize,
    pub functions_with_diagnostics: usize,
    pub total_diagnostics: usize,
    pub by_kind: HashMap<String, usize>,
}

pub fn compute_scores(diagnostics: &[Diagnostic], weights: &CostWeights) -> ProjectScore {
    let mut fn_diags: HashMap<String, Vec<&Diagnostic>> = HashMap::new();

    for diag in diagnostics {
        fn_diags
            .entry(diag.location.function.clone())
            .or_default()
            .push(diag);
    }

    let mut function_scores: Vec<FunctionScore> = fn_diags
        .iter()
        .map(|(name, diags)| {
            let score: f64 = diags
                .iter()
                .map(|d| {
                    let base = d.base_cost;
                    if d.in_loop {
                        base * weights.loop_multiplier
                    } else {
                        base
                    }
                })
                .sum();

            FunctionScore {
                name: name.clone(),
                score,
                diagnostic_count: diags.len(),
            }
        })
        .collect();

    function_scores.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total_score: f64 = function_scores.iter().map(|f| f.score).sum();

    // Normalize to 100-point scale (100 = perfect, 0 = worst)
    // Use a logarithmic scale: score = max(0, 100 - 10 * ln(1 + total_cost))
    let normalized_score = (100.0 - 10.0 * (1.0 + total_score).ln()).max(0.0);

    let mut by_kind: HashMap<String, usize> = HashMap::new();
    for diag in diagnostics {
        *by_kind.entry(diag.kind.to_string()).or_default() += 1;
    }

    let functions_with_diagnostics = function_scores
        .iter()
        .filter(|f| f.diagnostic_count > 0)
        .count();

    ProjectScore {
        total_score,
        normalized_score,
        function_scores,
        summary: Summary {
            total_functions_analyzed: fn_diags.len(),
            functions_with_diagnostics,
            total_diagnostics: diagnostics.len(),
            by_kind,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{DiagnosticKind, Location, Severity};

    fn make_diag(function: &str, kind: DiagnosticKind, cost: f64, in_loop: bool) -> Diagnostic {
        Diagnostic {
            kind,
            severity: Severity::Warning,
            message: "test".into(),
            suggestion: None,
            location: Location {
                file: "test.rs".into(),
                line: 1,
                column: 0,
                function: function.into(),
            },
            base_cost: cost,
            in_loop,
        }
    }

    #[test]
    fn empty_diagnostics_yield_perfect_score() {
        let weights = CostWeights::default();
        let score = compute_scores(&[], &weights);
        assert_eq!(score.total_score, 0.0);
        assert!(score.normalized_score > 99.0);
        assert_eq!(score.summary.total_diagnostics, 0);
    }

    #[test]
    fn loop_multiplier_applied() {
        let weights = CostWeights::default();
        let diags = vec![make_diag(
            "foo",
            DiagnosticKind::UnnecessaryClone,
            10.0,
            true,
        )];
        let score = compute_scores(&diags, &weights);
        assert_eq!(
            score.function_scores[0].score,
            10.0 * weights.loop_multiplier
        );
    }

    #[test]
    fn functions_sorted_by_score_desc() {
        let weights = CostWeights::default();
        let diags = vec![
            make_diag("low", DiagnosticKind::HeapAllocation, 5.0, false),
            make_diag("high", DiagnosticKind::UnnecessaryClone, 50.0, false),
        ];
        let score = compute_scores(&diags, &weights);
        assert_eq!(score.function_scores[0].name, "high");
        assert_eq!(score.function_scores[1].name, "low");
    }
}
