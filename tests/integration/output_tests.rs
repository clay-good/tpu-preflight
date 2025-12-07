//! Output formatting tests.
//!
//! Tests for terminal, JSON, and JUnit XML output formatters.

use tpu_preflight::cli::output::{get_formatter, JsonFormatter, JunitFormatter, OutputFormatter, TerminalFormatter};
use tpu_preflight::cli::args::OutputFormat;
use tpu_preflight::engine::result::ValidationReport;
use tpu_preflight::{Check, CheckCategory, CheckResult};

fn create_sample_report() -> ValidationReport {
    ValidationReport {
        timestamp: 1733500000,
        hostname: "test-vm-001".to_string(),
        tpu_type: Some("v5e".to_string()),
        checks: vec![
            Check {
                id: "HW-001".to_string(),
                name: "TPU Device Detection".to_string(),
                category: CheckCategory::Hardware,
                description: "Verify TPU chips are present".to_string(),
                result: Some(CheckResult::Pass {
                    message: "8 chips detected".to_string(),
                    duration_ms: 100,
                }),
            },
            Check {
                id: "HW-003".to_string(),
                name: "TPU Thermal Status".to_string(),
                category: CheckCategory::Hardware,
                description: "Check thermal status".to_string(),
                result: Some(CheckResult::Warn {
                    message: "Temperature elevated".to_string(),
                    details: "Chip 3 at 78C".to_string(),
                    duration_ms: 50,
                }),
            },
            Check {
                id: "STK-002".to_string(),
                name: "libtpu Version".to_string(),
                category: CheckCategory::Stack,
                description: "Check libtpu version".to_string(),
                result: Some(CheckResult::Fail {
                    message: "Version mismatch".to_string(),
                    details: "0.1.dev < 0.2.dev required".to_string(),
                    duration_ms: 75,
                }),
            },
            Check {
                id: "IO-004".to_string(),
                name: "Checkpoint Directory".to_string(),
                category: CheckCategory::Io,
                description: "Check checkpoint access".to_string(),
                result: Some(CheckResult::Skip {
                    reason: "CHECKPOINT_DIR not set".to_string(),
                }),
            },
        ],
        total_duration_ms: 500,
    }
}

fn create_empty_report() -> ValidationReport {
    ValidationReport {
        timestamp: 1733500000,
        hostname: "empty-vm".to_string(),
        tpu_type: None,
        checks: vec![],
        total_duration_ms: 0,
    }
}

fn create_all_pass_report() -> ValidationReport {
    ValidationReport {
        timestamp: 1733500000,
        hostname: "test-vm".to_string(),
        tpu_type: Some("v5e".to_string()),
        checks: vec![
            Check {
                id: "HW-001".to_string(),
                name: "TPU Device Detection".to_string(),
                category: CheckCategory::Hardware,
                description: "Test".to_string(),
                result: Some(CheckResult::Pass {
                    message: "OK".to_string(),
                    duration_ms: 100,
                }),
            },
            Check {
                id: "HW-002".to_string(),
                name: "HBM Memory".to_string(),
                category: CheckCategory::Hardware,
                description: "Test".to_string(),
                result: Some(CheckResult::Pass {
                    message: "OK".to_string(),
                    duration_ms: 100,
                }),
            },
        ],
        total_duration_ms: 200,
    }
}

// Terminal formatter tests

#[test]
fn test_terminal_formatter_basic() {
    let formatter = TerminalFormatter::new(false, false, false);
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("tpu-preflight validation report"));
    assert!(output.contains("test-vm-001"));
    assert!(output.contains("v5e"));
    assert!(output.contains("SUMMARY"));
}

#[test]
fn test_terminal_formatter_contains_checks() {
    let formatter = TerminalFormatter::new(false, false, false);
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("HW-001"));
    assert!(output.contains("TPU Device Detection"));
    assert!(output.contains("[PASS]"));
    assert!(output.contains("[WARN]"));
    assert!(output.contains("[FAIL]"));
    assert!(output.contains("[SKIP]"));
}

