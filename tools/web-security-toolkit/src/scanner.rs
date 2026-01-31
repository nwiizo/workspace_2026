//! Web vulnerability scanner
//!
//! Automated scanning for common web vulnerabilities.

use crate::headers::{analyze_headers, Severity};
use crate::http_client::{SecurityClient, SecurityResponse};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Scan result for a target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub target: String,
    pub timestamp: String,
    pub duration_ms: u64,
    pub findings: Vec<Finding>,
    pub summary: ScanSummary,
}

/// Individual security finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub category: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence: Option<String>,
    pub recommendation: String,
}

/// Summary of scan results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

impl ScanSummary {
    pub fn from_findings(findings: &[Finding]) -> Self {
        let mut summary = Self {
            total_findings: findings.len(),
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        };

        for f in findings {
            match f.severity.as_str() {
                "Critical" => summary.critical += 1,
                "High" => summary.high += 1,
                "Medium" => summary.medium += 1,
                "Low" => summary.low += 1,
                _ => summary.info += 1,
            }
        }

        summary
    }
}

/// Web vulnerability scanner
pub struct Scanner {
    client: SecurityClient,
    config: ScanConfig,
}

/// Scanner configuration
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub check_headers: bool,
    pub check_cookies: bool,
    pub check_info_disclosure: bool,
    pub check_common_paths: bool,
    pub check_cors: bool,
    pub timeout_seconds: u64,
    pub user_agent: String,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            check_headers: true,
            check_cookies: true,
            check_info_disclosure: true,
            check_common_paths: true,
            check_cors: true,
            timeout_seconds: 30,
            user_agent: "WebSecurityToolkit/1.0".to_string(),
        }
    }
}

impl Scanner {
    /// Create a new scanner
    pub fn new(config: ScanConfig) -> Self {
        let client = SecurityClient::new()
            .with_header("User-Agent", &config.user_agent)
            .unwrap_or_else(|_| SecurityClient::new());

        Self { client, config }
    }

