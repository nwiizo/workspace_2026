use crate::analysis::Diagnostic;
use crate::config::OutputFormat;
use crate::cost::ProjectScore;
use crate::error::Result;

pub struct AnalysisReport {
    pub crate_name: String,
    pub diagnostics: Vec<Diagnostic>,
    pub score: ProjectScore,
}

impl AnalysisReport {
    pub fn render(&self, format: OutputFormat) -> Result<String> {
        match format {
            OutputFormat::Text => Ok(self.render_text()),
            OutputFormat::Json => self.render_json(),
        }
    }

    fn render_text(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!(
            "=== RustLean Analysis Report ===\n\
             Crate: {}\n\
             Project efficiency score: {:.0}/100\n\n",
            self.crate_name, self.score.normalized_score
        ));

        // Summary
        let s = &self.score.summary;
        out.push_str(&format!(
            "Functions analyzed: {}\n\
             Functions with diagnostics: {}\n\
             Total diagnostics: {}\n\n",
            s.total_functions_analyzed, s.functions_with_diagnostics, s.total_diagnostics,
        ));

        // Top functions by cost
        if !self.score.function_scores.is_empty() {
            out.push_str("Top functions by cost:\n");
            for (i, fs) in self.score.function_scores.iter().take(10).enumerate() {
                out.push_str(&format!(
                    "  {}. {} (score: {:.1}, {} diagnostics)\n",
                    i + 1,
                    fs.name,
                    fs.score,
                    fs.diagnostic_count
                ));
            }
            out.push('\n');
        }

        // Diagnostics by severity
        if !self.diagnostics.is_empty() {
            out.push_str("Diagnostics:\n");
            for diag in &self.diagnostics {
                let loop_marker = if diag.in_loop { " [loop]" } else { "" };
                out.push_str(&format!(
                    "  [{severity}] {file}:{line}  [{kind}]{loop_marker}\n    {message}\n",
                    severity = diag.severity,
                    file = diag.location.file,
                    line = diag.location.line,
                    kind = diag.kind,
                    message = diag.message,
                ));
                if let Some(suggestion) = &diag.suggestion {
                    out.push_str(&format!("    -> {suggestion}\n"));
                }
                out.push('\n');
            }
        }

        out
    }

    fn render_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

// Manual Serialize for AnalysisReport since it contains the full data
impl serde::Serialize for AnalysisReport {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AnalysisReport", 3)?;
        state.serialize_field("crate_name", &self.crate_name)?;
        state.serialize_field("diagnostics", &self.diagnostics)?;
        state.serialize_field("score", &self.score)?;
        state.end()
    }
}
