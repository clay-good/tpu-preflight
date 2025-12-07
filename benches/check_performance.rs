//! Performance benchmarks for tpu-preflight.
//!
//! Validates that the tool runs quickly enough for CI/CD integration.
//! Target: Complete all checks in under 30 seconds.

use std::time::{Duration, Instant};
use tpu_preflight::cli::output::{JsonFormatter, JunitFormatter, OutputFormatter, TerminalFormatter};
use tpu_preflight::engine::orchestrator::{CheckOrchestrator, OrchestratorConfig, RegisteredCheck};
use tpu_preflight::engine::result::ValidationReport;
use tpu_preflight::{Check, CheckCategory, CheckResult};

/// Create a mock check that completes quickly
fn create_fast_check(id: &str, name: &str, category: CheckCategory) -> RegisteredCheck {
    let id_clone = id.to_string();
    RegisteredCheck {
        id: id.to_string(),
        name: name.to_string(),
        category,
        description: format!("Benchmark check {}", id),
        check_fn: Box::new(move || {
            // Simulate minimal work
            std::thread::sleep(Duration::from_micros(100));
            CheckResult::Pass {
                message: format!("{} completed", id_clone),
                duration_ms: 1,
            }
        }),
        dependencies: vec![],
        estimated_duration_ms: 10,
    }
}

/// Create a large validation report for output formatting benchmarks
fn create_large_report(num_checks: usize) -> ValidationReport {
    let mut checks = Vec::with_capacity(num_checks);

    for i in 0..num_checks {
        let category = match i % 5 {
            0 => CheckCategory::Hardware,
            1 => CheckCategory::Stack,
            2 => CheckCategory::Performance,
            3 => CheckCategory::Io,
            _ => CheckCategory::Security,
        };

        let result = match i % 4 {
            0 => CheckResult::Pass {
                message: format!("Check {} passed successfully", i),
                duration_ms: 100,
            },
            1 => CheckResult::Warn {
                message: format!("Check {} has warnings", i),
                details: "Some warning details here".to_string(),
                duration_ms: 150,
            },
            2 => CheckResult::Fail {
                message: format!("Check {} failed", i),
                details: "Failure details with more information".to_string(),
                duration_ms: 200,
            },
            _ => CheckResult::Skip {
                reason: format!("Check {} was skipped", i),
            },
        };

        checks.push(Check {
            id: format!("BENCH-{:04}", i),
            name: format!("Benchmark Check {}", i),
            category,
            description: format!("This is benchmark check number {}", i),
            result: Some(result),
        });
    }

    ValidationReport {
        timestamp: 1733500000,
        hostname: "benchmark-host".to_string(),
        tpu_type: Some("v5e-benchmark".to_string()),
        checks,
        total_duration_ms: num_checks as u64 * 100,
    }
}

/// Benchmark: Check execution overhead
/// Target: < 10ms overhead per check
fn bench_single_check_overhead() {
    let config = OrchestratorConfig::default();
    let mut orchestrator = CheckOrchestrator::new(config);

    orchestrator.register_check(create_fast_check("BENCH-001", "Single Check", CheckCategory::Hardware));

    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = orchestrator.run_all();
    }

    let elapsed = start.elapsed();
    let per_iteration = elapsed / iterations;

    println!("Single check overhead: {:?} per iteration", per_iteration);
    println!("  Target: < 10ms");
    println!("  Result: {}", if per_iteration < Duration::from_millis(10) { "PASS" } else { "FAIL" });

    assert!(per_iteration < Duration::from_millis(10),
        "Check overhead too high: {:?}", per_iteration);
}

/// Benchmark: Multiple checks sequential
/// Target: < 5ms overhead per check when running many
fn bench_multiple_checks_sequential() {
    let config = OrchestratorConfig::default();
    let mut orchestrator = CheckOrchestrator::new(config);

    // Register 31 checks (same as real implementation)
    for i in 0..31 {
        let category = match i % 5 {
            0 => CheckCategory::Hardware,
            1 => CheckCategory::Stack,
            2 => CheckCategory::Performance,
            3 => CheckCategory::Io,
            _ => CheckCategory::Security,
        };
        orchestrator.register_check(create_fast_check(
            &format!("BENCH-{:03}", i),
            &format!("Bench Check {}", i),
            category,
        ));
    }

    let iterations = 10;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = orchestrator.run_all();
    }

    let elapsed = start.elapsed();
    let per_iteration = elapsed / iterations;
    let per_check = per_iteration / 31;

    println!("31 checks sequential: {:?} total, {:?} per check", per_iteration, per_check);
    println!("  Target: < 5ms per check overhead");
    println!("  Result: {}", if per_check < Duration::from_millis(5) { "PASS" } else { "FAIL" });

    assert!(per_check < Duration::from_millis(5),
        "Per-check overhead too high: {:?}", per_check);
}

