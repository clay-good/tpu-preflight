//! Result aggregation and reporting.
//!
//! Collects check results, generates summaries, and supports baseline comparison.

use crate::{Check, CheckCategory, CheckResult};

/// Result summary statistics
#[derive(Debug, Clone, Default)]
pub struct ResultSummary {
    pub passed: u32,
    pub warned: u32,
    pub failed: u32,
    pub skipped: u32,
    pub total: u32,
    pub total_duration_ms: u64,
}

/// Validation report containing all check results
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub timestamp: u64,
    pub hostname: String,
    pub tpu_type: Option<String>,
    pub checks: Vec<Check>,
    pub total_duration_ms: u64,
}

impl ValidationReport {
    /// Create a new empty report
    pub fn new() -> Self {
        ValidationReport {
            timestamp: crate::platform::linux::get_unix_timestamp(),
            hostname: String::new(),
            tpu_type: None,
            checks: Vec::new(),
            total_duration_ms: 0,
        }
    }

    /// Calculate summary statistics
    pub fn summary(&self) -> ResultSummary {
        let mut summary = ResultSummary::default();

        for check in &self.checks {
            summary.total += 1;

            match &check.result {
                Some(CheckResult::Pass { duration_ms, .. }) => {
                    summary.passed += 1;
                    summary.total_duration_ms += duration_ms;
                }
                Some(CheckResult::Warn { duration_ms, .. }) => {
                    summary.warned += 1;
                    summary.total_duration_ms += duration_ms;
                }
                Some(CheckResult::Fail { duration_ms, .. }) => {
                    summary.failed += 1;
                    summary.total_duration_ms += duration_ms;
                }
                Some(CheckResult::Skip { .. }) => {
                    summary.skipped += 1;
                }
                None => {
                    summary.skipped += 1;
                }
            }
        }

        summary
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Baseline comparison result
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub new_failures: Vec<String>,
    pub new_warnings: Vec<String>,
    pub resolved: Vec<String>,
    pub regressions: Vec<String>,
    pub unchanged: Vec<String>,
}

/// Result aggregator for collecting check results
pub struct ResultAggregator {
    checks: Vec<Check>,
    hostname: String,
    tpu_type: Option<String>,
    total_duration_ms: u64,
}

impl ResultAggregator {
    /// Create a new result aggregator
    pub fn new() -> Self {
        ResultAggregator {
            checks: Vec::new(),
            hostname: String::new(),
            tpu_type: None,
            total_duration_ms: 0,
        }
    }

    /// Set report metadata
    pub fn set_metadata(&mut self, hostname: String, tpu_type: Option<String>, total_duration_ms: u64) {
        self.hostname = hostname;
        self.tpu_type = tpu_type;
        self.total_duration_ms = total_duration_ms;
    }

    /// Add a completed check result
    pub fn add_result(&mut self, check: Check) {
        self.checks.push(check);
    }

    /// Check if there are any failures
    pub fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|c| matches!(&c.result, Some(CheckResult::Fail { .. })))
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> ResultSummary {
        let mut summary = ResultSummary::default();

        for check in &self.checks {
            summary.total += 1;

            match &check.result {
                Some(CheckResult::Pass { duration_ms, .. }) => {
                    summary.passed += 1;
                    summary.total_duration_ms += duration_ms;
                }
                Some(CheckResult::Warn { duration_ms, .. }) => {
                    summary.warned += 1;
                    summary.total_duration_ms += duration_ms;
                }
                Some(CheckResult::Fail { duration_ms, .. }) => {
                    summary.failed += 1;
                    summary.total_duration_ms += duration_ms;
                }
                Some(CheckResult::Skip { .. }) => {
                    summary.skipped += 1;
                }
                None => {
                    summary.skipped += 1;
                }
            }
        }

        summary
    }

