//! `self-report` grader — structural assertions on the executor's
//! Self-report block.

use crate::types::{PhaseStatus, SelfReport};
use anyhow::Result;
use serde_yaml::Value;
use std::collections::BTreeMap;

pub(crate) fn evaluate(
    config: &BTreeMap<String, Value>,
    report: Option<&SelfReport>,
) -> Result<(bool, f64, Option<String>)> {
    let require_present = config
        .get("require_present")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let require_all_ok = config
        .get("require_all_phases_ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let max_unclear = config
        .get("max_unclear")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    let max_retries = config
        .get("max_retries")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);

    let Some(report) = report else {
        let pass = !require_present;
        let msg = if require_present {
            Some("Self-report block missing".to_string())
        } else {
            None
        };
        return Ok((pass, if pass { 1.0 } else { 0.0 }, msg));
    };

    let mut failures: Vec<String> = Vec::new();
    if require_all_ok {
        let bad: Vec<&str> = report
            .phase_trace
            .iter()
            .filter(|p| p.status != PhaseStatus::Ok)
            .map(|p| p.phase.as_str())
            .collect();
        if !bad.is_empty() {
            failures.push(format!("phases not OK: {}", bad.join(", ")));
        }
    }
    if let Some(max) = max_unclear
        && report.unclear_points.len() > max
    {
        failures.push(format!(
            "unclear={} exceeds max={}",
            report.unclear_points.len(),
            max
        ));
    }
    if let Some(max) = max_retries
        && report.retries > max
    {
        failures.push(format!("retries={} exceeds max={}", report.retries, max));
    }
    let pass = failures.is_empty();
    let score = if pass { 1.0 } else { 0.0 };
    let message = if pass {
        None
    } else {
        Some(failures.join("; "))
    };
    Ok((pass, score, message))
}
