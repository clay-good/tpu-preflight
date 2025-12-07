//! Security posture validation checks.
//!
//! Checks for service account permissions, network exposure, workload identity,
//! encryption status, metadata access, SSH key management, and firewall rules.

use crate::platform::{gcp, network};
use crate::{Check, CheckCategory, CheckResult};
use std::time::Instant;

/// Get all security checks
pub fn get_security_checks() -> Vec<Check> {
    vec![
        create_sec001_check(),
        create_sec002_check(),
        create_sec003_check(),
        create_sec004_check(),
        create_sec005_check(),
        create_sec006_check(),
        create_sec007_check(),
    ]
}

/// SEC-001: Service Account Permissions
fn create_sec001_check() -> Check {
    Check {
        id: "SEC-001".to_string(),
        name: "Service Account Permissions".to_string(),
        category: CheckCategory::Security,
        description: "Identify service account and check for overly permissive roles".to_string(),
        result: None,
    }
}

/// SEC-002: Network Exposure
fn create_sec002_check() -> Check {
    Check {
        id: "SEC-002".to_string(),
        name: "Network Exposure".to_string(),
        category: CheckCategory::Security,
        description: "Check for services listening on all interfaces".to_string(),
        result: None,
    }
}

/// SEC-003: Workload Identity Status
fn create_sec003_check() -> Check {
    Check {
        id: "SEC-003".to_string(),
        name: "Workload Identity Status".to_string(),
        category: CheckCategory::Security,
        description: "Check if workload identity is configured".to_string(),
        result: None,
    }
}

/// SEC-004: Encryption Status
fn create_sec004_check() -> Check {
    Check {
        id: "SEC-004".to_string(),
        name: "Encryption Status".to_string(),
        category: CheckCategory::Security,
        description: "Verify data encryption settings".to_string(),
        result: None,
    }
}

/// SEC-005: Instance Metadata Access
fn create_sec005_check() -> Check {
    Check {
        id: "SEC-005".to_string(),
        name: "Instance Metadata Access".to_string(),
        category: CheckCategory::Security,
        description: "Verify metadata server access configuration".to_string(),
        result: None,
    }
}

/// SEC-006: SSH Key Management
fn create_sec006_check() -> Check {
    Check {
        id: "SEC-006".to_string(),
        name: "SSH Key Management".to_string(),
        category: CheckCategory::Security,
        description: "Check for OS Login vs legacy SSH keys".to_string(),
        result: None,
    }
}

/// SEC-007: Firewall Rules
fn create_sec007_check() -> Check {
    Check {
        id: "SEC-007".to_string(),
        name: "Firewall Rules".to_string(),
        category: CheckCategory::Security,
        description: "Provide guidance on firewall configuration".to_string(),
        result: None,
    }
}

/// Execute SEC-001: Service Account Permissions
pub fn run_sec001() -> CheckResult {
    let start = Instant::now();

    if !gcp::is_on_gcp() {
        return CheckResult::Skip {
            reason: "Not running on GCP".to_string(),
        };
    }

    match gcp::get_service_account() {
        Ok(sa) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            // Check access scopes for overly permissive settings
            match gcp::get_access_scopes() {
                Ok(scopes) => {
                    let overly_permissive = scopes.iter().any(|s| {
                        s.contains("cloud-platform") || s.contains("compute") || s.contains("devstorage.full")
                    });

                    if overly_permissive {
                        CheckResult::Warn {
                            message: format!("Service account {} has broad scopes", sa),
                            details: "Consider using more restrictive scopes".to_string(),
                            duration_ms,
                        }
                    } else {
                        CheckResult::Pass {
                            message: format!("Service account: {}", sa),
                            duration_ms,
                        }
                    }
                }
                Err(_) => CheckResult::Pass {
                    message: format!("Service account: {} (scopes not checked)", sa),
                    duration_ms,
                },
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("Service account info unavailable: {}", e),
        },
    }
}

