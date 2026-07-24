use crate::Severity;

#[derive(Debug, Clone, Copy)]
pub struct OutputOptions {
    pub all: bool,
    pub summary: bool,
    pub japanese: bool,
    pub blind_spots: bool,
}

pub fn localized<'a>(japanese: bool, ja: &'a str, en: &'a str) -> &'a str {
    if japanese { ja } else { en }
}

pub fn localized_severity(severity: Severity, japanese: bool) -> &'static str {
    match (severity, japanese) {
        (Severity::Critical, true) => "致命的",
        (Severity::High, true) => "高",
        (Severity::Medium, true) => "中",
        (Severity::Low, true) => "低",
        (Severity::Critical, false) => "Critical",
        (Severity::High, false) => "High",
        (Severity::Medium, false) => "Medium",
        (Severity::Low, false) => "Low",
    }
}

pub fn localized_severity_by_name(severity: &str, japanese: bool) -> &str {
    if !japanese {
        return severity;
    }
    match severity {
        "Low" => "低",
        "Medium" => "中",
        "High" => "高",
        "Critical" => "致命的",
        _ => severity,
    }
}
