//! `iterate` subcommand. Runs the eval repeatedly, aggregates executor
//! self-reports into `<eval-dir>/ledger.yaml`, and stops when either:
//! - 2 consecutive iterations report 0 new unclear-point rules (converged), or
//! - 3 consecutive iterations report non-decreasing new-unclear counts
//!   (diverged → the skill needs structural rewrite, not more patches).

use crate::runner::{self, EvalSummary, LoadedEval, RunOptions};
use crate::types::{TaskResult, UnclearPoint};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

/// Convergence threshold: this many *consecutive* zero-new-rule iterations
/// stops the loop.
const CONVERGE_STREAK: u32 = 2;
/// Divergence threshold: this many *consecutive* non-decreasing new-rule
/// counts (including the first iteration that establishes the baseline)
/// triggers `[DIVERGENCE-SIGNAL]`.
const DIVERGE_STREAK: u32 = 3;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Ledger {
    pub iterations: Vec<IterationEntry>,
    pub known_rules: BTreeSet<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IterationEntry {
    pub iter: u32,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub mean_pass_rate: f64,
    pub total_unclear: usize,
    pub new_rules_this_iter: Vec<String>,
    pub reseen_rules_this_iter: Vec<String>,
}

pub async fn run(
    loaded: &LoadedEval,
    max_iter: u32,
    only_task: Option<&str>,
    model_override: Option<&str>,
) -> Result<()> {
    let ledger_path = loaded.eval_dir.join("ledger.yaml");
    let mut ledger = load_or_init_ledger(&ledger_path);
    let mut tracker = StreakTracker::default();

    for n in 1..=max_iter {
        println!("\n========== iteration {n}/{max_iter} ==========");
        let opts = RunOptions {
            only_task,
            model_override,
            iter: Some(n),
            write_jsonl: true,
            ..Default::default()
        };
        let summary = runner::run_eval(loaded, &opts).await?;
        let (new_rules, reseen_rules) = classify_rules(&summary, &ledger.known_rules);
        for r in &new_rules {
            ledger.known_rules.insert(r.clone());
        }
        let entry = IterationEntry {
            iter: n,
            timestamp: chrono::Local::now(),
            mean_pass_rate: summary.mean_pass_rate,
            total_unclear: summary.total_unclear,
            new_rules_this_iter: new_rules,
            reseen_rules_this_iter: reseen_rules,
        };
        ledger.iterations.push(entry.clone());
        save_ledger(&ledger_path, &ledger)?;

        println!(
            "iter {n}: pass={:.0}%, unclear={}, new_rules={}, reseen={}",
            summary.mean_pass_rate * 100.0,
            summary.total_unclear,
            entry.new_rules_this_iter.len(),
            entry.reseen_rules_this_iter.len(),
        );

        match tracker.observe(entry.new_rules_this_iter.len()) {
            Signal::Converged => {
                println!(
                    "\n[CONVERGED] {CONVERGE_STREAK} consecutive iterations with no new unclear rules."
                );
                return Ok(());
            }
            Signal::Diverged => {
                println!(
                    "\n[DIVERGENCE-SIGNAL] {DIVERGE_STREAK} iterations of non-decreasing \
                     new-unclear count. Stop patching — rewrite the skill structure."
                );
                return Ok(());
            }
            Signal::Continue => {}
        }
    }
    println!("\n[MAX-ITER] reached --max={max_iter} without convergence.");
    Ok(())
}

/// Convergence/divergence streak detection. Kept in a small struct so the
/// transition logic is unit-testable independently of claude invocations.
#[derive(Debug, Default)]
struct StreakTracker {
    zero_streak: u32,
    nondecreasing_streak: u32,
    last_count: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
enum Signal {
    Continue,
    Converged,
    Diverged,
}

impl StreakTracker {
    fn observe(&mut self, new_rules: usize) -> Signal {
        // Convergence: count consecutive zero-new-rule iterations.
        if new_rules == 0 {
            self.zero_streak += 1;
        } else {
            self.zero_streak = 0;
        }

        // Divergence: count consecutive iterations whose new-rule count
        // did not shrink. Iteration 1 establishes the baseline and counts
        // toward the streak (the README documents this as "3 iterations
        // of non-decreasing new-unclear count").
        match self.last_count {
            None => {
                if new_rules > 0 {
                    self.nondecreasing_streak = 1;
                } else {
                    self.nondecreasing_streak = 0;
                }
            }
            Some(prev) => {
                if new_rules >= prev && new_rules > 0 {
                    self.nondecreasing_streak += 1;
                } else {
                    self.nondecreasing_streak = 0;
                }
            }
        }
        self.last_count = Some(new_rules);

        if self.zero_streak >= CONVERGE_STREAK {
            Signal::Converged
        } else if self.nondecreasing_streak >= DIVERGE_STREAK {
            Signal::Diverged
        } else {
            Signal::Continue
        }
    }
}

fn classify_rules(summary: &EvalSummary, known: &BTreeSet<String>) -> (Vec<String>, Vec<String>) {
    let rules = collect_rules(&summary.tasks);
    let mut new = Vec::new();
    let mut reseen = Vec::new();
    for r in rules {
        if known.contains(&r) {
            reseen.push(r);
        } else {
            new.push(r);
        }
    }
    new.sort();
    new.dedup();
    reseen.sort();
    reseen.dedup();
    (new, reseen)
}

fn collect_rules(tasks: &[TaskResult]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in tasks {
        for trial in &t.trials {
            if let Some(sr) = &trial.self_report {
                for u in &sr.unclear_points {
                    out.push(normalize_rule(u));
                }
            }
        }
    }
    out
}

fn normalize_rule(u: &UnclearPoint) -> String {
    let r = if u.rule.is_empty() { &u.issue } else { &u.rule };
    r.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn load_or_init_ledger(path: &Path) -> Ledger {
    if path.is_file()
        && let Ok(text) = std::fs::read_to_string(path)
        && let Ok(l) = serde_yaml::from_str(&text)
    {
        return l;
    }
    Ledger::default()
}

/// Atomically write the ledger by serializing to a sibling tempfile and
/// renaming over the target. Prevents a kill mid-write from leaving a
/// zero-byte ledger that resets all accumulated `known_rules`.
fn save_ledger(path: &Path, ledger: &Ledger) -> Result<()> {
    let yaml = serde_yaml::to_string(ledger).context("serializing ledger")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let tmp = path.with_extension("yaml.tmp");
    std::fs::write(&tmp, yaml).with_context(|| format!("writing tempfile {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_after_two_zero_iterations() {
        let mut t = StreakTracker::default();
        assert_eq!(t.observe(3), Signal::Continue); // iter 1: 3 new rules
        assert_eq!(t.observe(1), Signal::Continue); // iter 2: decreased
        assert_eq!(t.observe(0), Signal::Continue); // iter 3: first zero
        assert_eq!(t.observe(0), Signal::Converged); // iter 4: second zero
    }

    #[test]
    fn diverges_after_three_nondecreasing_iterations() {
        let mut t = StreakTracker::default();
        // iter 1: baseline 2 new rules — counts as streak=1
        assert_eq!(t.observe(2), Signal::Continue);
        // iter 2: 3 >= 2 — streak=2
        assert_eq!(t.observe(3), Signal::Continue);
        // iter 3: 3 >= 3 — streak=3 → diverge
        assert_eq!(t.observe(3), Signal::Diverged);
    }

    #[test]
    fn decreasing_count_resets_divergence_streak() {
        let mut t = StreakTracker::default();
        assert_eq!(t.observe(5), Signal::Continue); // streak=1
        assert_eq!(t.observe(6), Signal::Continue); // streak=2
        assert_eq!(t.observe(2), Signal::Continue); // decreased → streak=0
        assert_eq!(t.observe(3), Signal::Continue); // streak=1 again
    }

    #[test]
    fn zero_iter_clears_divergence_streak() {
        let mut t = StreakTracker::default();
        assert_eq!(t.observe(5), Signal::Continue); // diverge-streak=1
        assert_eq!(t.observe(0), Signal::Continue); // zero — clears both kinds
        assert_eq!(t.observe(0), Signal::Converged);
    }
}
