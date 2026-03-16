//! Certification Report Generation
//!
//! This module generates machine-readable compliance reports for AI agents
//! and certification audits. Output formats include JSON for programmatic
//! processing and text summaries for human review.
//!
//! # Use Cases
//!
//! - **AI/Agent Integration**: JSON output for automated code review
//! - **CI/CD Pipelines**: Exit codes and structured output for build systems
//! - **Certification Audits**: DO-178C and ISO 26262 compliance evidence
//!
//! # Example
//!
//! ```ignore
//! use fastc::p10::{P10Checker, ComplianceReport, ReportFormat};
//!
//! let checker = P10Checker::safety_critical();
//! let violations = checker.check(&ast, source);
//! let report = ComplianceReport::new("main.fc", &checker, &violations);
//!
//! // JSON for AI agents
//! println!("{}", report.to_json());
//!
//! // Text summary for humans
//! println!("{}", report.to_text());
//! ```

use super::{P10Config, P10Violation, SafetyLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compliance status for a single file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceStatus {
    /// All enabled rules pass
    Compliant,
    /// Some rules have violations
    NonCompliant,
    /// Checking was skipped (relaxed mode)
    Skipped,
}

/// Information about a single rule's compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    /// Rule number (1-10)
    pub rule_number: u8,
    /// Rule name
    pub name: String,
    /// Whether this rule is enabled at current safety level
    pub enabled: bool,
    /// Whether this rule passed
    pub passed: bool,
    /// Number of violations for this rule
    pub violation_count: usize,
    /// Detailed violations (if any)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<ViolationDetail>,
}

/// Detailed information about a single violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationDetail {
    /// Violation code (e.g., "P10-R1")
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Source location
    pub location: SourceLocation,
    /// Suggested fix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Source code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Byte offset in source
    pub offset: usize,
    /// Length of the span
    pub length: usize,
}

/// Compliance report for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    /// FastC version that generated this report
    pub fastc_version: String,
    /// Timestamp of report generation (ISO 8601)
    pub timestamp: String,
    /// Source file path
    pub file: String,
    /// Safety level used for checking
    pub safety_level: String,
    /// Overall compliance status
    pub status: ComplianceStatus,
    /// Summary statistics
    pub summary: ReportSummary,
    /// Results for each rule
    pub rules: Vec<RuleResult>,
    /// Metadata for certification
    pub certification: CertificationMetadata,
}

/// Summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    /// Total number of rules checked
    pub rules_checked: usize,
    /// Number of rules that passed
    pub rules_passed: usize,
    /// Number of rules that failed
    pub rules_failed: usize,
    /// Total number of violations
    pub total_violations: usize,
    /// Number of functions analyzed
    pub functions_analyzed: usize,
}

/// Metadata for certification bodies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationMetadata {
    /// Standard being targeted (e.g., "DO-178C", "ISO 26262")
    pub standard: String,
    /// Applicable rules from the standard
    pub applicable_rules: Vec<String>,
    /// Tool qualification level
    pub tool_qualification: String,
    /// Analysis method
    pub analysis_method: String,
}

impl ComplianceReport {
    /// Create a new compliance report
    pub fn new(
        filename: &str,
        config: &P10Config,
        violations: &[P10Violation],
        source: &str,
        function_count: usize,
    ) -> Self {
        let timestamp = chrono_lite_timestamp();

        // Group violations by rule number
        let mut by_rule: HashMap<u8, Vec<&P10Violation>> = HashMap::new();
        for v in violations {
            let rule_num = extract_rule_number(&v.code);
            by_rule.entry(rule_num).or_default().push(v);
        }

        // Build rule results
        let rules = build_rule_results(config, &by_rule, source);

        let rules_checked = rules.iter().filter(|r| r.enabled).count();
        let rules_passed = rules.iter().filter(|r| r.enabled && r.passed).count();
        let rules_failed = rules_checked - rules_passed;

        let status = if config.level == SafetyLevel::Relaxed {
            ComplianceStatus::Skipped
        } else if violations.is_empty() {
            ComplianceStatus::Compliant
        } else {
            ComplianceStatus::NonCompliant
        };

        ComplianceReport {
            fastc_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp,
            file: filename.to_string(),
            safety_level: format!("{:?}", config.level),
            status,
            summary: ReportSummary {
                rules_checked,
                rules_passed,
                rules_failed,
                total_violations: violations.len(),
                functions_analyzed: function_count,
            },
            rules,
            certification: CertificationMetadata {
                standard: certification_standard(config),
                applicable_rules: applicable_cert_rules(config),
                tool_qualification: "TQL-5 (Advisory)".to_string(),
                analysis_method: "Static Analysis".to_string(),
            },
        }
    }