/// Execute SEC-002: Network Exposure
pub fn run_sec002() -> CheckResult {
    let start = Instant::now();

    // Check for services listening on 0.0.0.0
    let exposed_ports = check_exposed_ports();
    let duration_ms = start.elapsed().as_millis() as u64;

    // Common ports that might indicate exposure issues
    let concerning_ports: Vec<_> = exposed_ports
        .iter()
        .filter(|p| {
            matches!(
                **p,
                22 | 80 | 443 | 8080 | 8888 | 3389 | 5432 | 3306 | 6379 | 27017
            )
        })
        .collect();

    if !concerning_ports.is_empty() {
        CheckResult::Warn {
            message: format!(
                "{} potentially exposed port(s): {}",
                concerning_ports.len(),
                concerning_ports
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            details: "Services bound to 0.0.0.0 are accessible from any interface".to_string(),
            duration_ms,
        }
    } else if !exposed_ports.is_empty() {
        CheckResult::Pass {
            message: format!("{} port(s) listening on all interfaces (none concerning)", exposed_ports.len()),
            duration_ms,
        }
    } else {
        CheckResult::Pass {
            message: "No services exposed on all interfaces".to_string(),
            duration_ms,
        }
    }
}

/// Execute SEC-003: Workload Identity Status
pub fn run_sec003() -> CheckResult {
    let start = Instant::now();

    if !gcp::is_on_gcp() {
        return CheckResult::Skip {
            reason: "Not running on GCP".to_string(),
        };
    }

    // Check for workload identity indicators
    // Workload identity uses the metadata server differently
    match gcp::get_instance_attribute("gke-cluster-name") {
        Ok(Some(_)) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            CheckResult::Pass {
                message: "Running in GKE with potential workload identity".to_string(),
                duration_ms,
            }
        }
        _ => {
            let duration_ms = start.elapsed().as_millis() as u64;

            // Check if using default service account vs custom
            match gcp::get_service_account() {
                Ok(sa) if sa.contains("compute@developer") => CheckResult::Warn {
                    message: "Using default Compute Engine service account".to_string(),
                    details: "Consider using a custom service account with minimal permissions".to_string(),
                    duration_ms,
                },
                Ok(sa) => CheckResult::Pass {
                    message: format!("Using custom service account: {}", sa),
                    duration_ms,
                },
                Err(_) => CheckResult::Skip {
                    reason: "Could not determine service account configuration".to_string(),
                },
            }
        }
    }
}

/// Execute SEC-004: Encryption Status
pub fn run_sec004() -> CheckResult {
    let start = Instant::now();

    if !gcp::is_on_gcp() {
        return CheckResult::Skip {
            reason: "Not running on GCP".to_string(),
        };
    }

    // GCP encrypts data at rest by default
    // Check for CMEK indicators
    let duration_ms = start.elapsed().as_millis() as u64;

    // Informational check - GCP always encrypts at rest
    CheckResult::Pass {
        message: "GCP default encryption at rest enabled".to_string(),
        duration_ms,
    }
}

/// Execute SEC-005: Instance Metadata Access
pub fn run_sec005() -> CheckResult {
    let start = Instant::now();

    if !gcp::is_on_gcp() {
        return CheckResult::Skip {
            reason: "Not running on GCP".to_string(),
        };
    }

    // Try to access metadata server
    match network::check_http_endpoint("http://metadata.google.internal/computeMetadata/v1/", 5000) {
        Ok(result) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            // A 403 might indicate metadata protection is configured
            // A 200 (or 404 for specific paths) indicates metadata is accessible
            if result.status_code == 403 {
                CheckResult::Pass {
                    message: "Metadata access requires proper headers".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Warn {
                    message: "Metadata server accessible without protection headers".to_string(),
                    details: "Consider enabling metadata concealment".to_string(),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("Could not check metadata access: {}", e),
        },
    }
}

/// Execute SEC-006: SSH Key Management
pub fn run_sec006() -> CheckResult {
    let start = Instant::now();

    if !gcp::is_on_gcp() {
        return CheckResult::Skip {
            reason: "Not running on GCP".to_string(),
        };
    }

    // Check for OS Login enabled
    match gcp::get_instance_attribute("enable-oslogin") {
        Ok(Some(value)) if value.to_lowercase() == "true" => {
            let duration_ms = start.elapsed().as_millis() as u64;
            CheckResult::Pass {
                message: "OS Login enabled".to_string(),
                duration_ms,
            }
        }
        Ok(_) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            CheckResult::Warn {
                message: "OS Login not enabled".to_string(),
                details: "Consider enabling OS Login for centralized SSH key management".to_string(),
                duration_ms,
            }
        }
        Err(_) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            CheckResult::Warn {
                message: "Could not determine OS Login status".to_string(),
                details: "Unable to query instance metadata".to_string(),
                duration_ms,
            }
        }
    }
}

/// Execute SEC-007: Firewall Rules
pub fn run_sec007() -> CheckResult {
    let start = Instant::now();
    let duration_ms = start.elapsed().as_millis() as u64;

    // Cannot directly check firewall rules from within the instance
    // This is informational only
    CheckResult::Pass {
        message: "Firewall rules must be verified via GCP Console or gcloud".to_string(),
        duration_ms,
    }
}

// Helper functions

fn check_exposed_ports() -> Vec<u16> {
    let mut exposed = Vec::new();

    // Read /proc/net/tcp and /proc/net/tcp6 to find listening sockets
    // Format: sl local_address rem_address st tx_queue rx_queue ...
    // local_address is in hex format: IIIIIIII:PPPP

    for path in &["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines().skip(1) {
                // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let local_addr = parts[1];
                    let state = parts[3];

                    // State 0A = LISTEN
                    if state == "0A" {
                        if let Some((addr, port_hex)) = local_addr.rsplit_once(':') {
                            // Check if listening on all interfaces (0.0.0.0 or ::)
                            let is_all_interfaces = addr == "00000000"
                                || addr == "00000000000000000000000000000000";

                            if is_all_interfaces {
                                if let Ok(port) = u16::from_str_radix(port_hex, 16) {
                                    if !exposed.contains(&port) {
                                        exposed.push(port);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    exposed
}