    /// Scan a target URL
    pub fn scan(&self, target: &str) -> ScanResult {
        let start = Instant::now();
        let mut findings = Vec::new();

        // Normalize target URL
        let target = if !target.starts_with("http") {
            format!("https://{}", target)
        } else {
            target.to_string()
        };

        // Initial request
        if let Ok(response) = self.client.get(&target) {
            // Security headers check
            if self.config.check_headers {
                findings.extend(self.check_headers(&response));
            }

            // Cookie security check
            if self.config.check_cookies {
                findings.extend(self.check_cookies(&response));
            }

            // Information disclosure check
            if self.config.check_info_disclosure {
                findings.extend(self.check_info_disclosure(&response));
            }

            // CORS check
            if self.config.check_cors {
                findings.extend(self.check_cors(&target));
            }
        }

        // Common sensitive paths check
        if self.config.check_common_paths {
            findings.extend(self.check_common_paths(&target));
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let summary = ScanSummary::from_findings(&findings);

        ScanResult {
            target,
            timestamp: chrono_lite_now(),
            duration_ms,
            findings,
            summary,
        }
    }

    fn check_headers(&self, response: &SecurityResponse) -> Vec<Finding> {
        let checks = analyze_headers(&response.headers);
        checks
            .into_iter()
            .filter(|c| c.severity != Severity::Info || !c.present)
            .map(|c| Finding {
                category: "Security Headers".to_string(),
                severity: severity_to_string(&c.severity),
                title: format!("{} header issue", c.name),
                description: c.description,
                evidence: c.value,
                recommendation: c.recommendation,
            })
            .collect()
    }

    fn check_cookies(&self, response: &SecurityResponse) -> Vec<Finding> {
        response
            .cookies
            .iter()
            .flat_map(|cookie| {
                cookie.security_issues().into_iter().map(|issue| Finding {
                    category: "Cookie Security".to_string(),
                    severity: if cookie.name.to_lowercase().contains("session")
                        || cookie.name.to_lowercase().contains("token")
                    {
                        "High".to_string()
                    } else {
                        "Medium".to_string()
                    },
                    title: format!("Insecure cookie: {}", cookie.name),
                    description: issue,
                    evidence: Some(format!("{}={}", cookie.name, cookie.value)),
                    recommendation: "Add HttpOnly, Secure, and SameSite flags".to_string(),
                })
            })
            .collect()
    }

    fn check_info_disclosure(&self, response: &SecurityResponse) -> Vec<Finding> {
        let mut findings = Vec::new();
        let body_lower = response.body.to_lowercase();

        // Check for stack traces
        let stack_trace_patterns = [
            "at line",
            "stacktrace:",
            "stack trace:",
            "traceback (most recent call last)",
            "error in",
            "exception in thread",
            "fatal error:",
            "unhandled exception",
        ];

        for pattern in stack_trace_patterns {
            if body_lower.contains(pattern) {
                findings.push(Finding {
                    category: "Information Disclosure".to_string(),
                    severity: "Medium".to_string(),
                    title: "Stack trace in response".to_string(),
                    description: "Application error details are exposed".to_string(),
                    evidence: Some(format!("Pattern found: {}", pattern)),
                    recommendation: "Implement proper error handling, disable debug mode"
                        .to_string(),
                });
                break;
            }
        }

        // Check for SQL errors
        let sql_error_patterns = [
            "sql syntax",
            "mysql_fetch",
            "ora-",
            "pg_query",
            "sqlite_",
            "sqlstate",
            "syntax error at or near",
        ];

        for pattern in sql_error_patterns {
            if body_lower.contains(pattern) {
                findings.push(Finding {
                    category: "Information Disclosure".to_string(),
                    severity: "High".to_string(),
                    title: "Database error in response".to_string(),
                    description:
                        "Database error messages are exposed, indicating potential SQL injection"
                            .to_string(),
                    evidence: Some(format!("Pattern found: {}", pattern)),
                    recommendation: "Implement proper error handling, use parameterized queries"
                        .to_string(),
                });
                break;
            }
        }

        // Check for debug info
        if body_lower.contains("debug=true")
            || body_lower.contains("debug: true")
            || body_lower.contains("\"debug\":true")
        {
            findings.push(Finding {
                category: "Information Disclosure".to_string(),
                severity: "Low".to_string(),
                title: "Debug mode indicator".to_string(),
                description: "Application may be running in debug mode".to_string(),
                evidence: None,
                recommendation: "Disable debug mode in production".to_string(),
            });
        }

        // Check for source code
        if response.body.contains("<?php")
            || response.body.contains("<%@")
            || response.body.contains("<%=")
        {
            findings.push(Finding {
                category: "Information Disclosure".to_string(),
                severity: "Critical".to_string(),
                title: "Source code disclosure".to_string(),
                description: "Server-side source code is visible in response".to_string(),
                evidence: None,
                recommendation: "Configure server to properly process server-side scripts"
                    .to_string(),
            });
        }

        findings
    }

    fn check_cors(&self, target: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Test with malicious origin
        let test_origin = "https://evil.com";
        let client = SecurityClient::new()
            .with_header("Origin", test_origin)
            .unwrap_or_else(|_| SecurityClient::new());

        if let Ok(response) = client.get(target) {
            if let Some(acao) = response.headers.get("access-control-allow-origin") {
                // Check if origin is reflected
                if acao == test_origin {
                    let with_creds = response
                        .headers
                        .get("access-control-allow-credentials")
                        .map(|v| v == "true")
                        .unwrap_or(false);

                    findings.push(Finding {
                        category: "CORS Misconfiguration".to_string(),
                        severity: if with_creds { "Critical" } else { "High" }.to_string(),
                        title: "CORS origin reflection".to_string(),
                        description: format!(
                            "Origin is reflected in CORS header{}",
                            if with_creds {
                                " with credentials allowed"
                            } else {
                                ""
                            }
                        ),
                        evidence: Some(format!("Access-Control-Allow-Origin: {}", acao)),
                        recommendation: "Implement strict origin whitelist".to_string(),
                    });
                } else if acao == "*" {
                    findings.push(Finding {
                        category: "CORS Misconfiguration".to_string(),
                        severity: "Medium".to_string(),
                        title: "CORS wildcard origin".to_string(),
                        description: "Any origin can access this resource".to_string(),
                        evidence: Some("Access-Control-Allow-Origin: *".to_string()),
                        recommendation: "Restrict to specific trusted origins".to_string(),
                    });
                }
            }
        }

        findings
    }

    fn check_common_paths(&self, target: &str) -> Vec<Finding> {
        let mut findings = Vec::new();

        let sensitive_paths = vec![
            ("/.git/HEAD", "Git repository exposed", "Critical"),
            ("/.env", "Environment file exposed", "Critical"),
            ("/robots.txt", "Robots.txt found", "Info"),
            ("/sitemap.xml", "Sitemap found", "Info"),
            ("/.well-known/security.txt", "Security.txt found", "Info"),
            ("/phpinfo.php", "PHP info page exposed", "High"),
            ("/server-status", "Server status page exposed", "Medium"),
            ("/debug", "Debug endpoint exposed", "High"),
            ("/api/swagger.json", "API documentation exposed", "Low"),
            ("/api/docs", "API documentation exposed", "Low"),
            ("/graphql", "GraphQL endpoint found", "Info"),
            ("/.htaccess", "Htaccess file accessible", "Medium"),
            ("/web.config", "Web.config file accessible", "Medium"),
            ("/crossdomain.xml", "Flash crossdomain policy", "Low"),
            ("/clientaccesspolicy.xml", "Silverlight policy", "Low"),
            ("/elmah.axd", "ELMAH error log exposed", "High"),
            ("/trace.axd", ".NET trace exposed", "High"),
            ("/actuator/health", "Spring actuator exposed", "Medium"),
            ("/metrics", "Metrics endpoint exposed", "Medium"),
            ("/healthz", "Health check endpoint", "Info"),
        ];

        let base = target.trim_end_matches('/');

        for (path, description, severity) in sensitive_paths {
            let url = format!("{}{}", base, path);

            if let Ok(response) = self.client.get(&url) {
                if response.is_success() && !response.body.is_empty() {
                    // Additional validation for certain paths
                    let is_valid = match path {
                        "/.git/HEAD" => response.body.starts_with("ref:"),
                        "/.env" => {
                            response.body.contains('=') && !response.body.contains("<!DOCTYPE")
                        }
                        "/phpinfo.php" => response.body.contains("phpinfo()"),
                        _ => true,
                    };

                    if is_valid && severity != "Info" {
                        findings.push(Finding {
                            category: "Sensitive Path Exposure".to_string(),
                            severity: severity.to_string(),
                            title: format!("Sensitive file found: {}", path),
                            description: description.to_string(),
                            evidence: Some(format!(
                                "Status: {}, Size: {} bytes",
                                response.status,
                                response.body.len()
                            )),
                            recommendation: "Remove or restrict access to sensitive files"
                                .to_string(),
                        });
                    }
                }
            }
        }

        findings
    }
}

fn severity_to_string(severity: &Severity) -> String {
    match severity {
        Severity::Critical => "Critical",
        Severity::High => "High",
        Severity::Medium => "Medium",
        Severity::Low => "Low",
        Severity::Info => "Info",
    }
    .to_string()
}

fn chrono_lite_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

/// Generate findings report in markdown
pub fn generate_report(result: &ScanResult) -> String {
    let mut report = String::new();

    report.push_str("# Security Scan Report\n\n");
    report.push_str(&format!("**Target:** {}\n", result.target));
    report.push_str(&format!("**Scan Duration:** {}ms\n\n", result.duration_ms));

    report.push_str("## Summary\n\n");
    report.push_str(&format!(
        "| Severity | Count |\n|----------|-------|\n| Critical | {} |\n| High | {} |\n| Medium | {} |\n| Low | {} |\n| Info | {} |\n| **Total** | **{}** |\n\n",
        result.summary.critical,
        result.summary.high,
        result.summary.medium,
        result.summary.low,
        result.summary.info,
        result.summary.total_findings
    ));

    // Group findings by severity
    let severity_order = ["Critical", "High", "Medium", "Low", "Info"];

    for severity in severity_order {
        let severity_findings: Vec<_> = result
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect();

        if !severity_findings.is_empty() {
            report.push_str(&format!("## {} Findings\n\n", severity));

            for (i, finding) in severity_findings.iter().enumerate() {
                report.push_str(&format!("### {}. {}\n\n", i + 1, finding.title));
                report.push_str(&format!("**Category:** {}\n\n", finding.category));
                report.push_str(&format!("{}\n\n", finding.description));

                if let Some(evidence) = &finding.evidence {
                    report.push_str(&format!("**Evidence:**\n```\n{}\n```\n\n", evidence));
                }

                report.push_str(&format!(
                    "**Recommendation:** {}\n\n---\n\n",
                    finding.recommendation
                ));
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_summary() {
        let findings = vec![
            Finding {
                category: "test".to_string(),
                severity: "High".to_string(),
                title: "test".to_string(),
                description: "test".to_string(),
                evidence: None,
                recommendation: "test".to_string(),
            },
            Finding {
                category: "test".to_string(),
                severity: "Medium".to_string(),
                title: "test".to_string(),
                description: "test".to_string(),
                evidence: None,
                recommendation: "test".to_string(),
            },
        ];

        let summary = ScanSummary::from_findings(&findings);
        assert_eq!(summary.total_findings, 2);
        assert_eq!(summary.high, 1);
        assert_eq!(summary.medium, 1);
    }
}
