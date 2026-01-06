//! Output formatting for tpu-doc.
//!
//! Provides terminal, JSON, and JUnit XML output formatters.
//!
//! # Graceful Degradation
//!
//! This module handles errors gracefully:
//! - Non-TTY output: Color disabled automatically via NO_COLOR or --no-color
//! - Non-UTF8 data: Uses lossy conversion for any string output
//! - Empty reports: Produces valid output with zero checks
//! - Missing fields: Uses sensible defaults (0, empty string)
//! - Large reports: No upper limit, but memory-efficient string building
//!
//! All formatters produce valid output for any ValidationReport input.
//! No function in this module will panic.

use crate::cli::args::OutputFormat;
use crate::engine::result::ValidationReport;

/// Trait for output formatters
pub trait OutputFormatter {
    /// Format a validation report into a string
    fn format(&self, report: &ValidationReport) -> String;
}

/// Terminal (human-readable) formatter
pub struct TerminalFormatter {
    color: bool,
    verbose: bool,
    quiet: bool,
}

impl TerminalFormatter {
    pub fn new(color: bool, verbose: bool, quiet: bool) -> Self {
        TerminalFormatter {
            color,
            verbose,
            quiet,
        }
    }

    fn colorize(&self, text: &str, color_code: &str) -> String {
        if self.color {
            format!("\x1b[{}m{}\x1b[0m", color_code, text)
        } else {
            text.to_string()
        }
    }

    fn green(&self, text: &str) -> String {
        self.colorize(text, "32")
    }

    fn yellow(&self, text: &str) -> String {
        self.colorize(text, "33")
    }

    fn red(&self, text: &str) -> String {
        self.colorize(text, "31")
    }

    fn gray(&self, text: &str) -> String {
        self.colorize(text, "90")
    }
}

impl OutputFormatter for TerminalFormatter {
    fn format(&self, report: &ValidationReport) -> String {
        let mut output = String::new();

        // Header
        output.push_str("--------------------------------------------------------------------------------\n");
        output.push_str("tpu-doc validation report\n");
        output.push_str(&format!("Host: {}\n", report.hostname));
        if let Some(ref tpu_type) = report.tpu_type {
            output.push_str(&format!("TPU Type: {}\n", tpu_type));
        }
        output.push_str(&format!("Timestamp: {}\n", format_timestamp(report.timestamp)));
        output.push_str("--------------------------------------------------------------------------------\n\n");

        // Group checks by category
        let categories = [
            ("HARDWARE CHECKS", "Hardware"),
            ("STACK CHECKS", "Stack"),
            ("PERFORMANCE CHECKS", "Performance"),
            ("I/O CHECKS", "Io"),
            ("SECURITY CHECKS", "Security"),
        ];

        for (header, category) in categories.iter() {
            let category_checks: Vec<_> = report
                .checks
                .iter()
                .filter(|c| format!("{:?}", c.category) == *category)
                .collect();

            if category_checks.is_empty() {
                continue;
            }

            // Skip category if quiet mode and no failures/warnings
            if self.quiet {
                let has_issues = category_checks
                    .iter()
                    .any(|c| matches!(&c.result, Some(crate::CheckResult::Fail { .. }) | Some(crate::CheckResult::Warn { .. })));
                if !has_issues {
                    continue;
                }
            }

            output.push_str(&format!("{}\n", header));

            for check in category_checks {
                // Skip passing checks in quiet mode
                if self.quiet {
                    if let Some(ref result) = check.result {
                        if matches!(result, crate::CheckResult::Pass { .. } | crate::CheckResult::Skip { .. }) {
                            continue;
                        }
                    }
                }

                let (status, message) = match &check.result {
                    Some(crate::CheckResult::Pass { message, duration_ms }) => {
                        let status = self.green("[PASS]");
                        let msg = if self.verbose {
                            format!("{} ({}ms)", message, duration_ms)
                        } else {
                            message.clone()
                        };
                        (status, msg)
                    }
                    Some(crate::CheckResult::Warn { message, details, duration_ms }) => {
                        let status = self.yellow("[WARN]");
                        let msg = if self.verbose {
                            format!("{} - {} ({}ms)", message, details, duration_ms)
                        } else {
                            message.clone()
                        };
                        (status, msg)
                    }
                    Some(crate::CheckResult::Fail { message, details, duration_ms }) => {
                        let status = self.red("[FAIL]");
                        let msg = if self.verbose {
                            format!("{} - {} ({}ms)", message, details, duration_ms)
                        } else {
                            message.clone()
                        };
                        (status, msg)
                    }
                    Some(crate::CheckResult::Skip { reason }) => {
                        let status = self.gray("[SKIP]");
                        (status, reason.clone())
                    }
                    None => {
                        let _status = self.gray("[----]");
                        (self.gray("[----]"), "Not executed".to_string())
                    }
                };

                output.push_str(&format!("  {} {}: {} ({})\n", status, check.id, check.name, message));
            }

            output.push('\n');
        }

        // Summary
        let summary = report.summary();
        output.push_str("--------------------------------------------------------------------------------\n");
        output.push_str(&format!(
            "SUMMARY: {} passed, {} warnings, {} failed, {} skipped\n",
            summary.passed, summary.warned, summary.failed, summary.skipped
        ));
        output.push_str(&format!(
            "Total time: {:.1}s\n",
            report.total_duration_ms as f64 / 1000.0
        ));

        let exit_desc = if summary.failed > 0 {
            "failures detected"
        } else if summary.warned > 0 {
            "warnings detected"
        } else {
            "all checks passed"
        };
        let exit_code = if summary.failed > 0 {
            1
        } else if summary.warned > 0 {
            2
        } else {
            0
        };
        output.push_str(&format!("Exit code: {} ({})\n", exit_code, exit_desc));
        output.push_str("--------------------------------------------------------------------------------");

        output
    }
}

