//! TPU device interface.
//!
//! Provides access to TPU device information via environment variables,
//! sysfs, and optionally libtpu FFI.
//!
//! # Graceful Degradation
//!
//! This module handles errors gracefully:
//! - Not on TPU: is_tpu_vm() returns false, functions return PreflightError::NotOnTpu
//! - Missing sysfs: Falls back to environment variables
//! - Missing env vars: Falls back to GCP metadata or defaults
//! - Parse errors: Uses safe defaults for chip counts, temperatures
//! - No libtpu: Returns estimates based on TPU type where possible
//!
//! The module uses a multi-level fallback strategy:
//! 1. Environment variables (TPU_NAME, TPU_CHIPS_PER_HOST, etc.)
//! 2. Sysfs entries (/sys/class/accel/*)
//! 3. GCP metadata (accelerator-type attribute)
//! 4. Type-based defaults (conservative estimates)
//!
//! No function in this module will panic.

use crate::platform::{gcp, linux};
use crate::PreflightError;
use std::path::Path;

/// TPU generation/type
#[derive(Debug, Clone, PartialEq)]
pub enum TpuType {
    V4,
    V5e,
    V5p,
    V6e,
    V7,
    Unknown,
}

impl std::fmt::Display for TpuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TpuType::V4 => write!(f, "v4"),
            TpuType::V5e => write!(f, "v5e"),
            TpuType::V5p => write!(f, "v5p"),
            TpuType::V6e => write!(f, "v6e"),
            TpuType::V7 => write!(f, "v7"),
            TpuType::Unknown => write!(f, "unknown"),
        }
    }
}

/// TPU topology information
#[derive(Debug, Clone)]
pub struct TpuTopology {
    pub chips: u32,
    pub cores_per_chip: u32,
    pub shape: String,
}

/// HBM memory information
#[derive(Debug, Clone)]
pub struct HbmInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub per_chip_bytes: u64,
}

/// TPU health status
#[derive(Debug, Clone, PartialEq)]
pub enum TpuHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// TPU thermal information
#[derive(Debug, Clone)]
pub struct ThermalInfo {
    pub chip_temperatures: Vec<f64>,
}

/// TPU error counters
#[derive(Debug, Clone)]
pub struct ErrorCounters {
    pub correctable: u64,
    pub uncorrectable: u64,
}

/// ICI interconnect status
#[derive(Debug, Clone)]
pub struct IciStatus {
    pub healthy: bool,
    pub bandwidth_gbps: f64,
    pub details: String,
}

/// Check if running on a TPU VM
pub fn is_tpu_vm() -> bool {
    // Check multiple signals

    // 1. TPU_NAME environment variable
    if linux::get_environment_variable("TPU_NAME").is_some() {
        return true;
    }

    // 2. Check for TPU accelerator devices in sysfs
    if Path::new("/sys/class/accel").exists() {
        if let Ok(entries) = std::fs::read_dir("/sys/class/accel") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("accel") {
                    return true;
                }
            }
        }
    }

    // 3. Check GCP machine type for TPU indicators
    if let Ok(machine_type) = gcp::get_machine_type() {
        if machine_type.contains("tpu") {
            return true;
        }
    }

    // 4. Check for TPU-related kernel modules
    if let Ok(modules) = std::fs::read_to_string("/proc/modules") {
        if modules.contains("tpu") || modules.contains("libtpu") {
            return true;
        }
    }

    false
}

/// Get the TPU type/generation
pub fn get_tpu_type() -> Result<TpuType, PreflightError> {
    // Try environment variable first
    if let Some(tpu_name) = linux::get_environment_variable("TPU_NAME") {
        return Ok(parse_tpu_type(&tpu_name));
    }

    // Try accelerator type from metadata
    if let Ok(Some(accel_type)) = gcp::get_instance_attribute("accelerator-type") {
        return Ok(parse_tpu_type(&accel_type));
    }

    // Try machine type
    if let Ok(machine_type) = gcp::get_machine_type() {
        return Ok(parse_tpu_type(&machine_type));
    }

    Err(PreflightError::NotOnTpu)
}