/// Benchmark: Multiple checks parallel
/// Target: Parallel should be faster than sequential for independent checks
fn bench_multiple_checks_parallel() {
    let sequential_config = OrchestratorConfig {
        parallel: false,
        ..Default::default()
    };

    let parallel_config = OrchestratorConfig {
        parallel: true,
        max_parallel: 4,
        ..Default::default()
    };

    // Create checks with no dependencies (can run in parallel)
    let create_checks = |orchestrator: &mut CheckOrchestrator| {
        for i in 0..20 {
            orchestrator.register_check(create_fast_check(
                &format!("BENCH-{:03}", i),
                &format!("Bench Check {}", i),
                CheckCategory::Hardware,
            ));
        }
    };

    // Sequential run
    let mut seq_orchestrator = CheckOrchestrator::new(sequential_config);
    create_checks(&mut seq_orchestrator);

    let seq_start = Instant::now();
    let _ = seq_orchestrator.run_all();
    let seq_elapsed = seq_start.elapsed();

    // Parallel run
    let mut par_orchestrator = CheckOrchestrator::new(parallel_config);
    create_checks(&mut par_orchestrator);

    let par_start = Instant::now();
    let _ = par_orchestrator.run_all();
    let par_elapsed = par_start.elapsed();

    println!("Sequential: {:?}", seq_elapsed);
    println!("Parallel:   {:?}", par_elapsed);
    println!("  Note: Current parallel implementation runs sequentially in batches");
}

/// Benchmark: Terminal output formatting
/// Target: < 100ms for 100 checks
fn bench_terminal_output_formatting() {
    let report = create_large_report(100);
    let formatter = TerminalFormatter::new(true, false, false);

    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = formatter.format(&report);
    }

    let elapsed = start.elapsed();
    let per_iteration = elapsed / iterations;

    println!("Terminal formatting (100 checks): {:?}", per_iteration);
    println!("  Target: < 100ms");
    println!("  Result: {}", if per_iteration < Duration::from_millis(100) { "PASS" } else { "FAIL" });

    assert!(per_iteration < Duration::from_millis(100),
        "Terminal formatting too slow: {:?}", per_iteration);
}

/// Benchmark: JSON output formatting
/// Target: < 100ms for 100 checks
fn bench_json_output_formatting() {
    let report = create_large_report(100);
    let formatter = JsonFormatter::new(true);

    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = formatter.format(&report);
    }

    let elapsed = start.elapsed();
    let per_iteration = elapsed / iterations;

    println!("JSON formatting (100 checks): {:?}", per_iteration);
    println!("  Target: < 100ms");
    println!("  Result: {}", if per_iteration < Duration::from_millis(100) { "PASS" } else { "FAIL" });

    assert!(per_iteration < Duration::from_millis(100),
        "JSON formatting too slow: {:?}", per_iteration);
}

/// Benchmark: JUnit output formatting
/// Target: < 100ms for 100 checks
fn bench_junit_output_formatting() {
    let report = create_large_report(100);
    let formatter = JunitFormatter::new();

    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = formatter.format(&report);
    }

    let elapsed = start.elapsed();
    let per_iteration = elapsed / iterations;

    println!("JUnit formatting (100 checks): {:?}", per_iteration);
    println!("  Target: < 100ms");
    println!("  Result: {}", if per_iteration < Duration::from_millis(100) { "PASS" } else { "FAIL" });

    assert!(per_iteration < Duration::from_millis(100),
        "JUnit formatting too slow: {:?}", per_iteration);
}

/// Benchmark: Output size for large reports
fn bench_output_sizes() {
    let report = create_large_report(100);

    let terminal_formatter = TerminalFormatter::new(false, false, false);
    let json_formatter = JsonFormatter::new(true);
    let junit_formatter = JunitFormatter::new();

    let terminal_output = terminal_formatter.format(&report);
    let json_output = json_formatter.format(&report);
    let junit_output = junit_formatter.format(&report);

    println!("Output sizes for 100 checks:");
    println!("  Terminal: {} bytes", terminal_output.len());
    println!("  JSON:     {} bytes", json_output.len());
    println!("  JUnit:    {} bytes", junit_output.len());
}

/// Run all benchmarks
fn main() {
    println!("=== tpu-preflight Performance Benchmarks ===\n");

    println!("--- Check Execution Benchmarks ---");
    bench_single_check_overhead();
    println!();

    bench_multiple_checks_sequential();
    println!();

    bench_multiple_checks_parallel();
    println!();

    println!("--- Output Formatting Benchmarks ---");
    bench_terminal_output_formatting();
    println!();

    bench_json_output_formatting();
    println!();

    bench_junit_output_formatting();
    println!();

    println!("--- Output Size Benchmarks ---");
    bench_output_sizes();
    println!();

    println!("=== Benchmarks Complete ===");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_check_overhead() {
        bench_single_check_overhead();
    }

    #[test]
    fn test_multiple_checks_sequential() {
        bench_multiple_checks_sequential();
    }

    #[test]
    fn test_terminal_output_formatting() {
        bench_terminal_output_formatting();
    }

    #[test]
    fn test_json_output_formatting() {
        bench_json_output_formatting();
    }

    #[test]
    fn test_junit_output_formatting() {
        bench_junit_output_formatting();
    }
}
