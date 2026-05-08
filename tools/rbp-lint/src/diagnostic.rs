use std::fmt;
use std::path::{Path, PathBuf};

use ra_ap_syntax::{TextRange, TextSize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => f.write_str("error"),
            Self::Warning => f.write_str("warning"),
            Self::Note => f.write_str("note"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: PathBuf,
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub snippet: String,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn from_range(
        file: &Path,
        rule: &'static str,
        severity: Severity,
        message: impl Into<String>,
        source: &str,
        range: TextRange,
        suggestion: Option<String>,
    ) -> Self {
        let (line, column) = offset_to_line_col(source, range.start());
        let (end_line, end_column) = offset_to_line_col(source, range.end());
        let snippet = source
            .get(usize::from(range.start())..usize::from(range.end()))
            .unwrap_or("")
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        Self {
            file: file.to_path_buf(),
            rule,
            severity,
            message: message.into(),
            line,
            column,
            end_line,
            end_column,
            snippet,
            suggestion,
        }
    }

    pub fn render_human(&self) -> String {
        let header = format!(
            "{sev}[{rule}] {file}:{line}:{col}: {msg}",
            sev = self.severity,
            rule = self.rule,
            file = self.file.display(),
            line = self.line,
            col = self.column,
            msg = self.message,
        );
        let snippet = if self.snippet.is_empty() {
            String::new()
        } else {
            format!("\n  | {}", self.snippet.trim_end())
        };
        let suggestion = self
            .suggestion
            .as_ref()
            .map(|s| format!("\n  = help: {s}"))
            .unwrap_or_default();
        format!("{header}{snippet}{suggestion}")
    }
}

fn offset_to_line_col(source: &str, offset: TextSize) -> (u32, u32) {
    let target = usize::from(offset).min(source.len());
    let mut line: u32 = 1;
    let mut col: u32 = 1;
    for (i, ch) in source.char_indices() {
        if i >= target {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_basic() {
        let src = "ab\ncd\nef";
        assert_eq!(offset_to_line_col(src, TextSize::new(0)), (1, 1));
        assert_eq!(offset_to_line_col(src, TextSize::new(3)), (2, 1));
        assert_eq!(offset_to_line_col(src, TextSize::new(7)), (3, 2));
    }
}
