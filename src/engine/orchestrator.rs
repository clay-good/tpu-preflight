//! Check execution orchestrator.
//!
//! Manages check registration, dependency resolution, and execution.
//!
//! # Graceful Degradation
//!
//! This module handles errors gracefully:
//! - Check panics: Caught via std::panic::catch_unwind, converted to Fail result
//! - Check timeout: Returns Fail result with timeout message (when parallel enabled)
//! - Dependency failure: Continues with remaining checks unless fail_fast
//! - Invalid check ID: Silently skipped in run_specific/run_excluding
//! - Empty check list: Returns empty report (not an error)
//!
//! The orchestrator ensures all registered checks complete (or are skipped)
//! regardless of individual check failures, unless fail_fast is enabled.
//! No function in this module will panic.

use crate::engine::result::{ResultAggregator, ValidationReport};
use crate::platform::{linux, tpu};
use crate::{Check, CheckCategory, CheckResult};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Orchestrator configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub parallel: bool,
    pub fail_fast: bool,
    pub timeout_ms: u64,
    pub max_parallel: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        OrchestratorConfig {
            parallel: false,
            fail_fast: false,
            timeout_ms: 30000,
            max_parallel: 4,
        }
    }
}

/// A registered check with its execution function
pub struct RegisteredCheck {
    pub id: String,
    pub name: String,
    pub category: CheckCategory,
    pub description: String,
    pub check_fn: Box<dyn Fn() -> CheckResult + Send + Sync>,
    pub dependencies: Vec<String>,
    pub estimated_duration_ms: u64,
}

/// Check orchestrator
pub struct CheckOrchestrator {
    config: OrchestratorConfig,
    checks: Vec<RegisteredCheck>,
}

impl CheckOrchestrator {
    /// Create a new orchestrator with the given configuration
    pub fn new(config: OrchestratorConfig) -> Self {
        CheckOrchestrator {
            config,
            checks: Vec::new(),
        }
    }

    /// Register checks for execution
    pub fn register_checks(&mut self, checks: Vec<RegisteredCheck>) {
        self.checks.extend(checks);
    }

    /// Register a single check
    pub fn register_check(&mut self, check: RegisteredCheck) {
        self.checks.push(check);
    }

    /// Run all registered checks
    pub fn run_all(&self) -> ValidationReport {
        self.run_checks(&self.checks.iter().map(|c| c.id.clone()).collect::<Vec<_>>())
    }

    /// Run checks in a specific category
    pub fn run_category(&self, category: CheckCategory) -> ValidationReport {
        let ids: Vec<String> = self
            .checks
            .iter()
            .filter(|c| c.category == category)
            .map(|c| c.id.clone())
            .collect();
        self.run_checks(&ids)
    }

    /// Run checks in multiple categories
    pub fn run_categories(&self, categories: &[CheckCategory]) -> ValidationReport {
        let ids: Vec<String> = self
            .checks
            .iter()
            .filter(|c| categories.contains(&c.category))
            .map(|c| c.id.clone())
            .collect();
        self.run_checks(&ids)
    }

    /// Run specific checks by ID
    pub fn run_specific(&self, check_ids: &[String]) -> ValidationReport {
        self.run_checks(check_ids)
    }

    /// Run all checks except specified IDs
    pub fn run_excluding(&self, skip_ids: &[String]) -> ValidationReport {
        let ids: Vec<String> = self
            .checks
            .iter()
            .filter(|c| !skip_ids.contains(&c.id))
            .map(|c| c.id.clone())
            .collect();
        self.run_checks(&ids)
    }