    /// Get checks by category
    pub fn get_by_category(&self, category: CheckCategory) -> Vec<&Check> {
        self.checks
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    /// Get only failed checks
    pub fn get_failures(&self) -> Vec<&Check> {
        self.checks
            .iter()
            .filter(|c| matches!(&c.result, Some(CheckResult::Fail { .. })))
            .collect()
    }

    /// Get only warning checks
    pub fn get_warnings(&self) -> Vec<&Check> {
        self.checks
            .iter()
            .filter(|c| matches!(&c.result, Some(CheckResult::Warn { .. })))
            .collect()
    }

    /// Create final validation report
    pub fn to_report(&self) -> ValidationReport {
        ValidationReport {
            timestamp: crate::platform::linux::get_unix_timestamp(),
            hostname: self.hostname.clone(),
            tpu_type: self.tpu_type.clone(),
            checks: self.checks.clone(),
            total_duration_ms: self.total_duration_ms,
        }
    }

    /// Compare against a baseline report
    pub fn compare_to_baseline(&self, baseline: &ValidationReport) -> ComparisonResult {
        let mut result = ComparisonResult {
            new_failures: Vec::new(),
            new_warnings: Vec::new(),
            resolved: Vec::new(),
            regressions: Vec::new(),
            unchanged: Vec::new(),
        };

        // Create lookup for baseline results
        let baseline_results: std::collections::HashMap<&str, &CheckResult> = baseline
            .checks
            .iter()
            .filter_map(|c| c.result.as_ref().map(|r| (c.id.as_str(), r)))
            .collect();

        for check in &self.checks {
            let current_status = check.result.as_ref().map(|r| match r {
                CheckResult::Pass { .. } => "pass",
                CheckResult::Warn { .. } => "warn",
                CheckResult::Fail { .. } => "fail",
                CheckResult::Skip { .. } => "skip",
            });

            let baseline_status = baseline_results.get(check.id.as_str()).map(|r| match r {
                CheckResult::Pass { .. } => "pass",
                CheckResult::Warn { .. } => "warn",
                CheckResult::Fail { .. } => "fail",
                CheckResult::Skip { .. } => "skip",
            });

            match (baseline_status, current_status) {
                (Some("pass"), Some("fail")) => {
                    result.regressions.push(check.id.clone());
                }
                (Some("pass"), Some("warn")) => {
                    result.new_warnings.push(check.id.clone());
                }
                (Some("fail"), Some("pass")) => {
                    result.resolved.push(check.id.clone());
                }
                (Some("warn"), Some("pass")) => {
                    result.resolved.push(check.id.clone());
                }
                (None, Some("fail")) => {
                    result.new_failures.push(check.id.clone());
                }
                (None, Some("warn")) => {
                    result.new_warnings.push(check.id.clone());
                }
                _ => {
                    result.unchanged.push(check.id.clone());
                }
            }
        }

        result
    }
}

impl Default for ResultAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Save a validation report as JSON baseline
pub fn save_as_baseline(report: &ValidationReport, path: &str) -> Result<(), crate::PreflightError> {
    use crate::cli::output::{JsonFormatter, OutputFormatter};

    let formatter = JsonFormatter::new(true);
    let json = formatter.format(report);

    std::fs::write(path, json).map_err(|e| crate::PreflightError::IoError {
        context: "save_as_baseline".to_string(),
        message: e.to_string(),
    })
}

/// Load a validation report from JSON baseline
pub fn load_baseline(path: &str) -> Result<ValidationReport, crate::PreflightError> {
    let _content = std::fs::read_to_string(path).map_err(|e| crate::PreflightError::IoError {
        context: "load_baseline".to_string(),
        message: e.to_string(),
    })?;

    // In a full implementation, we would parse the JSON back into a ValidationReport
    // For now, return an error since we don't have a JSON parser
    Err(crate::PreflightError::ParseError {
        context: "load_baseline".to_string(),
        message: "JSON parsing not yet implemented".to_string(),
    })
}
