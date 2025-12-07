//! Hardware validation checks.
//!
//! Checks for TPU device detection, memory, thermal status, error counters,
//! interconnect status, and driver status.

use crate::platform::tpu::{self};
use crate::{Check, CheckCategory, CheckResult};
use std::time::Instant;

/// Get all hardware checks
pub fn get_hardware_checks() -> Vec<Check> {
    vec![
        create_hw001_check(),
        create_hw002_check(),
        create_hw003_check(),
        create_hw004_check(),
        create_hw005_check(),
        create_hw006_check(),
    ]
}

/// HW-001: TPU Device Detection
fn create_hw001_check() -> Check {
    Check {
        id: "HW-001".to_string(),
        name: "TPU Device Detection".to_string(),
        category: CheckCategory::Hardware,
        description: "Verify expected number of TPU chips are present".to_string(),
        result: None,
    }
}

/// HW-002: HBM Memory Availability
fn create_hw002_check() -> Check {
    Check {
        id: "HW-002".to_string(),
        name: "HBM Memory Availability".to_string(),
        category: CheckCategory::Hardware,
        description: "Check total HBM capacity and availability".to_string(),
        result: None,
    }
}

/// HW-003: TPU Thermal Status
fn create_hw003_check() -> Check {
    Check {
        id: "HW-003".to_string(),
        name: "TPU Thermal Status".to_string(),
        category: CheckCategory::Hardware,
        description: "Check temperature of each TPU chip".to_string(),
        result: None,
    }
}

/// HW-004: TPU Error Counters
fn create_hw004_check() -> Check {
    Check {
        id: "HW-004".to_string(),
        name: "TPU Error Counters".to_string(),
        category: CheckCategory::Hardware,
        description: "Check for accumulated hardware errors".to_string(),
        result: None,
    }
}

/// HW-005: ICI Interconnect Status
fn create_hw005_check() -> Check {
    Check {
        id: "HW-005".to_string(),
        name: "ICI Interconnect Status".to_string(),
        category: CheckCategory::Hardware,
        description: "Verify inter-chip interconnect is functional".to_string(),
        result: None,
    }
}

/// HW-006: Driver Status
fn create_hw006_check() -> Check {
    Check {
        id: "HW-006".to_string(),
        name: "Driver Status".to_string(),
        category: CheckCategory::Hardware,
        description: "Verify TPU driver kernel module is loaded".to_string(),
        result: None,
    }
}