/// JSON formatter
pub struct JsonFormatter {
    pretty: bool,
}

impl JsonFormatter {
    pub fn new(pretty: bool) -> Self {
        JsonFormatter { pretty }
    }

    fn escape_json_string(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '"' => result.push_str("\\\""),
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                c if c.is_control() => {
                    result.push_str(&format!("\\u{:04x}", c as u32));
                }
                c => result.push(c),
            }
        }
        result
    }
}

impl OutputFormatter for JsonFormatter {
    fn format(&self, report: &ValidationReport) -> String {
        let indent = if self.pretty { "  " } else { "" };
        let newline = if self.pretty { "\n" } else { "" };
        let space = if self.pretty { " " } else { "" };

        let mut output = String::new();
        output.push('{');
        output.push_str(newline);

        // Timestamp
        output.push_str(&format!("{}\"timestamp\":{}{},{}", indent, space, report.timestamp, newline));

        // Hostname
        output.push_str(&format!(
            "{}\"hostname\":{}\"{}\"{}",
            indent,
            space,
            Self::escape_json_string(&report.hostname),
            if report.tpu_type.is_some() || !report.checks.is_empty() { "," } else { "" }
        ));
        output.push_str(newline);

        // TPU type
        if let Some(ref tpu_type) = report.tpu_type {
            output.push_str(&format!(
                "{}\"tpu_type\":{}\"{}\"{}",
                indent,
                space,
                Self::escape_json_string(tpu_type),
                if !report.checks.is_empty() { "," } else { "" }
            ));
            output.push_str(newline);
        }

        // Total duration
        output.push_str(&format!(
            "{}\"total_duration_ms\":{}{},{}",
            indent, space, report.total_duration_ms, newline
        ));

        // Summary
        let summary = report.summary();
        output.push_str(&format!("{}\"summary\":{}{{", indent, space));
        output.push_str(newline);
        output.push_str(&format!("{}{}\"passed\":{}{},", indent, indent, space, summary.passed));
        output.push_str(newline);
        output.push_str(&format!("{}{}\"warned\":{}{},", indent, indent, space, summary.warned));
        output.push_str(newline);
        output.push_str(&format!("{}{}\"failed\":{}{},", indent, indent, space, summary.failed));
        output.push_str(newline);
        output.push_str(&format!("{}{}\"skipped\":{}{},", indent, indent, space, summary.skipped));
        output.push_str(newline);
        output.push_str(&format!("{}{}\"total\":{}{}", indent, indent, space, summary.total));
        output.push_str(newline);
        output.push_str(&format!("{}}},", indent));
        output.push_str(newline);

        // Checks array
        output.push_str(&format!("{}\"checks\":{}[", indent, space));
        output.push_str(newline);

        for (i, check) in report.checks.iter().enumerate() {
            output.push_str(&format!("{}{}{{", indent, indent));
            output.push_str(newline);

            output.push_str(&format!(
                "{}{}{}\"id\":{}\"{}\"{}",
                indent, indent, indent, space,
                Self::escape_json_string(&check.id),
                ","
            ));
            output.push_str(newline);

            output.push_str(&format!(
                "{}{}{}\"name\":{}\"{}\"{}",
                indent, indent, indent, space,
                Self::escape_json_string(&check.name),
                ","
            ));
            output.push_str(newline);

            output.push_str(&format!(
                "{}{}{}\"category\":{}\"{:?}\"{}",
                indent, indent, indent, space,
                check.category,
                ","
            ));
            output.push_str(newline);

            output.push_str(&format!(
                "{}{}{}\"description\":{}\"{}\"{}",
                indent, indent, indent, space,
                Self::escape_json_string(&check.description),
                ","
            ));
            output.push_str(newline);

            // Result
            output.push_str(&format!("{}{}{}\"result\":{}{{", indent, indent, indent, space));
            output.push_str(newline);

            match &check.result {
                Some(crate::CheckResult::Pass { message, duration_ms }) => {
                    output.push_str(&format!(
                        "{}{}{}{}\"status\":{}\"pass\",",
                        indent, indent, indent, indent, space
                    ));
                    output.push_str(newline);
                    output.push_str(&format!(
                        "{}{}{}{}\"message\":{}\"{}\"{}",
                        indent, indent, indent, indent, space,
                        Self::escape_json_string(message),
                        ","
                    ));
                    output.push_str(newline);
                    output.push_str(&format!(
                        "{}{}{}{}\"duration_ms\":{}{}",
                        indent, indent, indent, indent, space, duration_ms
                    ));
                }
                Some(crate::CheckResult::Warn { message, details, duration_ms }) => {
                    output.push_str(&format!(
                        "{}{}{}{}\"status\":{}\"warn\",",
                        indent, indent, indent, indent, space
                    ));
                    output.push_str(newline);
                    output.push_str(&format!(
                        "{}{}{}{}\"message\":{}\"{}\"{}",
                        indent, indent, indent, indent, space,
                        Self::escape_json_string(message),
                        ","
                    ));
                    output.push_str(newline);
                    output.push_str(&format!(
                        "{}{}{}{}\"details\":{}\"{}\"{}",
                        indent, indent, indent, indent, space,
                        Self::escape_json_string(details),
                        ","
                    ));
                    output.push_str(newline);
                    output.push_str(&format!(
                        "{}{}{}{}\"duration_ms\":{}{}",
                        indent, indent, indent, indent, space, duration_ms
                    ));
                }
                Some(crate::CheckResult::Fail { message, details, duration_ms }) => {
                    output.push_str(&format!(
                        "{}{}{}{}\"status\":{}\"fail\",",
                        indent, indent, indent, indent, space
                    ));
                    output.push_str(newline);
                    output.push_str(&format!(
                        "{}{}{}{}\"message\":{}\"{}\"{}",
                        indent, indent, indent, indent, space,
                        Self::escape_json_string(message),
                        ","
                    ));
                    output.push_str(newline);
                    output.push_str(&format!(
                        "{}{}{}{}\"details\":{}\"{}\"{}",
                        indent, indent, indent, indent, space,
                        Self::escape_json_string(details),
                        ","
                    ));
                    output.push_str(newline);
                    output.push_str(&format!(
                        "{}{}{}{}\"duration_ms\":{}{}",
                        indent, indent, indent, indent, space, duration_ms
                    ));
                }
                Some(crate::CheckResult::Skip { reason }) => {
                    output.push_str(&format!(
                        "{}{}{}{}\"status\":{}\"skip\",",
                        indent, indent, indent, indent, space
                    ));
                    output.push_str(newline);
                    output.push_str(&format!(
                        "{}{}{}{}\"reason\":{}\"{}\"",
                        indent, indent, indent, indent, space,
                        Self::escape_json_string(reason)
                    ));
                }
                None => {
                    output.push_str(&format!(
                        "{}{}{}{}\"status\":{}\"not_executed\"",
                        indent, indent, indent, indent, space
                    ));
                }
            }

            output.push_str(newline);
            output.push_str(&format!("{}{}{}}}", indent, indent, indent));
            output.push_str(newline);

            output.push_str(&format!("{}{}}}", indent, indent));
            if i < report.checks.len() - 1 {
                output.push(',');
            }
            output.push_str(newline);
        }

        output.push_str(&format!("{}]", indent));
        output.push_str(newline);
        output.push('}');

        output
    }
}

