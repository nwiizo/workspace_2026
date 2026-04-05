use crate::error::AppError;
use crate::model::{CompileResult, Diagnostic};
use std::io::Write;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::process::Command;
use tokio::time::timeout;

const COMPILE_TIMEOUT: Duration = Duration::from_secs(5);

/// Check if the given Rust source compiles successfully.
pub async fn check_compile(source: &str) -> Result<CompileResult, AppError> {
    let mut tmp = NamedTempFile::with_suffix(".rs")
        .map_err(|e| AppError::Internal(format!("Failed to create temp file: {e}")))?;

    tmp.write_all(source.as_bytes())
        .map_err(|e| AppError::Internal(format!("Failed to write temp file: {e}")))?;

    let path = tmp.path().to_path_buf();

    let result = timeout(
        COMPILE_TIMEOUT,
        Command::new("rustc")
            .arg("--edition")
            .arg("2024")
            .arg("--error-format=json")
            .arg("--emit=metadata")
            .arg("-o")
            .arg("/dev/null")
            .arg(&path)
            .output(),
    )
    .await
    .map_err(|_| AppError::Timeout)?
    .map_err(|e| AppError::CompileError(format!("Failed to run rustc: {e}")))?;

    let stderr = String::from_utf8_lossy(&result.stderr);
    let diagnostics = parse_diagnostics(&stderr);

    Ok(CompileResult {
        success: result.status.success(),
        diagnostics,
    })
}

fn parse_diagnostics(stderr: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for line in stderr.lines() {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(level) = value.get("level").and_then(|l| l.as_str())
            && (level == "error" || level == "warning")
        {
            let message = value
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error")
                .to_string();

            let (line_num, col) = extract_span(&value);

            diagnostics.push(Diagnostic {
                level: level.to_string(),
                message,
                line: line_num,
                column: col,
            });
        }
    }

    diagnostics
}

fn extract_span(value: &serde_json::Value) -> (Option<usize>, Option<usize>) {
    let spans = value.get("spans").and_then(|s| s.as_array());
    if let Some(spans) = spans
        && let Some(span) = spans.first()
    {
        let line = span
            .get("line_start")
            .and_then(|l| l.as_u64())
            .map(|l| l as usize);
        let col = span
            .get("column_start")
            .and_then(|c| c.as_u64())
            .map(|c| c as usize);
        return (line, col);
    }
    (None, None)
}