    /// Serialize to JSON (for AI agents)
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Serialize to compact JSON (for CI/CD)
    pub fn to_json_compact(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate text summary (for humans)
    pub fn to_text(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!("╔══════════════════════════════════════════════════════════════╗\n"));
        out.push_str(&format!("║              FASTC COMPLIANCE REPORT                         ║\n"));
        out.push_str(&format!("╚══════════════════════════════════════════════════════════════╝\n\n"));

        out.push_str(&format!("File:         {}\n", self.file));
        out.push_str(&format!("Safety Level: {}\n", self.safety_level));
        out.push_str(&format!("Status:       {:?}\n", self.status));
        out.push_str(&format!("Generated:    {}\n", self.timestamp));
        out.push_str(&format!("FastC:        v{}\n\n", self.fastc_version));

        out.push_str("─────────────────────────────────────────────────────────────────\n");
        out.push_str("SUMMARY\n");
        out.push_str("─────────────────────────────────────────────────────────────────\n");
        out.push_str(&format!(
            "  Rules Checked:     {}\n",
            self.summary.rules_checked
        ));
        out.push_str(&format!(
            "  Rules Passed:      {}\n",
            self.summary.rules_passed
        ));
        out.push_str(&format!(
            "  Rules Failed:      {}\n",
            self.summary.rules_failed
        ));
        out.push_str(&format!(
            "  Total Violations:  {}\n",
            self.summary.total_violations
        ));
        out.push_str(&format!(
            "  Functions Analyzed:{}\n\n",
            self.summary.functions_analyzed
        ));

        out.push_str("─────────────────────────────────────────────────────────────────\n");
        out.push_str("RULE STATUS\n");
        out.push_str("─────────────────────────────────────────────────────────────────\n");

        for rule in &self.rules {
            let status_icon = if !rule.enabled {
                "○" // Not enabled
            } else if rule.passed {
                "✓" // Passed
            } else {
                "✗" // Failed
            };

            let status_text = if !rule.enabled {
                "SKIP"
            } else if rule.passed {
                "PASS"
            } else {
                "FAIL"
            };

            out.push_str(&format!(
                "  {} Rule {:2}: {:<30} [{}]\n",
                status_icon, rule.rule_number, rule.name, status_text
            ));

            if !rule.violations.is_empty() {
                for v in &rule.violations {
                    out.push_str(&format!(
                        "      └─ Line {}: {}\n",
                        v.location.line, v.message
                    ));
                }
            }
        }
        out.push('\n');

        out.push_str("─────────────────────────────────────────────────────────────────\n");
        out.push_str("CERTIFICATION INFO\n");
        out.push_str("─────────────────────────────────────────────────────────────────\n");
        out.push_str(&format!(
            "  Target Standard:   {}\n",
            self.certification.standard
        ));
        out.push_str(&format!(
            "  Tool Qualification:{}\n",
            self.certification.tool_qualification
        ));
        out.push_str(&format!(
            "  Analysis Method:   {}\n",
            self.certification.analysis_method
        ));

        if !self.certification.applicable_rules.is_empty() {
            out.push_str("  Applicable Rules:\n");
            for rule in &self.certification.applicable_rules {
                out.push_str(&format!("    - {}\n", rule));
            }
        }

        out
    }

    /// Check if this report indicates compliance
    pub fn is_compliant(&self) -> bool {
        self.status == ComplianceStatus::Compliant || self.status == ComplianceStatus::Skipped
    }
}

/// Multi-file compliance report for project-wide analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectReport {
    /// FastC version
    pub fastc_version: String,
    /// Timestamp
    pub timestamp: String,
    /// Project name (if available)
    pub project_name: Option<String>,
    /// Safety level
    pub safety_level: String,
    /// Overall project status
    pub status: ComplianceStatus,
    /// Project-wide summary
    pub summary: ProjectSummary,
    /// Individual file reports
    pub files: Vec<ComplianceReport>,
}

/// Project-wide summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    /// Total files analyzed
    pub files_analyzed: usize,
    /// Files that are compliant
    pub files_compliant: usize,
    /// Files with violations
    pub files_non_compliant: usize,
    /// Total violations across all files
    pub total_violations: usize,
    /// Total functions analyzed
    pub total_functions: usize,
}

impl ProjectReport {
    /// Create a project report from multiple file reports
    pub fn from_files(
        project_name: Option<String>,
        safety_level: SafetyLevel,
        files: Vec<ComplianceReport>,
    ) -> Self {
        let files_compliant = files
            .iter()
            .filter(|f| f.status == ComplianceStatus::Compliant)
            .count();
        let files_non_compliant = files
            .iter()
            .filter(|f| f.status == ComplianceStatus::NonCompliant)
            .count();
        let total_violations: usize = files.iter().map(|f| f.summary.total_violations).sum();
        let total_functions: usize = files.iter().map(|f| f.summary.functions_analyzed).sum();

        let status = if files.is_empty() {
            ComplianceStatus::Skipped
        } else if files_non_compliant == 0 {
            ComplianceStatus::Compliant
        } else {
            ComplianceStatus::NonCompliant
        };

        ProjectReport {
            fastc_version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono_lite_timestamp(),
            project_name,
            safety_level: format!("{:?}", safety_level),
            status,
            summary: ProjectSummary {
                files_analyzed: files.len(),
                files_compliant,
                files_non_compliant,
                total_violations,
                total_functions,
            },
            files,
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Check if project is compliant
    pub fn is_compliant(&self) -> bool {
        self.status == ComplianceStatus::Compliant || self.status == ComplianceStatus::Skipped
    }
}

// Helper functions

fn chrono_lite_timestamp() -> String {
    // Simple timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Format as ISO 8601 approximation
    let secs = now.as_secs();
    let days = secs / 86400;
    let years = 1970 + days / 365;
    let remaining_days = days % 365;
    let months = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        years, months, day, hours, minutes, seconds
    )
}

fn extract_rule_number(code: &str) -> u8 {
    // Parse rule number from codes like "P10-001", "P10-002", etc.
    if let Some(rest) = code.strip_prefix("P10-") {
        rest.chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0)
    } else {
        0
    }
}