    /// Execute the specified checks
    fn run_checks(&self, check_ids: &[String]) -> ValidationReport {
        let start = Instant::now();
        let aggregator = Arc::new(Mutex::new(ResultAggregator::new()));

        // Get checks to run in order (respecting dependencies)
        let ordered_checks = self.resolve_dependencies(check_ids);

        if self.config.parallel {
            self.run_parallel(&ordered_checks, aggregator.clone());
        } else {
            self.run_sequential(&ordered_checks, aggregator.clone());
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;

        // Build report
        let hostname = linux::get_hostname().unwrap_or_else(|_| "unknown".to_string());
        let tpu_type = tpu::get_tpu_type().ok().map(|t| t.to_string());

        // Handle potential mutex poisoning gracefully
        let report = match aggregator.lock() {
            Ok(mut agg) => {
                agg.set_metadata(hostname, tpu_type, total_duration_ms);
                agg.to_report()
            }
            Err(poisoned) => {
                // If the mutex is poisoned, recover the data anyway
                let mut agg = poisoned.into_inner();
                agg.set_metadata(hostname, tpu_type, total_duration_ms);
                agg.to_report()
            }
        };
        report
    }

    /// Run checks sequentially
    fn run_sequential(&self, check_ids: &[String], aggregator: Arc<Mutex<ResultAggregator>>) {
        for check_id in check_ids {
            if let Some(check) = self.checks.iter().find(|c| &c.id == check_id) {
                let result = self.execute_check(check);

                let check_struct = Check {
                    id: check.id.clone(),
                    name: check.name.clone(),
                    category: check.category.clone(),
                    description: check.description.clone(),
                    result: Some(result.clone()),
                };

                if let Ok(mut agg) = aggregator.lock() {
                    agg.add_result(check_struct);
                }

                // Check for fail-fast
                if self.config.fail_fast {
                    if let CheckResult::Fail { .. } = result {
                        break;
                    }
                }
            }
        }
    }

    /// Run checks in parallel (where safe)
    fn run_parallel(&self, check_ids: &[String], aggregator: Arc<Mutex<ResultAggregator>>) {
        use std::thread;

        // Group checks that can run in parallel (no dependencies between them)
        let mut remaining: Vec<String> = check_ids.to_vec();
        let mut completed: Vec<String> = Vec::new();

        while !remaining.is_empty() {
            // Find checks that can run (all dependencies satisfied)
            let runnable: Vec<String> = remaining
                .iter()
                .filter(|id| {
                    if let Some(check) = self.checks.iter().find(|c| &c.id == *id) {
                        check.dependencies.iter().all(|dep| completed.contains(dep))
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            if runnable.is_empty() {
                // No checks can run - might have circular dependencies
                // Fall back to running remaining sequentially
                for id in &remaining {
                    if let Some(check) = self.checks.iter().find(|c| &c.id == id) {
                        let result = self.execute_check(check);
                        let check_struct = Check {
                            id: check.id.clone(),
                            name: check.name.clone(),
                            category: check.category.clone(),
                            description: check.description.clone(),
                            result: Some(result),
                        };
                        if let Ok(mut agg) = aggregator.lock() {
                            agg.add_result(check_struct);
                        }
                    }
                }
                break;
            }

            // Run runnable checks in parallel (up to max_parallel)
            let batch: Vec<_> = runnable
                .iter()
                .take(self.config.max_parallel)
                .cloned()
                .collect();

            // Collect check info for parallel execution
            let batch_checks: Vec<_> = batch
                .iter()
                .filter_map(|check_id| {
                    self.checks.iter().find(|c| &c.id == check_id).map(|c| {
                        (
                            c.id.clone(),
                            c.name.clone(),
                            c.category.clone(),
                            c.description.clone(),
                            c.check_fn.as_ref() as *const (dyn Fn() -> CheckResult + Send + Sync),
                        )
                    })
                })
                .collect();

            // Execute checks in parallel using scoped threads
            let timeout_ms = self.config.timeout_ms;
            let results: Vec<_> = thread::scope(|s| {
                let handles: Vec<_> = batch_checks
                    .iter()
                    .map(|(id, name, category, description, check_fn_ptr)| {
                        let id = id.clone();
                        let name = name.clone();
                        let category = category.clone();
                        let description = description.clone();
                        // Safety: check_fn_ptr is valid for the duration of this scope
                        let check_fn = unsafe { &*(*check_fn_ptr) };
                        s.spawn(move || {
                            let start = Instant::now();
                            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                check_fn()
                            }));
                            let elapsed = start.elapsed().as_millis() as u64;

                            let check_result = match result {
                                Ok(r) => {
                                    if elapsed > timeout_ms {
                                        CheckResult::Fail {
                                            message: format!("Check timed out after {}ms", elapsed),
                                            details: "Check exceeded global timeout".to_string(),
                                            duration_ms: elapsed,
                                        }
                                    } else {
                                        r
                                    }
                                }
                                Err(_) => CheckResult::Fail {
                                    message: "Check panicked during execution".to_string(),
                                    details: "An unexpected error occurred".to_string(),
                                    duration_ms: elapsed,
                                },
                            };

                            Check {
                                id,
                                name,
                                category,
                                description,
                                result: Some(check_result),
                            }
                        })
                    })
                    .collect();

                handles
                    .into_iter()
                    .filter_map(|h| h.join().ok())
                    .collect()
            });

            // Add results to aggregator
            for check_struct in results {
                if let Ok(mut agg) = aggregator.lock() {
                    agg.add_result(check_struct);
                }
            }

            // Mark as completed
            for id in &batch {
                completed.push(id.clone());
                remaining.retain(|r| r != id);
            }

            // Check fail-fast
            if self.config.fail_fast {
                if let Ok(agg) = aggregator.lock() {
                    if agg.has_failures() {
                        break;
                    }
                }
            }
        }
    }

    /// Execute a single check with timeout handling
    fn execute_check(&self, check: &RegisteredCheck) -> CheckResult {
        let start = Instant::now();

        // Execute the check function
        // In a production implementation, we'd use panic::catch_unwind
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (check.check_fn)()
        }));

        match result {
            Ok(check_result) => {
                // Check if we exceeded timeout
                let elapsed = start.elapsed().as_millis() as u64;
                if elapsed > self.config.timeout_ms {
                    CheckResult::Fail {
                        message: format!("Check timed out after {}ms", elapsed),
                        details: "Check exceeded global timeout".to_string(),
                        duration_ms: elapsed,
                    }
                } else {
                    check_result
                }
            }
            Err(_) => CheckResult::Fail {
                message: "Check panicked during execution".to_string(),
                details: "An unexpected error occurred".to_string(),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Resolve check dependencies and return ordered list
    fn resolve_dependencies(&self, check_ids: &[String]) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();

        fn visit(
            id: &str,
            checks: &[RegisteredCheck],
            check_ids: &[String],
            visited: &mut std::collections::HashSet<String>,
            result: &mut Vec<String>,
        ) {
            if visited.contains(id) {
                return;
            }

            if let Some(check) = checks.iter().find(|c| c.id == id) {
                // Visit dependencies first
                for dep in &check.dependencies {
                    if check_ids.contains(dep) {
                        visit(dep, checks, check_ids, visited, result);
                    }
                }
            }

            visited.insert(id.to_string());
            if check_ids.contains(&id.to_string()) {
                result.push(id.to_string());
            }
        }

        for id in check_ids {
            visit(id, &self.checks, check_ids, &mut visited, &mut result);
        }

        result
    }
}