/// Execute HW-001: TPU Device Detection
pub fn run_hw001() -> CheckResult {
    let start = Instant::now();

    // Check if we're on a TPU VM
    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    // Get chip count
    match tpu::get_tpu_chip_count() {
        Ok(count) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            // Get expected chip count from environment or TPU type
            let expected = tpu::get_expected_chip_count().unwrap_or(count);

            if count == 0 {
                CheckResult::Fail {
                    message: "No TPU chips detected".to_string(),
                    details: "Expected at least one TPU chip but found none".to_string(),
                    duration_ms,
                }
            } else if count < expected {
                CheckResult::Fail {
                    message: format!("Fewer TPU chips than expected: {} found, {} expected", count, expected),
                    details: "Some TPU chips may be offline or malfunctioning".to_string(),
                    duration_ms,
                }
            } else if count > expected {
                CheckResult::Warn {
                    message: format!("More TPU chips than expected: {} found, {} expected", count, expected),
                    details: "This is unusual but not necessarily an error".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("{} chips detected", count),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Fail {
            message: "Failed to detect TPU chips".to_string(),
            details: e.to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        },
    }
}

/// Execute HW-002: HBM Memory Availability
pub fn run_hw002() -> CheckResult {
    let start = Instant::now();

    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    match tpu::get_hbm_info() {
        Ok(hbm) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let availability_pct = if hbm.total_bytes > 0 {
                (hbm.available_bytes as f64 / hbm.total_bytes as f64) * 100.0
            } else {
                0.0
            };

            let total_gb = hbm.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let available_gb = hbm.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            if availability_pct < 50.0 {
                CheckResult::Fail {
                    message: format!("HBM availability critically low: {:.1}%", availability_pct),
                    details: format!("{:.1}GB available of {:.1}GB total", available_gb, total_gb),
                    duration_ms,
                }
            } else if availability_pct < 90.0 {
                CheckResult::Warn {
                    message: format!("HBM availability below threshold: {:.1}%", availability_pct),
                    details: format!("{:.1}GB available of {:.1}GB total", available_gb, total_gb),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("{:.1}GB available ({:.1}%)", available_gb, availability_pct),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("HBM info unavailable: {}", e),
        },
    }
}

/// Execute HW-003: TPU Thermal Status
pub fn run_hw003() -> CheckResult {
    let start = Instant::now();

    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    match tpu::get_thermal_info() {
        Ok(thermal) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let max_temp = thermal.chip_temperatures.iter().cloned().fold(0.0f64, f64::max);

            if max_temp >= 85.0 {
                CheckResult::Fail {
                    message: format!("TPU temperature critical: {:.1}C", max_temp),
                    details: "One or more chips above 85C threshold".to_string(),
                    duration_ms,
                }
            } else if max_temp >= 75.0 {
                CheckResult::Warn {
                    message: format!("TPU temperature elevated: {:.1}C", max_temp),
                    details: "One or more chips above 75C warning threshold".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("Max temperature: {:.1}C", max_temp),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("Thermal info unavailable: {}", e),
        },
    }
}

/// Execute HW-004: TPU Error Counters
pub fn run_hw004() -> CheckResult {
    let start = Instant::now();

    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    match tpu::get_error_counters() {
        Ok(errors) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            if errors.uncorrectable > 0 {
                CheckResult::Fail {
                    message: format!("{} uncorrectable errors detected", errors.uncorrectable),
                    details: "Uncorrectable errors indicate hardware issues".to_string(),
                    duration_ms,
                }
            } else if errors.correctable > 0 {
                CheckResult::Warn {
                    message: format!("{} correctable errors detected", errors.correctable),
                    details: "Correctable errors are handled but may indicate degradation".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: "No hardware errors".to_string(),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("Error counters unavailable: {}", e),
        },
    }
}

/// Execute HW-005: ICI Interconnect Status
pub fn run_hw005() -> CheckResult {
    let start = Instant::now();

    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    // Skip for single-chip configurations
    match tpu::get_tpu_chip_count() {
        Ok(count) if count <= 1 => {
            return CheckResult::Skip {
                reason: "Single-chip configuration - ICI not applicable".to_string(),
            };
        }
        Err(e) => {
            return CheckResult::Skip {
                reason: format!("Could not determine chip count: {}", e),
            };
        }
        _ => {}
    }

    match tpu::get_ici_status() {
        Ok(status) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            if !status.healthy {
                CheckResult::Fail {
                    message: "ICI interconnect errors detected".to_string(),
                    details: status.details,
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("ICI healthy, bandwidth: {:.1} GB/s", status.bandwidth_gbps),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("ICI status unavailable: {}", e),
        },
    }
}

/// Execute HW-006: Driver Status
pub fn run_hw006() -> CheckResult {
    let start = Instant::now();

    let driver_loaded = tpu::check_tpu_driver_loaded();
    let duration_ms = start.elapsed().as_millis() as u64;

    if !driver_loaded {
        return CheckResult::Fail {
            message: "TPU driver not loaded".to_string(),
            details: "The TPU kernel module is not loaded".to_string(),
            duration_ms,
        };
    }

    match tpu::get_driver_version() {
        Ok(version) => {
            // Check version compatibility (simplified)
            CheckResult::Pass {
                message: format!("Driver version: {}", version),
                duration_ms,
            }
        }
        Err(e) => CheckResult::Warn {
            message: "Driver loaded but version unknown".to_string(),
            details: e.to_string(),
            duration_ms,
        },
    }
}