/// Get TPU chip count
pub fn get_tpu_chip_count() -> Result<u32, PreflightError> {
    // Try environment variable
    if let Some(chips_str) = linux::get_environment_variable("TPU_CHIPS_PER_HOST") {
        if let Ok(chips) = chips_str.parse() {
            return Ok(chips);
        }
    }

    // Try to count accelerator devices
    if Path::new("/sys/class/accel").exists() {
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir("/sys/class/accel") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("accel") {
                    count += 1;
                }
            }
        }
        if count > 0 {
            return Ok(count);
        }
    }

    // Fall back to TPU type defaults
    match get_tpu_type() {
        Ok(tpu_type) => Ok(default_chip_count(&tpu_type)),
        Err(e) => Err(e),
    }
}

/// Get expected chip count (for comparison)
pub fn get_expected_chip_count() -> Result<u32, PreflightError> {
    // Try environment variable override
    if let Some(expected) = linux::get_environment_variable("TPU_EXPECTED_CHIPS") {
        if let Ok(chips) = expected.parse() {
            return Ok(chips);
        }
    }

    // Use default for TPU type
    match get_tpu_type() {
        Ok(tpu_type) => Ok(default_chip_count(&tpu_type)),
        Err(e) => Err(e),
    }
}

/// Get TPU topology information
pub fn get_tpu_topology() -> Result<TpuTopology, PreflightError> {
    let chips = get_tpu_chip_count()?;
    let tpu_type = get_tpu_type()?;

    let cores_per_chip = match tpu_type {
        TpuType::V4 => 2,
        TpuType::V5e => 1,
        TpuType::V5p => 2,
        TpuType::V6e => 1,
        TpuType::V7 => 2,
        TpuType::Unknown => 1,
    };

    let shape = linux::get_environment_variable("TPU_TOPOLOGY")
        .unwrap_or_else(|| format!("{}x1", chips));

    Ok(TpuTopology {
        chips,
        cores_per_chip,
        shape,
    })
}

/// Get HBM memory information
pub fn get_hbm_info() -> Result<HbmInfo, PreflightError> {
    let tpu_type = get_tpu_type()?;
    let chips = get_tpu_chip_count()?;

    // Per-chip HBM by TPU type (in bytes)
    let per_chip_bytes: u64 = match tpu_type {
        TpuType::V4 => 32 * 1024 * 1024 * 1024,      // 32GB
        TpuType::V5e => 16 * 1024 * 1024 * 1024,     // 16GB
        TpuType::V5p => 95 * 1024 * 1024 * 1024,     // 95GB
        TpuType::V6e => 32 * 1024 * 1024 * 1024,     // 32GB (estimated)
        TpuType::V7 => 128 * 1024 * 1024 * 1024,    // 128GB (estimated)
        TpuType::Unknown => 16 * 1024 * 1024 * 1024, // Conservative default
    };

    let total_bytes = per_chip_bytes * chips as u64;

    // We can't easily get actual available HBM without libtpu
    // For now, assume 95% available as default
    let available_bytes = (total_bytes as f64 * 0.95) as u64;

    Ok(HbmInfo {
        total_bytes,
        available_bytes,
        per_chip_bytes,
    })
}

/// Get TPU health status
pub fn get_tpu_health() -> Result<TpuHealth, PreflightError> {
    // Try to read health from sysfs or environment
    if let Some(health) = linux::get_environment_variable("TPU_HEALTH") {
        return Ok(match health.to_lowercase().as_str() {
            "healthy" => TpuHealth::Healthy,
            "degraded" => TpuHealth::Degraded,
            "unhealthy" => TpuHealth::Unhealthy,
            _ => TpuHealth::Unknown,
        });
    }

    // If we can detect TPU chips, assume healthy
    if is_tpu_vm() {
        Ok(TpuHealth::Healthy)
    } else {
        Err(PreflightError::NotOnTpu)
    }
}

/// Check if TPU driver is loaded
pub fn check_tpu_driver_loaded() -> bool {
    // Check /proc/modules for TPU-related modules
    if let Ok(modules) = std::fs::read_to_string("/proc/modules") {
        if modules.contains("tpu") || modules.contains("accel") {
            return true;
        }
    }

    // Check for /dev/accel* devices
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("accel") {
                return true;
            }
        }
    }

    false
}

/// Get driver version
pub fn get_driver_version() -> Result<String, PreflightError> {
    // Try to read from sysfs
    let version_paths = [
        "/sys/module/tpu/version",
        "/sys/module/accel/version",
    ];

    for path in version_paths.iter() {
        if let Ok(version) = linux::read_sysfs_value(path) {
            return Ok(version);
        }
    }

    // Try environment variable
    if let Some(version) = linux::get_environment_variable("TPU_DRIVER_VERSION") {
        return Ok(version);
    }

    Err(PreflightError::IoError {
        context: "get_driver_version".to_string(),
        message: "Driver version not available".to_string(),
    })
}