/// Create all registered checks with their execution functions
pub fn create_all_checks() -> Vec<RegisteredCheck> {
    use crate::checks::{config, hardware, io, performance, security, stack};

    let mut checks = Vec::new();

    // Hardware checks
    checks.push(RegisteredCheck {
        id: "HW-001".to_string(),
        name: "TPU Device Detection".to_string(),
        category: CheckCategory::Hardware,
        description: "Verify expected number of TPU chips are present".to_string(),
        check_fn: Box::new(hardware::run_hw001),
        dependencies: vec![],
        estimated_duration_ms: 1000,
    });

    checks.push(RegisteredCheck {
        id: "HW-002".to_string(),
        name: "HBM Memory Availability".to_string(),
        category: CheckCategory::Hardware,
        description: "Check total HBM capacity and availability".to_string(),
        check_fn: Box::new(hardware::run_hw002),
        dependencies: vec!["HW-001".to_string()],
        estimated_duration_ms: 1000,
    });

    checks.push(RegisteredCheck {
        id: "HW-003".to_string(),
        name: "TPU Thermal Status".to_string(),
        category: CheckCategory::Hardware,
        description: "Check temperature of each TPU chip".to_string(),
        check_fn: Box::new(hardware::run_hw003),
        dependencies: vec!["HW-001".to_string()],
        estimated_duration_ms: 500,
    });

    checks.push(RegisteredCheck {
        id: "HW-004".to_string(),
        name: "TPU Error Counters".to_string(),
        category: CheckCategory::Hardware,
        description: "Check for accumulated hardware errors".to_string(),
        check_fn: Box::new(hardware::run_hw004),
        dependencies: vec!["HW-001".to_string()],
        estimated_duration_ms: 500,
    });

    checks.push(RegisteredCheck {
        id: "HW-005".to_string(),
        name: "ICI Interconnect Status".to_string(),
        category: CheckCategory::Hardware,
        description: "Verify inter-chip interconnect is functional".to_string(),
        check_fn: Box::new(hardware::run_hw005),
        dependencies: vec!["HW-001".to_string()],
        estimated_duration_ms: 1000,
    });

    checks.push(RegisteredCheck {
        id: "HW-006".to_string(),
        name: "Driver Status".to_string(),
        category: CheckCategory::Hardware,
        description: "Verify TPU driver kernel module is loaded".to_string(),
        check_fn: Box::new(hardware::run_hw006),
        dependencies: vec![],
        estimated_duration_ms: 500,
    });

    // Stack checks
    checks.push(RegisteredCheck {
        id: "STK-001".to_string(),
        name: "JAX Version".to_string(),
        category: CheckCategory::Stack,
        description: "Detect and validate installed JAX version".to_string(),
        check_fn: Box::new(stack::run_stk001),
        dependencies: vec![],
        estimated_duration_ms: 1000,
    });

    checks.push(RegisteredCheck {
        id: "STK-002".to_string(),
        name: "libtpu Version".to_string(),
        category: CheckCategory::Stack,
        description: "Detect and validate libtpu version".to_string(),
        check_fn: Box::new(stack::run_stk002),
        dependencies: vec![],
        estimated_duration_ms: 500,
    });

    checks.push(RegisteredCheck {
        id: "STK-003".to_string(),
        name: "XLA Compiler Version".to_string(),
        category: CheckCategory::Stack,
        description: "Detect XLA compiler version".to_string(),
        check_fn: Box::new(stack::run_stk003),
        dependencies: vec![],
        estimated_duration_ms: 500,
    });

    checks.push(RegisteredCheck {
        id: "STK-004".to_string(),
        name: "Python Version".to_string(),
        category: CheckCategory::Stack,
        description: "Check Python version compatibility".to_string(),
        check_fn: Box::new(stack::run_stk004),
        dependencies: vec![],
        estimated_duration_ms: 500,
    });

    checks.push(RegisteredCheck {
        id: "STK-005".to_string(),
        name: "PJRT Plugin Status".to_string(),
        category: CheckCategory::Stack,
        description: "Verify PJRT TPU plugin is available".to_string(),
        check_fn: Box::new(stack::run_stk005),
        dependencies: vec![],
        estimated_duration_ms: 500,
    });

    checks.push(RegisteredCheck {
        id: "STK-006".to_string(),
        name: "Dependency Conflicts".to_string(),
        category: CheckCategory::Stack,
        description: "Check for known conflicting package versions".to_string(),
        check_fn: Box::new(stack::run_stk006),
        dependencies: vec![],
        estimated_duration_ms: 1000,
    });

    checks.push(RegisteredCheck {
        id: "STK-007".to_string(),
        name: "Environment Variables".to_string(),
        category: CheckCategory::Stack,
        description: "Verify required environment variables are set".to_string(),
        check_fn: Box::new(stack::run_stk007),
        dependencies: vec![],
        estimated_duration_ms: 100,
    });

    // Performance checks
    checks.push(RegisteredCheck {
        id: "PERF-001".to_string(),
        name: "MXU Utilization Test".to_string(),
        category: CheckCategory::Performance,
        description: "Run standardized matrix multiplication and measure MXU utilization".to_string(),
        check_fn: Box::new(performance::run_perf001),
        dependencies: vec!["HW-001".to_string(), "STK-001".to_string()],
        estimated_duration_ms: 10000,
    });

    checks.push(RegisteredCheck {
        id: "PERF-002".to_string(),
        name: "HBM Bandwidth Test".to_string(),
        category: CheckCategory::Performance,
        description: "Measure HBM memory bandwidth".to_string(),
        check_fn: Box::new(performance::run_perf002),
        dependencies: vec!["HW-001".to_string(), "HW-002".to_string()],
        estimated_duration_ms: 5000,
    });

    checks.push(RegisteredCheck {
        id: "PERF-003".to_string(),
        name: "Chip-to-Chip Latency".to_string(),
        category: CheckCategory::Performance,
        description: "Measure latency between TPU chips".to_string(),
        check_fn: Box::new(performance::run_perf003),
        dependencies: vec!["HW-001".to_string(), "HW-005".to_string()],
        estimated_duration_ms: 3000,
    });

    checks.push(RegisteredCheck {
        id: "PERF-004".to_string(),
        name: "Compilation Latency".to_string(),
        category: CheckCategory::Performance,
        description: "Measure XLA compilation time for standard graph".to_string(),
        check_fn: Box::new(performance::run_perf004),
        dependencies: vec!["STK-001".to_string(), "STK-003".to_string()],
        estimated_duration_ms: 60000,
    });

    checks.push(RegisteredCheck {
        id: "PERF-005".to_string(),
        name: "Memory Pressure Test".to_string(),
        category: CheckCategory::Performance,
        description: "Allocate and free HBM to verify no fragmentation issues".to_string(),
        check_fn: Box::new(performance::run_perf005),
        dependencies: vec!["HW-002".to_string()],
        estimated_duration_ms: 5000,
    });

    // I/O checks
    checks.push(RegisteredCheck {
        id: "IO-001".to_string(),
        name: "GCS Read Throughput".to_string(),
        category: CheckCategory::Io,
        description: "Measure read throughput from Google Cloud Storage".to_string(),
        check_fn: Box::new(io::run_io001),
        dependencies: vec!["IO-003".to_string()],
        estimated_duration_ms: 10000,
    });

    checks.push(RegisteredCheck {
        id: "IO-002".to_string(),
        name: "Local Disk Throughput".to_string(),
        category: CheckCategory::Io,
        description: "Measure sequential read/write to local SSD".to_string(),
        check_fn: Box::new(io::run_io002),
        dependencies: vec![],
        estimated_duration_ms: 5000,
    });

    checks.push(RegisteredCheck {
        id: "IO-003".to_string(),
        name: "GCS Connectivity".to_string(),
        category: CheckCategory::Io,
        description: "Verify connectivity to storage.googleapis.com".to_string(),
        check_fn: Box::new(io::run_io003),
        dependencies: vec!["IO-006".to_string()],
        estimated_duration_ms: 2000,
    });

    checks.push(RegisteredCheck {
        id: "IO-004".to_string(),
        name: "Checkpoint Directory Access".to_string(),
        category: CheckCategory::Io,
        description: "Verify checkpoint directory access and space".to_string(),
        check_fn: Box::new(io::run_io004),
        dependencies: vec![],
        estimated_duration_ms: 1000,
    });

    checks.push(RegisteredCheck {
        id: "IO-005".to_string(),
        name: "Network Latency to GCP Services".to_string(),
        category: CheckCategory::Io,
        description: "Measure latency to GCP services".to_string(),
        check_fn: Box::new(io::run_io005),
        dependencies: vec!["IO-006".to_string()],
        estimated_duration_ms: 5000,
    });

    checks.push(RegisteredCheck {
        id: "IO-006".to_string(),
        name: "DNS Resolution".to_string(),
        category: CheckCategory::Io,
        description: "Verify DNS resolution is working".to_string(),
        check_fn: Box::new(io::run_io006),
        dependencies: vec![],
        estimated_duration_ms: 2000,
    });

    // Security checks
    checks.push(RegisteredCheck {
        id: "SEC-001".to_string(),
        name: "Service Account Permissions".to_string(),
        category: CheckCategory::Security,
        description: "Identify service account and check for overly permissive roles".to_string(),
        check_fn: Box::new(security::run_sec001),
        dependencies: vec![],
        estimated_duration_ms: 2000,
    });

    checks.push(RegisteredCheck {
        id: "SEC-002".to_string(),
        name: "Network Exposure".to_string(),
        category: CheckCategory::Security,
        description: "Check for services listening on all interfaces".to_string(),
        check_fn: Box::new(security::run_sec002),
        dependencies: vec![],
        estimated_duration_ms: 500,
    });

    checks.push(RegisteredCheck {
        id: "SEC-003".to_string(),
        name: "Workload Identity Status".to_string(),
        category: CheckCategory::Security,
        description: "Check if workload identity is configured".to_string(),
        check_fn: Box::new(security::run_sec003),
        dependencies: vec!["SEC-001".to_string()],
        estimated_duration_ms: 1000,
    });

    checks.push(RegisteredCheck {
        id: "SEC-004".to_string(),
        name: "Encryption Status".to_string(),
        category: CheckCategory::Security,
        description: "Verify data encryption settings".to_string(),
        check_fn: Box::new(security::run_sec004),
        dependencies: vec![],
        estimated_duration_ms: 500,
    });

    checks.push(RegisteredCheck {
        id: "SEC-005".to_string(),
        name: "Instance Metadata Access".to_string(),
        category: CheckCategory::Security,
        description: "Verify metadata server access configuration".to_string(),
        check_fn: Box::new(security::run_sec005),
        dependencies: vec![],
        estimated_duration_ms: 1000,
    });

    checks.push(RegisteredCheck {
        id: "SEC-006".to_string(),
        name: "SSH Key Management".to_string(),
        category: CheckCategory::Security,
        description: "Check for OS Login vs legacy SSH keys".to_string(),
        check_fn: Box::new(security::run_sec006),
        dependencies: vec![],
        estimated_duration_ms: 1000,
    });

    checks.push(RegisteredCheck {
        id: "SEC-007".to_string(),
        name: "Firewall Rules".to_string(),
        category: CheckCategory::Security,
        description: "Provide guidance on firewall configuration".to_string(),
        check_fn: Box::new(security::run_sec007),
        dependencies: vec![],
        estimated_duration_ms: 100,
    });

    // Configuration checks
    checks.push(RegisteredCheck {
        id: "CFG-001".to_string(),
        name: "XLA Flags Audit".to_string(),
        category: CheckCategory::Config,
        description: "Check XLA_FLAGS for potential issues".to_string(),
        check_fn: Box::new(config::check_xla_flags),
        dependencies: vec![],
        estimated_duration_ms: 100,
    });

    checks.push(RegisteredCheck {
        id: "CFG-002".to_string(),
        name: "JAX Configuration Audit".to_string(),
        category: CheckCategory::Config,
        description: "Check JAX configuration values".to_string(),
        check_fn: Box::new(config::check_jax_config),
        dependencies: vec!["STK-001".to_string()],
        estimated_duration_ms: 100,
    });

    checks.push(RegisteredCheck {
        id: "CFG-003".to_string(),
        name: "Memory Preallocation Check".to_string(),
        category: CheckCategory::Config,
        description: "Check memory preallocation settings".to_string(),
        check_fn: Box::new(config::check_memory_config),
        dependencies: vec![],
        estimated_duration_ms: 100,
    });

    checks.push(RegisteredCheck {
        id: "CFG-004".to_string(),
        name: "Distributed Configuration Check".to_string(),
        category: CheckCategory::Config,
        description: "Check multi-host configuration".to_string(),
        check_fn: Box::new(config::check_distributed_config),
        dependencies: vec!["HW-001".to_string()],
        estimated_duration_ms: 100,
    });

    checks.push(RegisteredCheck {
        id: "CFG-005".to_string(),
        name: "Logging Configuration Check".to_string(),
        category: CheckCategory::Config,
        description: "Check logging and debug settings".to_string(),
        check_fn: Box::new(config::check_logging_config),
        dependencies: vec![],
        estimated_duration_ms: 100,
    });

    checks
}
