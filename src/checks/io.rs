//! I/O throughput validation checks.
//!
//! Checks for GCS read throughput, local disk throughput, GCS connectivity,
//! checkpoint directory access, network latency, and DNS resolution.

use crate::platform::{gcp, linux, network};
use crate::{Check, CheckCategory, CheckResult};
use std::time::Instant;

/// Get all I/O checks
pub fn get_io_checks() -> Vec<Check> {
    vec![
        create_io001_check(),
        create_io002_check(),
        create_io003_check(),
        create_io004_check(),
        create_io005_check(),
        create_io006_check(),
    ]
}

/// IO-001: GCS Read Throughput
fn create_io001_check() -> Check {
    Check {
        id: "IO-001".to_string(),
        name: "GCS Read Throughput".to_string(),
        category: CheckCategory::Io,
        description: "Measure read throughput from Google Cloud Storage".to_string(),
        result: None,
    }
}

/// IO-002: Local Disk Throughput
fn create_io002_check() -> Check {
    Check {
        id: "IO-002".to_string(),
        name: "Local Disk Throughput".to_string(),
        category: CheckCategory::Io,
        description: "Measure sequential read/write to local SSD".to_string(),
        result: None,
    }
}

/// IO-003: GCS Connectivity
fn create_io003_check() -> Check {
    Check {
        id: "IO-003".to_string(),
        name: "GCS Connectivity".to_string(),
        category: CheckCategory::Io,
        description: "Verify connectivity to storage.googleapis.com".to_string(),
        result: None,
    }
}

/// IO-004: Checkpoint Directory Access
fn create_io004_check() -> Check {
    Check {
        id: "IO-004".to_string(),
        name: "Checkpoint Directory Access".to_string(),
        category: CheckCategory::Io,
        description: "Verify checkpoint directory access and space".to_string(),
        result: None,
    }
}

/// IO-005: Network Latency to GCP Services
fn create_io005_check() -> Check {
    Check {
        id: "IO-005".to_string(),
        name: "Network Latency to GCP Services".to_string(),
        category: CheckCategory::Io,
        description: "Measure latency to GCP services".to_string(),
        result: None,
    }
}

/// IO-006: DNS Resolution
fn create_io006_check() -> Check {
    Check {
        id: "IO-006".to_string(),
        name: "DNS Resolution".to_string(),
        category: CheckCategory::Io,
        description: "Verify DNS resolution is working".to_string(),
        result: None,
    }
}

/// Execute IO-001: GCS Read Throughput
pub fn run_io001() -> CheckResult {
    let _start = Instant::now();

    // Check if gsutil is available
    match std::process::Command::new("which").arg("gsutil").output() {
        Ok(output) if output.status.success() => {}
        _ => {
            return CheckResult::Skip {
                reason: "gsutil not available".to_string(),
            };
        }
    }

    // Check GCS connectivity first
    if !gcp::is_on_gcp() {
        return CheckResult::Skip {
            reason: "Not running on GCP".to_string(),
        };
    }

    // In a full implementation, we would:
    // 1. Download a test file from a known GCS location
    // 2. Measure throughput
    // For now, skip since we don't have a test bucket configured
    CheckResult::Skip {
        reason: "GCS throughput test requires configured test bucket".to_string(),
    }
}

