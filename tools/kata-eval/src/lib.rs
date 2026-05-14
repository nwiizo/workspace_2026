//! kata-eval — skill evaluation library used by the `kata` CLI.
//!
//! Schema-compatible with mizchi/waxa and microsoft/waza (`eval.yaml` +
//! `tasks/*.yaml`). Adds the empirical-prompt-tuning policy layer on top:
//! structured self-report grader, RED/GREEN/REFACTOR iterate loop with
//! `ledger.yaml`, and LLM-as-Judge for semantic equivalents that
//! surface-literal regex / code graders miss.

pub(crate) mod config;
pub(crate) mod executor;
pub(crate) mod graders;
pub mod iterate;
pub(crate) mod jsonl;
pub mod runner;
pub(crate) mod self_report;
pub(crate) mod skill;
pub mod types;
