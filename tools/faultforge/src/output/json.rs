use crate::simulation::types::{CascadeResult, SpofResult};
use serde_json;

pub fn render_cascade(result: &CascadeResult) -> serde_json::Result<String> {
    serde_json::to_string_pretty(result)
}

pub fn render_spof(result: &SpofResult) -> serde_json::Result<String> {
    serde_json::to_string_pretty(result)
}