#[test]
fn test_terminal_formatter_summary() {
    let formatter = TerminalFormatter::new(false, false, false);
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("1 passed"));
    assert!(output.contains("1 warning"));
    assert!(output.contains("1 failed"));
    assert!(output.contains("1 skipped"));
}

#[test]
fn test_terminal_formatter_verbose() {
    let formatter = TerminalFormatter::new(false, true, false);
    let report = create_sample_report();
    let output = formatter.format(&report);

    // Verbose should include duration
    assert!(output.contains("ms"));
}

#[test]
fn test_terminal_formatter_quiet() {
    let formatter = TerminalFormatter::new(false, false, true);
    let report = create_sample_report();
    let output = formatter.format(&report);

    // Quiet mode should not include passing checks
    // but should include failures and warnings
    assert!(output.contains("[FAIL]"));
    assert!(output.contains("[WARN]"));
}

#[test]
fn test_terminal_formatter_color() {
    let formatter = TerminalFormatter::new(true, false, false);
    let report = create_sample_report();
    let output = formatter.format(&report);

    // Color output should include ANSI escape codes
    assert!(output.contains("\x1b["));
}

#[test]
fn test_terminal_formatter_no_color() {
    let formatter = TerminalFormatter::new(false, false, false);
    let report = create_sample_report();
    let output = formatter.format(&report);

    // No color output should not include ANSI escape codes
    assert!(!output.contains("\x1b[32m")); // green
    assert!(!output.contains("\x1b[33m")); // yellow
    assert!(!output.contains("\x1b[31m")); // red
}

#[test]
fn test_terminal_formatter_empty_report() {
    let formatter = TerminalFormatter::new(false, false, false);
    let report = create_empty_report();
    let output = formatter.format(&report);

    assert!(output.contains("empty-vm"));
    assert!(output.contains("SUMMARY"));
    assert!(output.contains("0 passed"));
}

// JSON formatter tests

#[test]
fn test_json_formatter_basic() {
    let formatter = JsonFormatter::new(true);
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.starts_with('{'));
    assert!(output.ends_with('}'));
    assert!(output.contains("\"hostname\""));
    assert!(output.contains("\"test-vm-001\""));
}

#[test]
fn test_json_formatter_contains_fields() {
    let formatter = JsonFormatter::new(true);
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("\"timestamp\""));
    assert!(output.contains("\"tpu_type\""));
    assert!(output.contains("\"checks\""));
    assert!(output.contains("\"summary\""));
    assert!(output.contains("\"total_duration_ms\""));
}

#[test]
fn test_json_formatter_check_structure() {
    let formatter = JsonFormatter::new(true);
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("\"id\""));
    assert!(output.contains("\"name\""));
    assert!(output.contains("\"category\""));
    assert!(output.contains("\"description\""));
    assert!(output.contains("\"result\""));
    assert!(output.contains("\"status\""));
}

#[test]
fn test_json_formatter_result_types() {
    let formatter = JsonFormatter::new(true);
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("\"pass\""));
    assert!(output.contains("\"warn\""));
    assert!(output.contains("\"fail\""));
    assert!(output.contains("\"skip\""));
}

#[test]
fn test_json_formatter_summary_counts() {
    let formatter = JsonFormatter::new(true);
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("\"passed\""));
    assert!(output.contains("\"warned\""));
    assert!(output.contains("\"failed\""));
    assert!(output.contains("\"skipped\""));
    assert!(output.contains("\"total\""));
}

#[test]
fn test_json_formatter_escapes_special_chars() {
    let formatter = JsonFormatter::new(true);
    let mut report = create_sample_report();
    report.hostname = "test\"vm".to_string();
    let output = formatter.format(&report);

    assert!(output.contains("test\\\"vm"));
}

#[test]
fn test_json_formatter_empty_report() {
    let formatter = JsonFormatter::new(true);
    let report = create_empty_report();
    let output = formatter.format(&report);

    assert!(output.contains("\"checks\": ["));
    assert!(output.contains("\"total\": 0"));
}

