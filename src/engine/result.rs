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
pub fn save_as_baseline(report: &ValidationReport, path: &str) -> Result<(), crate::TpuDocError> {
    use crate::cli::output::{JsonFormatter, OutputFormatter};

    let formatter = JsonFormatter::new(true);
    let json = formatter.format(report);

    std::fs::write(path, json).map_err(|e| crate::TpuDocError::IoError {
        context: "save_as_baseline".to_string(),
        message: e.to_string(),
    })
}

/// Load a validation report from JSON baseline
pub fn load_baseline(path: &str) -> Result<ValidationReport, crate::TpuDocError> {
    let content = std::fs::read_to_string(path).map_err(|e| crate::TpuDocError::IoError {
        context: "load_baseline".to_string(),
        message: e.to_string(),
    })?;

    parse_json_report(&content).map_err(|e| crate::TpuDocError::ParseError {
        context: "load_baseline".to_string(),
        message: e,
    })
}

/// Parse a JSON string into a ValidationReport
fn parse_json_report(json: &str) -> Result<ValidationReport, String> {
    let mut report = ValidationReport::new();

    // Extract timestamp
    if let Some(ts) = extract_json_number(json, "timestamp") {
        report.timestamp = ts as u64;
    }

    // Extract hostname
    if let Some(hostname) = extract_json_string(json, "hostname") {
        report.hostname = hostname;
    }

    // Extract tpu_type
    report.tpu_type = extract_json_string(json, "tpu_type");

    // Extract total_duration_ms
    if let Some(duration) = extract_json_number(json, "total_duration_ms") {
        report.total_duration_ms = duration as u64;
    }

    // Extract checks array
    if let Some(checks_start) = json.find("\"checks\"") {
        if let Some(array_start) = json[checks_start..].find('[') {
            let array_begin = checks_start + array_start;
            if let Some(array_end) = find_matching_bracket(&json[array_begin..]) {
                let checks_json = &json[array_begin..array_begin + array_end + 1];
                report.checks = parse_checks_array(checks_json)?;
            }
        }
    }

    Ok(report)
}

/// Extract a string value from JSON by key
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\"", key);
    let key_pos = json.find(&search)?;
    let after_key = &json[key_pos + search.len()..];

    // Skip whitespace and colon
    let colon_pos = after_key.find(':')?;
    let after_colon = &after_key[colon_pos + 1..];

    // Find opening quote
    let quote_start = after_colon.find('"')?;
    let value_start = &after_colon[quote_start + 1..];

    // Find closing quote (handle escaped quotes)
    let mut end = 0;
    let mut escaped = false;
    for (i, c) in value_start.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            end = i;
            break;
        }
    }

    let value = &value_start[..end];
    Some(unescape_json_string(value))
}

/// Extract a number value from JSON by key
fn extract_json_number(json: &str, key: &str) -> Option<f64> {
    let search = format!("\"{}\"", key);
    let key_pos = json.find(&search)?;
    let after_key = &json[key_pos + search.len()..];

    // Skip whitespace and colon
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();

    // Extract number
    let end = after_colon
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(after_colon.len());

    after_colon[..end].parse().ok()
}

/// Find the matching closing bracket for an array or object
fn find_matching_bracket(s: &str) -> Option<usize> {
    let open = s.chars().next()?;
    let close = match open {
        '[' => ']',
        '{' => '}',
        _ => return None,
    };

    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;

    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' && in_string {
            escaped = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Parse the checks array from JSON
fn parse_checks_array(json: &str) -> Result<Vec<crate::Check>, String> {
    let mut checks = Vec::new();

    // Find each object in the array
    let mut pos = 1; // Skip opening bracket
    while pos < json.len() {
        // Find next object start
        if let Some(obj_start) = json[pos..].find('{') {
            let obj_begin = pos + obj_start;
            if let Some(obj_end) = find_matching_bracket(&json[obj_begin..]) {
                let check_json = &json[obj_begin..obj_begin + obj_end + 1];
                if let Ok(check) = parse_single_check(check_json) {
                    checks.push(check);
                }
                pos = obj_begin + obj_end + 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(checks)
}

/// Parse a single check object from JSON
fn parse_single_check(json: &str) -> Result<crate::Check, String> {
    let id = extract_json_string(json, "id").unwrap_or_default();
    let name = extract_json_string(json, "name").unwrap_or_default();
    let description = extract_json_string(json, "description").unwrap_or_default();

    // Parse category
    let category_str = extract_json_string(json, "category").unwrap_or_default();
    let category = match category_str.to_lowercase().as_str() {
        "hardware" => crate::CheckCategory::Hardware,
        "stack" => crate::CheckCategory::Stack,
        "performance" => crate::CheckCategory::Performance,
        "io" => crate::CheckCategory::Io,
        "security" => crate::CheckCategory::Security,
        "config" => crate::CheckCategory::Config,
        _ => crate::CheckCategory::Hardware,
    };

    // Parse result
    let result = parse_check_result(json);

    Ok(crate::Check {
        id,
        name,
        category,
        description,
        result,
    })
}

/// Parse the result field from a check JSON object
fn parse_check_result(json: &str) -> Option<crate::CheckResult> {
    // Find the result object
    let result_key = json.find("\"result\"")?;
    let after_key = &json[result_key..];

    // Check for null
    if after_key.contains("\"result\": null") || after_key.contains("\"result\":null") {
        return None;
    }

    // Find the result object
    let obj_start = after_key.find('{')?;
    let result_json = &after_key[obj_start..];
    let obj_end = find_matching_bracket(result_json)?;
    let result_obj = &result_json[..obj_end + 1];

    // Determine result type by looking for status field
    let status = extract_json_string(result_obj, "status")?;

    match status.to_lowercase().as_str() {
        "pass" => {
            let message = extract_json_string(result_obj, "message").unwrap_or_default();
            let duration_ms = extract_json_number(result_obj, "duration_ms").unwrap_or(0.0) as u64;
            Some(crate::CheckResult::Pass { message, duration_ms })
        }
        "warn" => {
            let message = extract_json_string(result_obj, "message").unwrap_or_default();
            let details = extract_json_string(result_obj, "details").unwrap_or_default();
            let duration_ms = extract_json_number(result_obj, "duration_ms").unwrap_or(0.0) as u64;
            Some(crate::CheckResult::Warn { message, details, duration_ms })
        }
        "fail" => {
            let message = extract_json_string(result_obj, "message").unwrap_or_default();
            let details = extract_json_string(result_obj, "details").unwrap_or_default();
            let duration_ms = extract_json_number(result_obj, "duration_ms").unwrap_or(0.0) as u64;
            Some(crate::CheckResult::Fail { message, details, duration_ms })
        }
        "skip" => {
            let reason = extract_json_string(result_obj, "reason").unwrap_or_default();
            Some(crate::CheckResult::Skip { reason })
        }
        _ => None,
    }
}

/// Unescape a JSON string value
fn unescape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                chars.next();
                match next {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    'u' => {
                        // Unicode escape
                        let hex: String = chars.by_ref().take(4).collect();
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                result.push(ch);
                            }
                        }
                    }
                    _ => {
                        result.push('\\');
                        result.push(next);
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}