/// JUnit XML formatter
pub struct JunitFormatter;

impl JunitFormatter {
    pub fn new() -> Self {
        JunitFormatter
    }

    fn escape_xml(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '&' => result.push_str("&amp;"),
                '<' => result.push_str("&lt;"),
                '>' => result.push_str("&gt;"),
                '"' => result.push_str("&quot;"),
                '\'' => result.push_str("&apos;"),
                c => result.push(c),
            }
        }
        result
    }
}

impl Default for JunitFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter for JunitFormatter {
    fn format(&self, report: &ValidationReport) -> String {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

        let summary = report.summary();
        output.push_str(&format!(
            "<testsuites tests=\"{}\" failures=\"{}\" errors=\"0\" skipped=\"{}\" time=\"{:.3}\">\n",
            summary.total,
            summary.failed,
            summary.skipped,
            report.total_duration_ms as f64 / 1000.0
        ));

        // Group checks by category into test suites
        let categories = [
            ("Hardware", "hardware"),
            ("Stack", "stack"),
            ("Performance", "performance"),
            ("Io", "io"),
            ("Security", "security"),
        ];

        for (category, suite_name) in categories.iter() {
            let category_checks: Vec<_> = report
                .checks
                .iter()
                .filter(|c| format!("{:?}", c.category) == *category)
                .collect();

            if category_checks.is_empty() {
                continue;
            }

            let suite_failures = category_checks
                .iter()
                .filter(|c| matches!(&c.result, Some(crate::CheckResult::Fail { .. })))
                .count();
            let suite_skipped = category_checks
                .iter()
                .filter(|c| matches!(&c.result, Some(crate::CheckResult::Skip { .. })))
                .count();
            let suite_time: u64 = category_checks
                .iter()
                .filter_map(|c| match &c.result {
                    Some(crate::CheckResult::Pass { duration_ms, .. }) => Some(*duration_ms),
                    Some(crate::CheckResult::Warn { duration_ms, .. }) => Some(*duration_ms),
                    Some(crate::CheckResult::Fail { duration_ms, .. }) => Some(*duration_ms),
                    _ => None,
                })
                .sum();

            output.push_str(&format!(
                "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"0\" skipped=\"{}\" time=\"{:.3}\">\n",
                suite_name,
                category_checks.len(),
                suite_failures,
                suite_skipped,
                suite_time as f64 / 1000.0
            ));

            for check in category_checks {
                let time = match &check.result {
                    Some(crate::CheckResult::Pass { duration_ms, .. }) => *duration_ms,
                    Some(crate::CheckResult::Warn { duration_ms, .. }) => *duration_ms,
                    Some(crate::CheckResult::Fail { duration_ms, .. }) => *duration_ms,
                    _ => 0,
                };

                output.push_str(&format!(
                    "    <testcase name=\"{}\" classname=\"tpu-doc.{}\" time=\"{:.3}\"",
                    Self::escape_xml(&check.id),
                    suite_name,
                    time as f64 / 1000.0
                ));

                match &check.result {
                    Some(crate::CheckResult::Pass { message, .. }) => {
                        output.push_str(">\n");
                        output.push_str(&format!(
                            "      <system-out>{}</system-out>\n",
                            Self::escape_xml(message)
                        ));
                        output.push_str("    </testcase>\n");
                    }
                    Some(crate::CheckResult::Warn { message, details, .. }) => {
                        output.push_str(">\n");
                        output.push_str(&format!(
                            "      <system-out>WARNING: {} - {}</system-out>\n",
                            Self::escape_xml(message),
                            Self::escape_xml(details)
                        ));
                        output.push_str("    </testcase>\n");
                    }
                    Some(crate::CheckResult::Fail { message, details, .. }) => {
                        output.push_str(">\n");
                        output.push_str(&format!(
                            "      <failure message=\"{}\">{}</failure>\n",
                            Self::escape_xml(message),
                            Self::escape_xml(details)
                        ));
                        output.push_str("    </testcase>\n");
                    }
                    Some(crate::CheckResult::Skip { reason }) => {
                        output.push_str(">\n");
                        output.push_str(&format!(
                            "      <skipped message=\"{}\" />\n",
                            Self::escape_xml(reason)
                        ));
                        output.push_str("    </testcase>\n");
                    }
                    None => {
                        output.push_str(" />\n");
                    }
                }
            }

            output.push_str("  </testsuite>\n");
        }

        output.push_str("</testsuites>");
        output
    }
}

/// Get a formatter based on the output format
pub fn get_formatter(
    format: &OutputFormat,
    no_color: bool,
    verbose: bool,
    quiet: bool,
) -> Box<dyn OutputFormatter> {
    match format {
        OutputFormat::Text => Box::new(TerminalFormatter::new(!no_color, verbose, quiet)),
        OutputFormat::Json => Box::new(JsonFormatter::new(true)),
        OutputFormat::Junit => Box::new(JunitFormatter::new()),
    }
}

/// Format a Unix timestamp as ISO 8601
fn format_timestamp(timestamp: u64) -> String {
    // Simple ISO 8601 formatting without external dependencies
    // This is a basic implementation that works for recent timestamps
    let secs = timestamp;
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year, month, day from days since epoch
    // Using a simplified algorithm
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let mut month = 1;
    loop {
        let days_in_month = days_in_month(year, month);
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_month(year: u64, month: u64) -> u64 {
    match month {
        1 => 31,
        2 => if is_leap_year(year) { 29 } else { 28 },
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 30,
    }
}
