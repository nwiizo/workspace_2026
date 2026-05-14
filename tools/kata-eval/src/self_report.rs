//! Parser for the structured `## Self-report` block appended by the executor.

use crate::types::{PhaseEntry, PhaseStatus, SelfReport, UnclearPoint};
use regex::Regex;
use std::sync::OnceLock;

/// Locate `## Self-report` in the output and return its contents, or `None`
/// if the executor never appended one.
pub(crate) fn extract(output: &str) -> Option<&str> {
    let needle = "## Self-report";
    let idx = output.find(needle)?;
    Some(&output[idx..])
}

pub fn parse(output: &str) -> Option<SelfReport> {
    let block = extract(output)?;
    let mut report = SelfReport {
        raw: block.to_string(),
        ..Default::default()
    };

    let phase = section(block, "Phase trace");
    if !phase.is_empty() {
        report.phase_trace = parse_phase_trace(phase);
    }
    let unclear = section(block, "Unclear points");
    if !unclear.is_empty() {
        report.unclear_points = parse_unclear(unclear);
    }
    let fillins = section(block, "Discretionary fill-ins");
    if !fillins.is_empty() {
        report.discretionary_fill_ins = parse_bulleted_strings(fillins);
    }
    let retries = section(block, "Retries");
    if !retries.is_empty() {
        report.retries = parse_retries(retries);
    }
    Some(report)
}

/// Extract the body under `### <heading>` up to the next `### ` or end of
/// block.
fn section<'a>(block: &'a str, heading: &str) -> &'a str {
    let needle = format!("### {heading}");
    let Some(start) = block.find(&needle) else {
        return "";
    };
    let after = &block[start + needle.len()..];
    let after = after.strip_prefix('\n').unwrap_or(after);
    if let Some(next) = after.find("\n### ") {
        &after[..next]
    } else {
        after
    }
}

fn parse_phase_trace(text: &str) -> Vec<PhaseEntry> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"^\s*-\s*([^:]+?):\s*(OK|stuck|skipped|missing)\s*(?:—|-|–)?\s*(.*)$")
            .expect("regex")
    });
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if let Some(c) = re.captures(line) {
            let status = match &c[2] {
                "OK" => PhaseStatus::Ok,
                "stuck" => PhaseStatus::Stuck,
                "skipped" => PhaseStatus::Skipped,
                // Guarded by the regex's `OK|stuck|skipped|missing` alternation.
                _ => PhaseStatus::Missing,
            };
            let reason = c.get(3).map(|m| m.as_str().trim().to_string());
            out.push(PhaseEntry {
                phase: c[1].trim().to_string(),
                status,
                reason: reason.filter(|s| !s.is_empty()),
            });
        }
    }
    out
}

fn parse_unclear(text: &str) -> Vec<UnclearPoint> {
    // Bullets of the form:
    //   - Issue: ...
    //     Cause: ...
    //     General Fix Rule: ...
    let mut out = Vec::new();
    let mut cur: Option<UnclearPoint> = None;
    let flush = |cur: &mut Option<UnclearPoint>, out: &mut Vec<UnclearPoint>| {
        if let Some(p) = cur.take()
            && !p.issue.is_empty()
        {
            out.push(p);
        }
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("- Issue:")
            .or_else(|| trimmed.strip_prefix("-Issue:"))
        {
            flush(&mut cur, &mut out);
            cur = Some(UnclearPoint {
                issue: rest.trim().to_string(),
                cause: String::new(),
                rule: String::new(),
            });
        } else if let Some(rest) = trimmed.strip_prefix("Cause:") {
            if let Some(p) = cur.as_mut() {
                p.cause = rest.trim().to_string();
            }
        } else if let Some(rest) = trimmed
            .strip_prefix("General Fix Rule:")
            .or_else(|| trimmed.strip_prefix("Fix Rule:"))
            .or_else(|| trimmed.strip_prefix("Rule:"))
        {
            if let Some(p) = cur.as_mut() {
                p.rule = rest.trim().to_string();
            }
        }
    }
    flush(&mut cur, &mut out);
    out
}

fn parse_bulleted_strings(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let t = l.trim();
            t.strip_prefix("- ").map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_retries(text: &str) -> u32 {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\d+").expect("regex"));
    re.find(text)
        .and_then(|m| m.as_str().parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"ECHO: hello world

## Self-report

### Phase trace
- read prompt: OK
- emit echo: OK

### Unclear points
- Issue: what casing
  Cause: skill silent on uppercase
  General Fix Rule: state casing explicitly

### Discretionary fill-ins
- kept original casing

### Retries
0
"#;

    #[test]
    fn parses_a_full_report() {
        let r = parse(SAMPLE).unwrap();
        assert_eq!(r.phase_trace.len(), 2);
        assert!(r.phase_trace.iter().all(|p| p.status == PhaseStatus::Ok));
        assert_eq!(r.unclear_points.len(), 1);
        assert_eq!(r.unclear_points[0].issue, "what casing");
        assert_eq!(r.discretionary_fill_ins.len(), 1);
        assert_eq!(r.retries, 0);
    }

    #[test]
    fn returns_none_when_block_absent() {
        assert!(parse("just output, no self-report").is_none());
    }
}
