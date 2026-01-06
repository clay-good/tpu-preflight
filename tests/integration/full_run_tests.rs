//! Full run integration tests.
//!
//! Tests for complete validation runs, including orchestration,
//! fail-fast behavior, and result aggregation.

use tpu_doc::engine::orchestrator::{CheckOrchestrator, OrchestratorConfig, RegisteredCheck};
use tpu_doc::engine::result::{ResultAggregator, ValidationReport};
use tpu_doc::{Check, CheckCategory, CheckResult, TpuDocConfig};

// Helper to create a check that always passes
fn create_passing_check(id: &str, name: &str, category: CheckCategory) -> RegisteredCheck {
    let id_clone = id.to_string();
    RegisteredCheck {
        id: id.to_string(),
        name: name.to_string(),
        category,
        description: format!("Test check {}", id),
        check_fn: Box::new(move || CheckResult::Pass {
            message: format!("{} passed", id_clone),
            duration_ms: 10,
        }),
        dependencies: vec![],
        estimated_duration_ms: 100,
    }
}

// Helper to create a check that always fails
fn create_failing_check(id: &str, name: &str, category: CheckCategory) -> RegisteredCheck {
    let id_clone = id.to_string();
    RegisteredCheck {
        id: id.to_string(),
        name: name.to_string(),
        category,
        description: format!("Test check {}", id),
        check_fn: Box::new(move || CheckResult::Fail {
            message: format!("{} failed", id_clone),
            details: "Test failure".to_string(),
            duration_ms: 10,
        }),
        dependencies: vec![],
        estimated_duration_ms: 100,
    }
}

// Helper to create a check that always warns
fn create_warning_check(id: &str, name: &str, category: CheckCategory) -> RegisteredCheck {
    let id_clone = id.to_string();
    RegisteredCheck {
        id: id.to_string(),
        name: name.to_string(),
        category,
        description: format!("Test check {}", id),
        check_fn: Box::new(move || CheckResult::Warn {
            message: format!("{} warning", id_clone),
            details: "Test warning".to_string(),
            duration_ms: 10,
        }),
        dependencies: vec![],
        estimated_duration_ms: 100,
    }
}

// Helper to create a check that always skips
fn create_skipping_check(id: &str, name: &str, category: CheckCategory) -> RegisteredCheck {
    let id_clone = id.to_string();
    RegisteredCheck {
        id: id.to_string(),
        name: name.to_string(),
        category,
        description: format!("Test check {}", id),
        check_fn: Box::new(move || CheckResult::Skip {
            reason: format!("{} skipped", id_clone),
        }),
        dependencies: vec![],
        estimated_duration_ms: 100,
    }
}

// Orchestrator tests