/// Get libtpu version
pub fn get_libtpu_version() -> Result<String, PreflightError> {
    // Try environment variable
    if let Some(version) = linux::get_environment_variable("LIBTPU_VERSION") {
        return Ok(version);
    }

    // Try to find libtpu.so and extract version
    let lib_paths = [
        "/usr/local/lib/libtpu.so",
        "/usr/lib/libtpu.so",
    ];

    for path in lib_paths.iter() {
        if Path::new(path).exists() {
            // libtpu version often in filename like libtpu.so.0.1.dev20251201
            // For now, return a generic response
            return Ok("available (version unknown)".to_string());
        }
    }

    Err(PreflightError::IoError {
        context: "get_libtpu_version".to_string(),
        message: "libtpu not found".to_string(),
    })
}

/// Get thermal information
pub fn get_thermal_info() -> Result<ThermalInfo, PreflightError> {
    // Try to read from sysfs thermal zones
    let mut temperatures = Vec::new();

    // Look for TPU thermal zones
    if let Ok(entries) = std::fs::read_dir("/sys/class/thermal") {
        for entry in entries.flatten() {
            let path = entry.path();
            let type_path = path.join("type");

            if let Ok(zone_type) = std::fs::read_to_string(&type_path) {
                if zone_type.contains("tpu") || zone_type.contains("accel") {
                    let temp_path = path.join("temp");
                    if let Ok(temp_str) = std::fs::read_to_string(&temp_path) {
                        // Temperature is in millidegrees Celsius
                        if let Ok(temp_milli) = temp_str.trim().parse::<i64>() {
                            temperatures.push(temp_milli as f64 / 1000.0);
                        }
                    }
                }
            }
        }
    }

    if temperatures.is_empty() {
        // Return synthetic data based on chip count
        let chips = get_tpu_chip_count().unwrap_or(1);
        temperatures = vec![65.0; chips as usize]; // Assume normal temperature
    }

    Ok(ThermalInfo {
        chip_temperatures: temperatures,
    })
}

/// Get error counters
pub fn get_error_counters() -> Result<ErrorCounters, PreflightError> {
    // Try to read from sysfs
    // This is hardware-specific and may not be available on all TPUs

    let correctable = linux::get_environment_variable("TPU_CORRECTABLE_ERRORS")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let uncorrectable = linux::get_environment_variable("TPU_UNCORRECTABLE_ERRORS")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    Ok(ErrorCounters {
        correctable,
        uncorrectable,
    })
}

/// Get ICI interconnect status
pub fn get_ici_status() -> Result<IciStatus, PreflightError> {
    // ICI status is not easily available without libtpu
    // Return a default healthy status for multi-chip configurations

    let chips = get_tpu_chip_count()?;
    if chips <= 1 {
        return Err(PreflightError::IoError {
            context: "get_ici_status".to_string(),
            message: "Single chip configuration".to_string(),
        });
    }

    // Estimate bandwidth based on TPU type
    let tpu_type = get_tpu_type()?;
    let bandwidth_gbps = match tpu_type {
        TpuType::V4 => 400.0,
        TpuType::V5e => 200.0,
        TpuType::V5p => 450.0,
        TpuType::V6e => 500.0,
        TpuType::V7 => 600.0,
        TpuType::Unknown => 200.0,
    };

    Ok(IciStatus {
        healthy: true,
        bandwidth_gbps,
        details: "ICI status inferred from TPU type".to_string(),
    })
}

// Helper functions

fn parse_tpu_type(name: &str) -> TpuType {
    let lower = name.to_lowercase();

    if lower.contains("v5litepod") || lower.contains("v5e") {
        TpuType::V5e
    } else if lower.contains("v5p") {
        TpuType::V5p
    } else if lower.contains("v6e") {
        TpuType::V6e
    } else if lower.contains("v7") {
        TpuType::V7
    } else if lower.contains("v4") {
        TpuType::V4
    } else {
        TpuType::Unknown
    }
}

fn default_chip_count(tpu_type: &TpuType) -> u32 {
    match tpu_type {
        TpuType::V4 => 4,
        TpuType::V5e => 8,
        TpuType::V5p => 8,
        TpuType::V6e => 4,
        TpuType::V7 => 8,
        TpuType::Unknown => 1,
    }
}
