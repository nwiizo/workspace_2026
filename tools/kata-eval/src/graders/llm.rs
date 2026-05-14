//! `llm` grader — LLM-as-Judge. Shells out to `claude -p` with a judge
//! prompt and parses `PASS / SCORE / REASON` from the reply.

use crate::executor::{ExecutorOptions, run_claude};
use crate::graders::GradingContext;
use anyhow::{Result, anyhow};
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::time::Duration;

pub(crate) async fn evaluate(
    config: &BTreeMap<String, Value>,
    ctx: &GradingContext<'_>,
) -> Result<(bool, f64, Option<String>)> {
    let rubric = config
        .get("rubric")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("llm grader: `rubric` is required"))?;
    let model = config
        .get("model")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| ctx.executor_opts.model.clone());
    let pass_threshold = config
        .get("pass_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.7);

    let prompt = build_judge_prompt(rubric, ctx.prompt, ctx.output);
    let judge_opts = ExecutorOptions {
        model,
        timeout: Duration::from_secs(120),
        require_self_report: false,
        system_prompt: Some(JUDGE_SYSTEM.to_string()),
    };
    let reply = run_claude(&prompt, &judge_opts).await?;
    let (pass, score, reason) = parse_verdict(&reply, pass_threshold);
    Ok((pass, score, reason))
}

const JUDGE_SYSTEM: &str = "You are an impartial grader. Respond ONLY with three lines: \
`PASS: yes|no`, `SCORE: <float 0..1>`, `REASON: <one sentence>`. \
No preamble, no markdown, no Self-report.";

fn build_judge_prompt(rubric: &str, prompt: &str, output: &str) -> String {
    format!(
        "RUBRIC:\n{rubric}\n\nORIGINAL PROMPT:\n{prompt}\n\nEXECUTOR OUTPUT:\n{output}\n\n\
         Decide PASS, SCORE, and REASON per the rubric. Format exactly:\n\
         PASS: yes\nSCORE: 0.9\nREASON: ...\n"
    )
}

pub(crate) fn parse_verdict(reply: &str, threshold: f64) -> (bool, f64, Option<String>) {
    let mut pass = None;
    let mut score = None;
    let mut reason = None;
    for line in reply.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("PASS:") {
            let v = v.trim().to_ascii_lowercase();
            pass = Some(matches!(v.as_str(), "yes" | "true" | "pass" | "1"));
        } else if let Some(v) = line.strip_prefix("SCORE:") {
            score = v.trim().parse::<f64>().ok();
        } else if let Some(v) = line.strip_prefix("REASON:") {
            reason = Some(v.trim().to_string());
        }
    }
    // If neither PASS nor SCORE was found the judge either errored or
    // ignored the protocol — surface that distinctly so it isn't mistaken
    // for a genuine "scored 0, failed".
    if pass.is_none() && score.is_none() {
        let preview: String = reply.chars().take(200).collect();
        let msg = format!(
            "judge did not return PASS/SCORE lines; first 200 chars: {}",
            preview.trim()
        );
        return (false, 0.0, Some(msg));
    }
    let score = score.unwrap_or(0.0);
    let pass = pass.unwrap_or(score >= threshold);
    (pass, score, reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed() {
        let (p, s, r) = parse_verdict("PASS: yes\nSCORE: 0.85\nREASON: looks good", 0.7);
        assert!(p);
        assert!((s - 0.85).abs() < 1e-9);
        assert_eq!(r.as_deref(), Some("looks good"));
    }

    #[test]
    fn falls_back_to_threshold() {
        let (p, s, _) = parse_verdict("SCORE: 0.6\nREASON: meh", 0.7);
        assert!(!p);
        assert!((s - 0.6).abs() < 1e-9);
    }

    #[test]
    fn surfaces_judge_protocol_errors() {
        let (p, s, msg) = parse_verdict(
            "I cannot grade this — rate limited.\n\nPlease try again.",
            0.7,
        );
        assert!(!p);
        assert_eq!(s, 0.0);
        let msg = msg.expect("error message");
        assert!(
            msg.contains("did not return PASS/SCORE"),
            "got message: {msg}"
        );
    }
}
