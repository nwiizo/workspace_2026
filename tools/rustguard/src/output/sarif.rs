use serde::Serialize;

use crate::analysis::AnalysisSummary;
use crate::diagnostics::{Finding, Severity};
use crate::error::Result;

/// SARIF v2.1.0 output for GitHub Code Scanning and other CI integrations.
/// Spec: https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLog {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: &'static str,
    version: &'static str,
    information_uri: &'static str,
    rules: Vec<SarifRule>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule {
    id: &'static str,
    short_description: SarifMessage,
    full_description: SarifMessage,
    default_configuration: SarifDefaultConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDefaultConfig {
    level: &'static str,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    rule_id: String,
    level: &'static str,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    related_locations: Vec<SarifRelatedLocation>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRegion {
    start_line: usize,
    start_column: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_column: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRelatedLocation {
    id: usize,
    physical_location: SarifPhysicalLocation,
}

fn severity_to_sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}

/// Convert an OS path to a file:// URI as required by SARIF §3.4.
fn path_to_uri(path: &str) -> String {
    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        // Relative path — use as-is (relative URI reference)
        path.replace('\\', "/")
    }
}

fn rule_definitions() -> Vec<SarifRule> {
    vec![
        SarifRule {
            id: "RG001",
            short_description: SarifMessage {
                text: "Unsafe function declaration".to_string(),
            },
            full_description: SarifMessage {
                text: "A function is declared as `unsafe`, requiring callers to use unsafe blocks."
                    .to_string(),
            },
            default_configuration: SarifDefaultConfig { level: "note" },
        },
        SarifRule {
            id: "RG002",
            short_description: SarifMessage {
                text: "Unsafe block usage".to_string(),
            },
            full_description: SarifMessage {
                text: "An unsafe block is used, potentially bypassing Rust's safety guarantees."
                    .to_string(),
            },
            default_configuration: SarifDefaultConfig { level: "warning" },
        },
        SarifRule {
            id: "RG003",
            short_description: SarifMessage {
                text: "Unsafe code reach".to_string(),
            },
            full_description: SarifMessage {
                text:
                    "Unsafe code is transitively reachable from safe functions via the call graph."
                        .to_string(),
            },
            default_configuration: SarifDefaultConfig { level: "warning" },
        },
    ]
}

pub fn render(findings: &[Finding], _summary: &AnalysisSummary) -> Result<String> {
    let results: Vec<SarifResult> = findings
        .iter()
        .map(|f| {
            let related: Vec<SarifRelatedLocation> = f
                .related_locations
                .iter()
                .enumerate()
                .map(|(i, loc)| SarifRelatedLocation {
                    id: i + 1,
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: path_to_uri(&loc.file.to_string_lossy()),
                        },
                        region: SarifRegion {
                            start_line: loc.line,
                            start_column: loc.column,
                            end_line: loc.end_line,
                            end_column: loc.end_column,
                        },
                    },
                })
                .collect();

            SarifResult {
                rule_id: f.rule_id.to_string(),
                level: severity_to_sarif_level(f.severity),
                message: SarifMessage {
                    text: f.message.clone(),
                },
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri: path_to_uri(&f.location.file.to_string_lossy()),
                        },
                        region: SarifRegion {
                            start_line: f.location.line,
                            start_column: f.location.column,
                            end_line: f.location.end_line,
                            end_column: f.location.end_column,
                        },
                    },
                }],
                related_locations: related,
            }
        })
        .collect();

    let log = SarifLog {
        schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        version: "2.1.0",
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "rustguard",
                    version: env!("CARGO_PKG_VERSION"),
                    information_uri: "https://github.com/nwiizo/rustguard",
                    rules: rule_definitions(),
                },
            },
            results,
        }],
    };

    let json = serde_json::to_string_pretty(&log)?;
    Ok(json)
}
