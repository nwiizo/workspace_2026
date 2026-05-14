//! Grader implementations. Each grader reads YAML config and returns a
//! `GraderResult` (pass/score/message/duration).

mod code;
mod llm;
mod self_report;
mod text;

use crate::executor::ExecutorOptions;
use crate::types::{Grader, GraderResult, GraderType, SelfReport};
use anyhow::Result;
use std::time::Instant;

pub struct GradingContext<'a> {
    pub output: &'a str,
    pub self_report: Option<&'a SelfReport>,
    pub executor_opts: &'a ExecutorOptions,
    /// Prompt the executor was given (after Self-report tail). Used by the
    /// LLM judge so it can see what the task asked for.
    pub prompt: &'a str,
}

pub async fn run(grader: &Grader, ctx: &GradingContext<'_>) -> GraderResult {
    let start = Instant::now();
    let res: Result<(bool, f64, Option<String>)> = match grader.kind {
        GraderType::Text => text::evaluate(&grader.config, ctx.output),
        GraderType::Code => code::evaluate(&grader.config, ctx.output),
        GraderType::SelfReport => self_report::evaluate(&grader.config, ctx.self_report),
        GraderType::Llm => llm::evaluate(&grader.config, ctx).await,
    };
    let duration_ms = start.elapsed().as_millis();
    match res {
        Ok((pass, score, message)) => GraderResult {
            name: grader.name.clone(),
            pass,
            score,
            message,
            duration_ms,
        },
        Err(e) => GraderResult {
            name: grader.name.clone(),
            pass: false,
            score: 0.0,
            message: Some(format!("grader error: {e}")),
            duration_ms,
        },
    }
}

/// `output_contains` is checked alongside graders and surfaced as a
/// pseudo-grader named `_output_contains`.
pub fn output_contains_check(output: &str, needles: &[String]) -> GraderResult {
    let start = Instant::now();
    let missing: Vec<&String> = needles
        .iter()
        .filter(|n| !output.contains(n.as_str()))
        .collect();
    let pass = missing.is_empty();
    let message = if pass {
        None
    } else {
        Some(format!(
            "missing: {}",
            missing
                .iter()
                .map(|s| format!("{s:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    };
    GraderResult {
        name: "_output_contains".to_string(),
        pass,
        score: if pass { 1.0 } else { 0.0 },
        message,
        duration_ms: start.elapsed().as_millis(),
    }
}