// JUnit formatter tests

#[test]
fn test_junit_formatter_basic() {
    let formatter = JunitFormatter::new();
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.starts_with("<?xml version=\"1.0\""));
    assert!(output.contains("<testsuites"));
    assert!(output.contains("</testsuites>"));
}

#[test]
fn test_junit_formatter_testsuites_attributes() {
    let formatter = JunitFormatter::new();
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("tests=\""));
    assert!(output.contains("failures=\""));
    assert!(output.contains("errors=\"0\""));
    assert!(output.contains("skipped=\""));
    assert!(output.contains("time=\""));
}

#[test]
fn test_junit_formatter_testsuite_per_category() {
    let formatter = JunitFormatter::new();
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("<testsuite name=\"hardware\""));
    assert!(output.contains("<testsuite name=\"stack\""));
    assert!(output.contains("<testsuite name=\"io\""));
}

#[test]
fn test_junit_formatter_testcase_structure() {
    let formatter = JunitFormatter::new();
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("<testcase name=\"HW-001\""));
    assert!(output.contains("classname=\"tpu-preflight."));
    assert!(output.contains("time=\""));
}

#[test]
fn test_junit_formatter_failure_element() {
    let formatter = JunitFormatter::new();
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("<failure message=\""));
}

#[test]
fn test_junit_formatter_skipped_element() {
    let formatter = JunitFormatter::new();
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("<skipped message=\""));
}

#[test]
fn test_junit_formatter_system_out() {
    let formatter = JunitFormatter::new();
    let report = create_sample_report();
    let output = formatter.format(&report);

    assert!(output.contains("<system-out>"));
}

#[test]
fn test_junit_formatter_escapes_xml_special_chars() {
    let formatter = JunitFormatter::new();
    let mut report = create_sample_report();
    report.checks[0].result = Some(CheckResult::Pass {
        message: "Test <with> & special \"chars\"".to_string(),
        duration_ms: 100,
    });
    let output = formatter.format(&report);

    assert!(output.contains("&lt;"));
    assert!(output.contains("&gt;"));
    assert!(output.contains("&amp;"));
    assert!(output.contains("&quot;"));
}

#[test]
fn test_junit_formatter_empty_report() {
    let formatter = JunitFormatter::new();
    let report = create_empty_report();
    let output = formatter.format(&report);

    assert!(output.contains("tests=\"0\""));
    assert!(output.contains("failures=\"0\""));
}

// get_formatter factory tests

#[test]
fn test_get_formatter_text() {
    let formatter = get_formatter(&OutputFormat::Text, false, false, false);
    let report = create_sample_report();
    let output = formatter.format(&report);
    assert!(output.contains("tpu-preflight validation report"));
}

#[test]
fn test_get_formatter_json() {
    let formatter = get_formatter(&OutputFormat::Json, false, false, false);
    let report = create_sample_report();
    let output = formatter.format(&report);
    assert!(output.starts_with('{'));
}

#[test]
fn test_get_formatter_junit() {
    let formatter = get_formatter(&OutputFormat::Junit, false, false, false);
    let report = create_sample_report();
    let output = formatter.format(&report);
    assert!(output.contains("<testsuites"));
}

// Summary calculation tests

#[test]
fn test_report_summary_calculation() {
    let report = create_sample_report();
    let summary = report.summary();

    assert_eq!(summary.passed, 1);
    assert_eq!(summary.warned, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.total, 4);
}

#[test]
fn test_report_summary_all_pass() {
    let report = create_all_pass_report();
    let summary = report.summary();

    assert_eq!(summary.passed, 2);
    assert_eq!(summary.warned, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.total, 2);
}

#[test]
fn test_report_summary_empty() {
    let report = create_empty_report();
    let summary = report.summary();

    assert_eq!(summary.passed, 0);
    assert_eq!(summary.warned, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.total, 0);
}
