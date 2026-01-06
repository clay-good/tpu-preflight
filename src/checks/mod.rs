//! Validation check modules.
//!
//! This module contains all validation checks organized by category:
//! - Hardware: TPU device health checks
//! - Stack: Software version compatibility checks
//! - Performance: Performance baseline checks
//! - I/O: Storage and network throughput checks
//! - Security: Security posture checks
//! - Config: Configuration audit checks
//!
//! # Graceful Degradation
//!
//! All checks follow these degradation rules:
//! - Not on TPU VM: Return CheckResult::Skip (not Fail)
//! - Data unavailable: Return CheckResult::Skip with reason
//! - Operation timeout: Return CheckResult::Fail with timeout message
//! - Parse errors: Return CheckResult::Warn or Skip depending on severity
//! - Partial data: Use available data, note limitations in message
//!
//! Checks never panic. All error conditions are converted to appropriate
//! CheckResult variants for the caller to handle.

pub mod config;
pub mod hardware;
pub mod io;
pub mod performance;
pub mod security;
pub mod stack;

use crate::{Check, CheckCategory};

/// Get all registered checks
pub fn get_all_checks() -> Vec<Check> {
    let mut checks = Vec::new();
    checks.extend(hardware::get_hardware_checks());
    checks.extend(stack::get_stack_checks());
    checks.extend(performance::get_performance_checks());
    checks.extend(io::get_io_checks());
    checks.extend(security::get_security_checks());
    checks.extend(config::get_config_checks());
    checks
}

/// Get checks for a specific category
pub fn get_checks_by_category(category: CheckCategory) -> Vec<Check> {
    match category {
        CheckCategory::Hardware => hardware::get_hardware_checks(),
        CheckCategory::Stack => stack::get_stack_checks(),
        CheckCategory::Performance => performance::get_performance_checks(),
        CheckCategory::Io => io::get_io_checks(),
        CheckCategory::Security => security::get_security_checks(),
        CheckCategory::Config => config::get_config_checks(),
    }
}