#[test]
fn test_orchestrator_run_all_checks() {
    let config = OrchestratorConfig::default();
    let mut orchestrator = CheckOrchestrator::new(config);

    orchestrator.register_check(create_passing_check("TEST-001", "Test 1", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("TEST-002", "Test 2", CheckCategory::Stack));
    orchestrator.register_check(create_passing_check("TEST-003", "Test 3", CheckCategory::Io));

    let report = orchestrator.run_all();
    let summary = report.summary();

    assert_eq!(summary.total, 3);
    assert_eq!(summary.passed, 3);
    assert_eq!(summary.failed, 0);
}

#[test]
fn test_orchestrator_run_category() {
    let config = OrchestratorConfig::default();
    let mut orchestrator = CheckOrchestrator::new(config);

    orchestrator.register_check(create_passing_check("HW-001", "Hardware 1", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("HW-002", "Hardware 2", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("STK-001", "Stack 1", CheckCategory::Stack));

    let report = orchestrator.run_category(CheckCategory::Hardware);
    let summary = report.summary();

    assert_eq!(summary.total, 2);
    assert_eq!(summary.passed, 2);
}

#[test]
fn test_orchestrator_run_specific() {
    let config = OrchestratorConfig::default();
    let mut orchestrator = CheckOrchestrator::new(config);

    orchestrator.register_check(create_passing_check("TEST-001", "Test 1", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("TEST-002", "Test 2", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("TEST-003", "Test 3", CheckCategory::Hardware));

    let report = orchestrator.run_specific(&[
        "TEST-001".to_string(),
        "TEST-003".to_string(),
    ]);
    let summary = report.summary();

    assert_eq!(summary.total, 2);
}

#[test]
fn test_orchestrator_run_excluding() {
    let config = OrchestratorConfig::default();
    let mut orchestrator = CheckOrchestrator::new(config);

    orchestrator.register_check(create_passing_check("TEST-001", "Test 1", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("TEST-002", "Test 2", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("TEST-003", "Test 3", CheckCategory::Hardware));

    let report = orchestrator.run_excluding(&["TEST-002".to_string()]);
    let summary = report.summary();

    assert_eq!(summary.total, 2);
}

#[test]
fn test_orchestrator_fail_fast() {
    let config = OrchestratorConfig {
        fail_fast: true,
        ..Default::default()
    };
    let mut orchestrator = CheckOrchestrator::new(config);

    orchestrator.register_check(create_passing_check("TEST-001", "Test 1", CheckCategory::Hardware));
    orchestrator.register_check(create_failing_check("TEST-002", "Test 2", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("TEST-003", "Test 3", CheckCategory::Hardware));

    let report = orchestrator.run_all();
    let summary = report.summary();

    // Should stop after first failure
    assert_eq!(summary.failed, 1);
    assert!(summary.total <= 2); // May have run 1 or 2 checks before failing
}

#[test]
fn test_orchestrator_mixed_results() {
    let config = OrchestratorConfig::default();
    let mut orchestrator = CheckOrchestrator::new(config);

    orchestrator.register_check(create_passing_check("TEST-001", "Test 1", CheckCategory::Hardware));
    orchestrator.register_check(create_warning_check("TEST-002", "Test 2", CheckCategory::Hardware));
    orchestrator.register_check(create_failing_check("TEST-003", "Test 3", CheckCategory::Hardware));
    orchestrator.register_check(create_skipping_check("TEST-004", "Test 4", CheckCategory::Hardware));

    let report = orchestrator.run_all();
    let summary = report.summary();

    assert_eq!(summary.passed, 1);
    assert_eq!(summary.warned, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.total, 4);
}

#[test]
fn test_orchestrator_dependencies() {
    let config = OrchestratorConfig::default();
    let mut orchestrator = CheckOrchestrator::new(config);

    // Check with no dependencies
    orchestrator.register_check(create_passing_check("TEST-001", "Test 1", CheckCategory::Hardware));

    // Check that depends on TEST-001
    let mut check2 = create_passing_check("TEST-002", "Test 2", CheckCategory::Hardware);
    check2.dependencies = vec!["TEST-001".to_string()];
    orchestrator.register_check(check2);

    // Check that depends on TEST-002
    let mut check3 = create_passing_check("TEST-003", "Test 3", CheckCategory::Hardware);
    check3.dependencies = vec!["TEST-002".to_string()];
    orchestrator.register_check(check3);

    let report = orchestrator.run_all();
    let summary = report.summary();

    assert_eq!(summary.total, 3);
    assert_eq!(summary.passed, 3);
}

#[test]
fn test_orchestrator_parallel_mode() {
    let config = OrchestratorConfig {
        parallel: true,
        max_parallel: 4,
        ..Default::default()
    };
    let mut orchestrator = CheckOrchestrator::new(config);

    orchestrator.register_check(create_passing_check("TEST-001", "Test 1", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("TEST-002", "Test 2", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("TEST-003", "Test 3", CheckCategory::Hardware));
    orchestrator.register_check(create_passing_check("TEST-004", "Test 4", CheckCategory::Hardware));

    let report = orchestrator.run_all();
    let summary = report.summary();

    assert_eq!(summary.total, 4);
    assert_eq!(summary.passed, 4);
}

// Result aggregator tests

#[test]
fn test_result_aggregator_add_results() {
    let mut aggregator = ResultAggregator::new();

    aggregator.add_result(Check {
        id: "TEST-001".to_string(),
        name: "Test 1".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Pass {
            message: "OK".to_string(),
            duration_ms: 100,
        }),
    });

    aggregator.add_result(Check {
        id: "TEST-002".to_string(),
        name: "Test 2".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Fail {
            message: "Failed".to_string(),
            details: "Details".to_string(),
            duration_ms: 100,
        }),
    });

    let summary = aggregator.get_summary();
    assert_eq!(summary.passed, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.total, 2);
}

#[test]
fn test_result_aggregator_has_failures() {
    let mut aggregator = ResultAggregator::new();

    aggregator.add_result(Check {
        id: "TEST-001".to_string(),
        name: "Test 1".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Pass {
            message: "OK".to_string(),
            duration_ms: 100,
        }),
    });

    assert!(!aggregator.has_failures());

    aggregator.add_result(Check {
        id: "TEST-002".to_string(),
        name: "Test 2".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Fail {
            message: "Failed".to_string(),
            details: "Details".to_string(),
            duration_ms: 100,
        }),
    });

    assert!(aggregator.has_failures());
}

#[test]
fn test_result_aggregator_get_by_category() {
    let mut aggregator = ResultAggregator::new();

    aggregator.add_result(Check {
        id: "HW-001".to_string(),
        name: "Hardware 1".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Pass {
            message: "OK".to_string(),
            duration_ms: 100,
        }),
    });

    aggregator.add_result(Check {
        id: "STK-001".to_string(),
        name: "Stack 1".to_string(),
        category: CheckCategory::Stack,
        description: "Test".to_string(),
        result: Some(CheckResult::Pass {
            message: "OK".to_string(),
            duration_ms: 100,
        }),
    });

    let hw_checks = aggregator.get_by_category(CheckCategory::Hardware);
    assert_eq!(hw_checks.len(), 1);
    assert_eq!(hw_checks[0].id, "HW-001");

    let stk_checks = aggregator.get_by_category(CheckCategory::Stack);
    assert_eq!(stk_checks.len(), 1);
    assert_eq!(stk_checks[0].id, "STK-001");
}

#[test]
fn test_result_aggregator_get_failures() {
    let mut aggregator = ResultAggregator::new();

    aggregator.add_result(Check {
        id: "TEST-001".to_string(),
        name: "Test 1".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Pass {
            message: "OK".to_string(),
            duration_ms: 100,
        }),
    });

    aggregator.add_result(Check {
        id: "TEST-002".to_string(),
        name: "Test 2".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Fail {
            message: "Failed".to_string(),
            details: "Details".to_string(),
            duration_ms: 100,
        }),
    });

    let failures = aggregator.get_failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].id, "TEST-002");
}

#[test]
fn test_result_aggregator_get_warnings() {
    let mut aggregator = ResultAggregator::new();

    aggregator.add_result(Check {
        id: "TEST-001".to_string(),
        name: "Test 1".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Pass {
            message: "OK".to_string(),
            duration_ms: 100,
        }),
    });

    aggregator.add_result(Check {
        id: "TEST-002".to_string(),
        name: "Test 2".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Warn {
            message: "Warning".to_string(),
            details: "Details".to_string(),
            duration_ms: 100,
        }),
    });

    let warnings = aggregator.get_warnings();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].id, "TEST-002");
}

