//! `text` grader — regex match / not-match against output.

use anyhow::Result;
use regex::Regex;
use serde_yaml::Value;
use std::collections::BTreeMap;

pub(crate) fn evaluate(
    config: &BTreeMap<String, Value>,
    output: &str,
) -> Result<(bool, f64, Option<String>)> {
    let regex_match = list_of_strings(config, "regex_match");
    let regex_not_match = list_of_strings(config, "regex_not_match");
    let contains = list_of_strings(config, "contains");
    let not_contains = list_of_strings(config, "not_contains");

    let mut failures: Vec<String> = Vec::new();
    for pat in &regex_match {
        let re = Regex::new(pat)?;
        if !re.is_match(output) {
            failures.push(format!("regex_match miss: /{pat}/"));
        }
    }
    for pat in &regex_not_match {
        let re = Regex::new(pat)?;
        if re.is_match(output) {
            failures.push(format!("regex_not_match hit: /{pat}/"));
        }
    }
    for s in &contains {
        if !output.contains(s.as_str()) {
            failures.push(format!("contains miss: {s:?}"));
        }
    }
    for s in &not_contains {
        if output.contains(s.as_str()) {
            failures.push(format!("not_contains hit: {s:?}"));
        }
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

fn list_of_strings(config: &BTreeMap<String, Value>, key: &str) -> Vec<String> {
    match config.get(key) {
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}