/// Execute IO-002: Local Disk Throughput
pub fn run_io002() -> CheckResult {
    let start = Instant::now();

    // Test write throughput using dd
    let test_file = "/tmp/tpu-preflight-disk-test";
    let block_size = "1M";
    let count = "100"; // 100MB test

    // Write test
    let write_result = std::process::Command::new("dd")
        .args([
            "if=/dev/zero",
            &format!("of={}", test_file),
            &format!("bs={}", block_size),
            &format!("count={}", count),
            "conv=fdatasync",
        ])
        .output();

    // Clean up test file
    let _ = std::fs::remove_file(test_file);

    let duration_ms = start.elapsed().as_millis() as u64;

    match write_result {
        Ok(output) => {
            // Parse dd output for throughput
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Look for throughput in format "XXX MB/s" or "XXX GB/s"
            let throughput_gbps = parse_dd_throughput(&stderr);

            match throughput_gbps {
                Some(throughput) => {
                    if throughput < 0.5 {
                        CheckResult::Warn {
                            message: format!("Local disk throughput low: {:.2} GB/s", throughput),
                            details: "Expected at least 1 GB/s for NVMe SSD".to_string(),
                            duration_ms,
                        }
                    } else {
                        CheckResult::Pass {
                            message: format!("Local disk throughput: {:.2} GB/s", throughput),
                            duration_ms,
                        }
                    }
                }
                None => CheckResult::Warn {
                    message: "Could not measure disk throughput".to_string(),
                    details: "dd output parsing failed".to_string(),
                    duration_ms,
                },
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("Disk throughput test failed: {}", e),
        },
    }
}

/// Execute IO-003: GCS Connectivity
pub fn run_io003() -> CheckResult {
    let start = Instant::now();

    match network::check_tcp_connectivity("storage.googleapis.com", 443, 5000) {
        Ok(result) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            if result.success {
                CheckResult::Pass {
                    message: format!("GCS connectivity OK, latency: {}ms", result.latency_ms),
                    duration_ms,
                }
            } else {
                CheckResult::Fail {
                    message: "Cannot connect to storage.googleapis.com".to_string(),
                    details: "TCP connection to port 443 failed".to_string(),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Fail {
            message: "GCS connectivity check failed".to_string(),
            details: e.to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        },
    }
}

/// Execute IO-004: Checkpoint Directory Access
pub fn run_io004() -> CheckResult {
    let start = Instant::now();

    // Check if CHECKPOINT_DIR is set
    let checkpoint_dir = match linux::get_environment_variable("CHECKPOINT_DIR") {
        Some(dir) => dir,
        None => {
            return CheckResult::Skip {
                reason: "CHECKPOINT_DIR environment variable not set".to_string(),
            };
        }
    };

    // Check if directory exists
    let path = std::path::Path::new(&checkpoint_dir);

    if !path.exists() {
        // Try to create it
        if std::fs::create_dir_all(path).is_err() {
            return CheckResult::Fail {
                message: "Cannot create checkpoint directory".to_string(),
                details: format!("Path: {}", checkpoint_dir),
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
    }

    // Check write permission
    let test_file = path.join(".tpu-preflight-test");
    let can_write = std::fs::write(&test_file, "test").is_ok();
    let _ = std::fs::remove_file(&test_file);

    if !can_write {
        return CheckResult::Fail {
            message: "No write permission for checkpoint directory".to_string(),
            details: format!("Path: {}", checkpoint_dir),
            duration_ms: start.elapsed().as_millis() as u64,
        };
    }

    // Check available space
    match linux::get_disk_space(&checkpoint_dir) {
        Ok(disk_info) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let available_gb = disk_info.available_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            if available_gb < 100.0 {
                CheckResult::Warn {
                    message: format!("Checkpoint directory space low: {:.1} GB available", available_gb),
                    details: "Recommended at least 100GB for checkpoints".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("Checkpoint directory OK, {:.1} GB available", available_gb),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Warn {
            message: "Could not check checkpoint directory space".to_string(),
            details: e.to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
        },
    }
}

/// Execute IO-005: Network Latency to GCP Services
pub fn run_io005() -> CheckResult {
    let start = Instant::now();

    let services = [
        ("metadata.google.internal", 80),
        ("storage.googleapis.com", 443),
        ("compute.googleapis.com", 443),
    ];

    let mut latencies = Vec::new();
    let mut failures = Vec::new();

    for (host, port) in services.iter() {
        match network::check_tcp_connectivity(host, *port, 5000) {
            Ok(result) if result.success => {
                latencies.push((host.to_string(), result.latency_ms));
            }
            Ok(_) => {
                failures.push(format!("{}:{} - connection failed", host, port));
            }
            Err(e) => {
                failures.push(format!("{}:{} - {}", host, port, e));
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    if !failures.is_empty() {
        return CheckResult::Warn {
            message: format!("{} service(s) unreachable", failures.len()),
            details: failures.join("; "),
            duration_ms,
        };
    }

    let max_latency = latencies.iter().map(|(_, l)| *l).max().unwrap_or(0);

    if max_latency > 10 {
        CheckResult::Warn {
            message: format!("Network latency elevated: max {}ms", max_latency),
            details: latencies
                .iter()
                .map(|(h, l)| format!("{}: {}ms", h, l))
                .collect::<Vec<_>>()
                .join(", "),
            duration_ms,
        }
    } else {
        CheckResult::Pass {
            message: format!("Network latency OK, max {}ms", max_latency),
            duration_ms,
        }
    }
}

/// Execute IO-006: DNS Resolution
pub fn run_io006() -> CheckResult {
    let start = Instant::now();

    let hostnames = [
        "storage.googleapis.com",
        "metadata.google.internal",
        "compute.googleapis.com",
    ];

    let mut failures = Vec::new();
    let mut slowest = 0u64;

    for hostname in hostnames.iter() {
        match network::check_dns_resolution(hostname) {
            Ok(result) => {
                if result.resolution_time_ms > slowest {
                    slowest = result.resolution_time_ms;
                }
            }
            Err(e) => {
                failures.push(format!("{}: {}", hostname, e));
            }
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    if !failures.is_empty() {
        CheckResult::Fail {
            message: "DNS resolution failed".to_string(),
            details: failures.join("; "),
            duration_ms,
        }
    } else {
        CheckResult::Pass {
            message: format!("DNS resolution OK, max {}ms", slowest),
            duration_ms,
        }
    }
}

// Helper functions

fn parse_dd_throughput(stderr: &str) -> Option<f64> {
    // Parse various dd output formats:
    // Linux: "104857600 bytes (105 MB, 100 MiB) copied, 0.0831556 s, 1.3 GB/s"
    // macOS: "104857600 bytes transferred in 0.083 secs (1262765060 bytes/sec)"
    // Alternative: "100+0 records in\n100+0 records out\n..."

    for line in stderr.lines() {
        // Try GB/s format first
        if let Some(gbps) = extract_rate(line, "GB/s", 1.0) {
            return Some(gbps);
        }
        if let Some(gbps) = extract_rate(line, "G/s", 1.0) {
            return Some(gbps);
        }

        // Try MB/s format
        if let Some(gbps) = extract_rate(line, "MB/s", 1.0 / 1024.0) {
            return Some(gbps);
        }
        if let Some(gbps) = extract_rate(line, "M/s", 1.0 / 1024.0) {
            return Some(gbps);
        }

        // Try kB/s format
        if let Some(gbps) = extract_rate(line, "kB/s", 1.0 / (1024.0 * 1024.0)) {
            return Some(gbps);
        }

        // Try bytes/sec format (macOS)
        if line.contains("bytes/sec") || line.contains("bytes/s") {
            // Look for pattern like "(1262765060 bytes/sec)"
            if let Some(start) = line.find('(') {
                if let Some(end) = line[start..].find("bytes") {
                    let num_str = line[start + 1..start + end].trim();
                    if let Ok(bytes_per_sec) = num_str.parse::<f64>() {
                        return Some(bytes_per_sec / (1024.0 * 1024.0 * 1024.0));
                    }
                }
            }
        }

        // Try to calculate from bytes and time if present
        // Format: "104857600 bytes ... copied, 0.0831556 s"
        if line.contains("bytes") && (line.contains("copied") || line.contains("transferred")) {
            if let Some(gbps) = calculate_throughput_from_line(line) {
                return Some(gbps);
            }
        }
    }

    None
}

fn extract_rate(line: &str, unit: &str, to_gbps: f64) -> Option<f64> {
    if !line.contains(unit) {
        return None;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == unit && i > 0 {
            let num_str = parts[i - 1].replace(',', ".");
            if let Ok(value) = num_str.parse::<f64>() {
                return Some(value * to_gbps);
            }
        }
        // Handle attached unit like "1.3GB/s"
        if part.ends_with(unit) {
            let num_str = part.strip_suffix(unit)?.replace(',', ".");
            if let Ok(value) = num_str.parse::<f64>() {
                return Some(value * to_gbps);
            }
        }
    }
    None
}

fn calculate_throughput_from_line(line: &str) -> Option<f64> {
    // Parse bytes count
    let bytes: f64 = line
        .split_whitespace()
        .take_while(|s| !s.contains("bytes"))
        .last()?
        .parse()
        .ok()?;

    // Parse time - look for patterns like "0.083 s" or "0.083 secs"
    let time_idx = line.find(',')?;
    let time_part = &line[time_idx + 1..];

    // Extract number before "s" or "sec"
    for part in time_part.split_whitespace() {
        if let Ok(time) = part.replace(',', ".").parse::<f64>() {
            if time > 0.0 {
                return Some(bytes / time / (1024.0 * 1024.0 * 1024.0));
            }
        }
    }
    None
}