#[test]
fn test_result_aggregator_to_report() {
    let mut aggregator = ResultAggregator::new();

    aggregator.add_result(Check {
        id: "TEST-001".to_string(),
        name: "Test 1".to_string(),
        category: CheckCategory::Hardware,
        description: "Test".to_string(),
        result: Some(CheckResult::Pass {
            message: "OK".to_string(),
            duration_ms: 100,
        }),
    });

    aggregator.set_metadata(
        "test-host".to_string(),
        Some("v5e".to_string()),
        500,
    );

    let report = aggregator.to_report();

    assert_eq!(report.hostname, "test-host");
    assert_eq!(report.tpu_type, Some("v5e".to_string()));
    assert_eq!(report.total_duration_ms, 500);
    assert_eq!(report.checks.len(), 1);
}

// TpuDocConfig tests

#[test]
fn test_tpu_doc_config_default() {
    let config = TpuDocConfig::default();

    assert!(config.categories.is_none());
    assert!(config.skip_checks.is_empty());
    assert!(config.only_checks.is_empty());
    assert!(!config.parallel);
    assert!(!config.fail_fast);
    assert_eq!(config.timeout_ms, 30000);
}

#[test]
fn test_tpu_doc_config_with_category() {
    let config = TpuDocConfig {
        categories: Some(vec![CheckCategory::Hardware]),
        ..Default::default()
    };

    assert!(config.categories.is_some());
    assert_eq!(config.categories.as_ref().unwrap().len(), 1);
}

#[test]
fn test_tpu_doc_config_with_skip() {
    let config = TpuDocConfig {
        skip_checks: vec!["HW-001".to_string(), "HW-002".to_string()],
        ..Default::default()
    };

    assert_eq!(config.skip_checks.len(), 2);
}

#[test]
fn test_tpu_doc_config_with_only() {
    let config = TpuDocConfig {
        only_checks: vec!["IO-006".to_string()],
        ..Default::default()
    };

    assert_eq!(config.only_checks.len(), 1);
}

// ValidationReport tests

#[test]
fn test_validation_report_new() {
    let report = ValidationReport::new();

    assert!(report.hostname.is_empty());
    assert!(report.tpu_type.is_none());
    assert!(report.checks.is_empty());
    assert_eq!(report.total_duration_ms, 0);
}

#[test]
fn test_validation_report_summary() {
    let report = ValidationReport {
        timestamp: 0,
        hostname: "test".to_string(),
        tpu_type: None,
        checks: vec![
            Check {
                id: "TEST-001".to_string(),
                name: "Test 1".to_string(),
                category: CheckCategory::Hardware,
                description: "Test".to_string(),
                result: Some(CheckResult::Pass {
                    message: "OK".to_string(),
                    duration_ms: 100,
                }),
            },
            Check {
                id: "TEST-002".to_string(),
                name: "Test 2".to_string(),
                category: CheckCategory::Hardware,
                description: "Test".to_string(),
                result: Some(CheckResult::Fail {
                    message: "Failed".to_string(),
                    details: "Details".to_string(),
                    duration_ms: 100,
                }),
            },
        ],
        total_duration_ms: 200,
    };

    let summary = report.summary();

    assert_eq!(summary.passed, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.total, 2);
    assert_eq!(summary.total_duration_ms, 200);
}
