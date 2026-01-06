//! Configuration audit checks (CFG-001 through CFG-005).
//!
//! Checks XLA, JAX, and system configuration for potential issues.

use crate::{Check, CheckCategory, CheckResult};
use std::env;
use std::time::Instant;

/// Get all configuration audit checks
pub fn get_config_checks() -> Vec<Check> {
    vec![
        Check {
            id: "CFG-001".to_string(),
            name: "XLA Flags Audit".to_string(),
            category: CheckCategory::Config,
            description: "Check XLA_FLAGS for potential issues".to_string(),
            result: None,
        },
        Check {
            id: "CFG-002".to_string(),
            name: "JAX Configuration Audit".to_string(),
            category: CheckCategory::Config,
            description: "Check JAX configuration values".to_string(),
            result: None,
        },
        Check {
            id: "CFG-003".to_string(),
            name: "Memory Preallocation Check".to_string(),
            category: CheckCategory::Config,
            description: "Check memory preallocation settings".to_string(),
            result: None,
        },
        Check {
            id: "CFG-004".to_string(),
            name: "Distributed Configuration Check".to_string(),
            category: CheckCategory::Config,
            description: "Check multi-host configuration".to_string(),
            result: None,
        },
        Check {
            id: "CFG-005".to_string(),
            name: "Logging Configuration Check".to_string(),
            category: CheckCategory::Config,
            description: "Check logging and debug settings".to_string(),
            result: None,
        },
    ]
}

/// Run CFG-001: XLA Flags Audit
pub fn check_xla_flags() -> CheckResult {
    let start = Instant::now();

    match env::var("XLA_FLAGS") {
        Ok(flags) => {
            let mut issues = Vec::new();

            // Check for debug flags
            let debug_patterns = [
                "--xla_dump_to",
                "--xla_dump_hlo",
                "--xla_log_all",
            ];

            for pattern in &debug_patterns {
                if flags.contains(pattern) {
                    issues.push(format!("Debug flag {} is set", pattern));
                }
            }

            // Check for disabled optimizations
            if flags.contains("--xla_disable_hlo_passes") {
                issues.push("HLO passes are disabled".to_string());
            }

            let duration_ms = start.elapsed().as_millis() as u64;

            if !issues.is_empty() {
                CheckResult::Warn {
                    message: format!("XLA_FLAGS has {} potential issues", issues.len()),
                    details: issues.join("; "),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: "XLA_FLAGS configuration is optimal".to_string(),
                    duration_ms,
                }
            }
        }
        Err(_) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            CheckResult::Pass {
                message: "XLA_FLAGS not set (using defaults)".to_string(),
                duration_ms,
            }
        }
    }
}

/// Run CFG-002: JAX Configuration Audit
pub fn check_jax_config() -> CheckResult {
    let start = Instant::now();
    let mut issues = Vec::new();

    // Check JAX_PLATFORMS
    if let Ok(platforms) = env::var("JAX_PLATFORMS") {
        if !platforms.contains("tpu") {
            issues.push("JAX_PLATFORMS does not include 'tpu'".to_string());
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    if !issues.is_empty() {
        CheckResult::Warn {
            message: "JAX configuration has potential issues".to_string(),
            details: issues.join("; "),
            duration_ms,
        }
    } else {
        CheckResult::Pass {
            message: "JAX configuration appears correct".to_string(),
            duration_ms,
        }
    }
}

/// Run CFG-003: Memory Preallocation Check
pub fn check_memory_config() -> CheckResult {
    let start = Instant::now();
    let mut issues = Vec::new();

    // Check memory fraction
    if let Ok(fraction) = env::var("XLA_PYTHON_CLIENT_MEM_FRACTION") {
        if let Ok(f) = fraction.parse::<f64>() {
            if f > 0.95 {
                issues.push(format!("High memory fraction: {} (risk of OOM)", f));
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    if !issues.is_empty() {
        CheckResult::Warn {
            message: "Memory configuration may cause issues".to_string(),
            details: issues.join("; "),
            duration_ms,
        }
    } else {
        CheckResult::Pass {
            message: "Memory configuration is appropriate".to_string(),
            duration_ms,
        }
    }
}

/// Run CFG-004: Distributed Configuration Check
pub fn check_distributed_config() -> CheckResult {
    let start = Instant::now();

    // Check if multi-host
    let coordinator = env::var("JAX_COORDINATOR_ADDRESS").ok();
    let worker_hostnames = env::var("TPU_WORKER_HOSTNAMES").ok();

    let is_multi_host = coordinator.is_some() ||
        worker_hostnames.as_ref().map(|h| h.contains(',')).unwrap_or(false);

    let duration_ms = start.elapsed().as_millis() as u64;

    if is_multi_host {
        if coordinator.is_none() {
            CheckResult::Fail {
                message: "Multi-host detected but JAX_COORDINATOR_ADDRESS not set".to_string(),
                details: "Set JAX_COORDINATOR_ADDRESS for distributed training".to_string(),
                duration_ms,
            }
        } else {
            CheckResult::Pass {
                message: "Distributed configuration is correct".to_string(),
                duration_ms,
            }
        }
    } else {
        CheckResult::Skip {
            reason: "Single-host configuration".to_string(),
        }
    }
}

/// Run CFG-005: Logging Configuration Check
pub fn check_logging_config() -> CheckResult {
    let start = Instant::now();
    let mut issues = Vec::new();

    // Check TensorFlow log level
    if let Ok(level) = env::var("TF_CPP_MIN_LOG_LEVEL") {
        if level == "0" {
            issues.push("TF_CPP_MIN_LOG_LEVEL=0 (verbose logging)".to_string());
        }
    }

    // Check JAX debug NaNs
    if let Ok(debug_nans) = env::var("JAX_DEBUG_NANS") {
        if debug_nans == "True" || debug_nans == "1" {
            issues.push("JAX_DEBUG_NANS is enabled (performance impact)".to_string());
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    if !issues.is_empty() {
        CheckResult::Warn {
            message: "Debug logging may impact performance".to_string(),
            details: issues.join("; "),
            duration_ms,
        }
    } else {
        CheckResult::Pass {
            message: "Logging configuration is production-appropriate".to_string(),
            duration_ms,
        }
    }
}
