//! Linux system interface.
//!
//! Provides access to system information via /proc, /sys, and syscalls.
//!
//! # Graceful Degradation
//!
//! This module handles errors gracefully:
//! - File not found: Returns appropriate TpuDocError::IoError
//! - Permission denied: Returns TpuDocError::IoError with context
//! - Parse errors: Returns TpuDocError::ParseError with details
//! - Missing data: Uses defaults (0, empty string) where safe
//! - Command failures: Returns error with command context
//!
//! No function in this module will panic. All errors are propagated
//! via Result types for the caller to handle.

use crate::TpuDocError;
use std::fs;
use std::path::Path;

/// Memory information from /proc/meminfo
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub free_bytes: u64,
}

/// CPU information from /proc/cpuinfo
#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub model_name: String,
    pub cores: u32,
    pub frequency_mhz: f64,
}

/// Disk space information
#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub free_bytes: u64,
}

/// Get the system hostname
pub fn get_hostname() -> Result<String, TpuDocError> {
    // Try /etc/hostname first
    if let Ok(hostname) = fs::read_to_string("/etc/hostname") {
        let hostname = hostname.trim().to_string();
        if !hostname.is_empty() {
            return Ok(hostname);
        }
    }

    // Fall back to /proc/sys/kernel/hostname
    if let Ok(hostname) = fs::read_to_string("/proc/sys/kernel/hostname") {
        let hostname = hostname.trim().to_string();
        if !hostname.is_empty() {
            return Ok(hostname);
        }
    }

    Err(TpuDocError::IoError {
        context: "get_hostname".to_string(),
        message: "Could not read hostname from /etc/hostname or /proc".to_string(),
    })
}

/// Get kernel version from /proc/version
pub fn get_kernel_version() -> Result<String, TpuDocError> {
    let content = fs::read_to_string("/proc/version").map_err(|e| TpuDocError::IoError {
        context: "get_kernel_version".to_string(),
        message: e.to_string(),
    })?;

    // Parse "Linux version X.Y.Z ..."
    if let Some(version) = content.split_whitespace().nth(2) {
        Ok(version.to_string())
    } else {
        Err(TpuDocError::ParseError {
            context: "get_kernel_version".to_string(),
            message: "Could not parse kernel version".to_string(),
        })
    }
}

/// Get memory information from /proc/meminfo
pub fn get_memory_info() -> Result<MemoryInfo, TpuDocError> {
    let content = fs::read_to_string("/proc/meminfo").map_err(|e| TpuDocError::IoError {
        context: "get_memory_info".to_string(),
        message: e.to_string(),
    })?;

    let mut total = 0u64;
    let mut available = 0u64;
    let mut free = 0u64;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let value: u64 = parts[1].parse().unwrap_or(0);
            // Values in /proc/meminfo are in kB
            let bytes = value * 1024;

            match parts[0] {
                "MemTotal:" => total = bytes,
                "MemAvailable:" => available = bytes,
                "MemFree:" => free = bytes,
                _ => {}
            }
        }
    }

    Ok(MemoryInfo {
        total_bytes: total,
        available_bytes: available,
        free_bytes: free,
    })
}

/// Get CPU information from /proc/cpuinfo
pub fn get_cpu_info() -> Result<CpuInfo, TpuDocError> {
    let content = fs::read_to_string("/proc/cpuinfo").map_err(|e| TpuDocError::IoError {
        context: "get_cpu_info".to_string(),
        message: e.to_string(),
    })?;

    let mut model_name = String::new();
    let mut frequency_mhz = 0.0f64;
    let mut core_count = 0u32;

    for line in content.lines() {
        if line.starts_with("model name") {
            if let Some(value) = line.split(':').nth(1) {
                model_name = value.trim().to_string();
            }
        } else if line.starts_with("cpu MHz") {
            if let Some(value) = line.split(':').nth(1) {
                frequency_mhz = value.trim().parse().unwrap_or(0.0);
            }
        } else if line.starts_with("processor") {
            core_count += 1;
        }
    }

    Ok(CpuInfo {
        model_name,
        cores: core_count,
        frequency_mhz,
    })
}

/// Get disk space information for a path
pub fn get_disk_space(path: &str) -> Result<DiskInfo, TpuDocError> {
    // Use statvfs via std::fs::metadata and a platform-specific approach
    // For simplicity, we'll use the df command
    let output = std::process::Command::new("df")
        .args(["-B1", path])
        .output()
        .map_err(|e| TpuDocError::IoError {
            context: "get_disk_space".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(TpuDocError::IoError {
            context: "get_disk_space".to_string(),
            message: "df command failed".to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.len() < 2 {
        return Err(TpuDocError::ParseError {
            context: "get_disk_space".to_string(),
            message: "Could not parse df output".to_string(),
        });
    }

    // Parse second line: Filesystem 1B-blocks Used Available Use% Mounted
    let parts: Vec<&str> = lines[1].split_whitespace().collect();
    if parts.len() >= 4 {
        let total: u64 = parts[1].parse().unwrap_or(0);
        let used: u64 = parts[2].parse().unwrap_or(0);
        let available: u64 = parts[3].parse().unwrap_or(0);

        Ok(DiskInfo {
            total_bytes: total,
            available_bytes: available,
            free_bytes: total.saturating_sub(used),
        })
    } else {
        Err(TpuDocError::ParseError {
            context: "get_disk_space".to_string(),
            message: "Could not parse df output".to_string(),
        })
    }
}

/// Read a value from sysfs
pub fn read_sysfs_value(path: &str) -> Result<String, TpuDocError> {
    fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .map_err(|e| TpuDocError::IoError {
            context: format!("read_sysfs_value({})", path),
            message: e.to_string(),
        })
}

/// Check if a process is running by name
pub fn check_process_running(name: &str) -> Result<bool, TpuDocError> {
    let proc_dir = Path::new("/proc");

    if !proc_dir.exists() {
        return Err(TpuDocError::IoError {
            context: "check_process_running".to_string(),
            message: "/proc does not exist".to_string(),
        });
    }

    for entry in fs::read_dir(proc_dir).map_err(|e| TpuDocError::IoError {
        context: "check_process_running".to_string(),
        message: e.to_string(),
    })? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Check if this is a PID directory
        if let Some(filename) = path.file_name() {
            if filename.to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
                // Read comm file for process name
                let comm_path = path.join("comm");
                if let Ok(comm) = fs::read_to_string(&comm_path) {
                    if comm.trim() == name {
                        return Ok(true);
                    }
                }
            }
        }
    }

    Ok(false)
}

/// Get an environment variable safely
pub fn get_environment_variable(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Get current Unix timestamp
pub fn get_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