fn build_rule_results(
    config: &P10Config,
    by_rule: &HashMap<u8, Vec<&P10Violation>>,
    source: &str,
) -> Vec<RuleResult> {
    // All 10 rules
    let rule_info: [(u8, &str, bool); 10] = [
        (1, "Simple Control Flow (no recursion)", !config.allow_recursion),
        (2, "Bounded Loops", config.require_loop_bounds),
        (3, "No Dynamic Allocation", !config.allow_runtime_alloc),
        (4, "Function Size Limit", config.level != SafetyLevel::Relaxed),
        (5, "Assertion Density", config.min_assertions_per_fn > 0),
        (6, "Minimal Scope", true), // By language design
        (7, "Check Return Values", true), // By type system
        (8, "Limited Preprocessor", true), // No preprocessor in FastC
        (9, "Restricted Pointers", config.max_pointer_depth <= 1),
        (10, "Zero Warnings", config.strict_mode),
    ];

    rule_info
        .iter()
        .map(|(num, name, enabled)| {
            let violations = by_rule.get(num).map(|v| v.as_slice()).unwrap_or(&[]);
            let violation_details: Vec<ViolationDetail> = violations
                .iter()
                .map(|v| ViolationDetail {
                    code: v.code.clone(),
                    message: v.message.clone(),
                    location: span_to_location(&v.span, source),
                    help: v.help.clone(),
                    note: v.note.clone(),
                })
                .collect();

            RuleResult {
                rule_number: *num,
                name: name.to_string(),
                enabled: *enabled,
                passed: violations.is_empty() || !*enabled,
                violation_count: violations.len(),
                violations: violation_details,
            }
        })
        .collect()
}

fn span_to_location(span: &std::ops::Range<usize>, source: &str) -> SourceLocation {
    let offset = span.start;
    let length = span.end.saturating_sub(span.start);

    // Calculate line and column
    let prefix = &source[..offset.min(source.len())];
    let line = prefix.chars().filter(|&c| c == '\n').count() + 1;
    let column = prefix.rfind('\n').map(|p| offset - p).unwrap_or(offset + 1);

    SourceLocation {
        line,
        column,
        offset,
        length,
    }
}

fn certification_standard(config: &P10Config) -> String {
    match config.level {
        SafetyLevel::SafetyCritical => {
            "NASA/JPL Power of 10 (DO-178C/ISO 26262 compatible)".to_string()
        }
        SafetyLevel::Standard => "NASA/JPL Power of 10 (Partial)".to_string(),
        SafetyLevel::Relaxed => "None (Prototyping Mode)".to_string(),
    }
}

fn applicable_cert_rules(config: &P10Config) -> Vec<String> {
    match config.level {
        SafetyLevel::SafetyCritical => vec![
            "DO-178C Section 6.3.4 - Source Code".to_string(),
            "DO-178C Table A-5 - Code Standards".to_string(),
            "ISO 26262-6:2018 Table 1 - Design principles".to_string(),
            "MISRA C:2012 - Applicable guidelines".to_string(),
        ],
        SafetyLevel::Standard => vec![
            "DO-178C Table A-5 - Code Standards (partial)".to_string(),
            "ISO 26262-6:2018 Table 1 - Design principles (partial)".to_string(),
        ],
        SafetyLevel::Relaxed => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_report_json() {
        let config = P10Config::standard();
        let report = ComplianceReport::new("test.fc", &config, &[], "fn main() {}", 1);

        let json = report.to_json();
        assert!(json.contains("\"status\": \"compliant\""));
        assert!(json.contains("\"safety_level\": \"Standard\""));
    }

    #[test]
    fn test_rule_number_extraction() {
        assert_eq!(extract_rule_number("P10-001"), 1);
        assert_eq!(extract_rule_number("P10-002"), 2);
        assert_eq!(extract_rule_number("P10-010"), 10);
        assert_eq!(extract_rule_number("invalid"), 0);
    }

    #[test]
    fn test_compliance_status() {
        let config = P10Config::standard();
        let report = ComplianceReport::new("test.fc", &config, &[], "fn main() {}", 1);
        assert!(report.is_compliant());

        let relaxed = P10Config::relaxed();
        let skipped = ComplianceReport::new("test.fc", &relaxed, &[], "fn main() {}", 1);
        assert_eq!(skipped.status, ComplianceStatus::Skipped);
    }
}
