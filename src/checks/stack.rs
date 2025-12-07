//! Software stack validation checks.
//!
//! Checks for JAX, libtpu, XLA, Python versions, PJRT plugin status,
//! dependency conflicts, and environment variables.

use crate::platform::{linux, tpu};
use crate::{Check, CheckCategory, CheckResult};
use std::time::Instant;

/// Get all stack checks
pub fn get_stack_checks() -> Vec<Check> {
    vec![
        create_stk001_check(),
        create_stk002_check(),
        create_stk003_check(),
        create_stk004_check(),
        create_stk005_check(),
        create_stk006_check(),
        create_stk007_check(),
    ]
}

/// STK-001: JAX Version
fn create_stk001_check() -> Check {
    Check {
        id: "STK-001".to_string(),
        name: "JAX Version".to_string(),
        category: CheckCategory::Stack,
        description: "Detect and validate installed JAX version".to_string(),
        result: None,
    }
}

/// STK-002: libtpu Version
fn create_stk002_check() -> Check {
    Check {
        id: "STK-002".to_string(),
        name: "libtpu Version".to_string(),
        category: CheckCategory::Stack,
        description: "Detect and validate libtpu version".to_string(),
        result: None,
    }
}

/// STK-003: XLA Compiler Version
fn create_stk003_check() -> Check {
    Check {
        id: "STK-003".to_string(),
        name: "XLA Compiler Version".to_string(),
        category: CheckCategory::Stack,
        description: "Detect XLA compiler version".to_string(),
        result: None,
    }
}

/// STK-004: Python Version
fn create_stk004_check() -> Check {
    Check {
        id: "STK-004".to_string(),
        name: "Python Version".to_string(),
        category: CheckCategory::Stack,
        description: "Check Python version compatibility".to_string(),
        result: None,
    }
}

/// STK-005: PJRT Plugin Status
fn create_stk005_check() -> Check {
    Check {
        id: "STK-005".to_string(),
        name: "PJRT Plugin Status".to_string(),
        category: CheckCategory::Stack,
        description: "Verify PJRT TPU plugin is available".to_string(),
        result: None,
    }
}

/// STK-006: Dependency Conflicts
fn create_stk006_check() -> Check {
    Check {
        id: "STK-006".to_string(),
        name: "Dependency Conflicts".to_string(),
        category: CheckCategory::Stack,
        description: "Check for known conflicting package versions".to_string(),
        result: None,
    }
}

/// STK-007: Environment Variables
fn create_stk007_check() -> Check {
    Check {
        id: "STK-007".to_string(),
        name: "Environment Variables".to_string(),
        category: CheckCategory::Stack,
        description: "Verify required environment variables are set".to_string(),
        result: None,
    }
}

/// Execute STK-001: JAX Version
pub fn run_stk001() -> CheckResult {
    let start = Instant::now();

    // Try to detect JAX version from environment or standard paths
    match detect_jax_version() {
        Ok(version) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            // Minimum required version for TPU support
            let min_version = (0, 4, 1);

            match parse_version(&version) {
                Some(parsed) => {
                    if parsed < min_version {
                        CheckResult::Fail {
                            message: format!("JAX version {} is too old", version),
                            details: format!(
                                "Minimum required version is {}.{}.{}",
                                min_version.0, min_version.1, min_version.2
                            ),
                            duration_ms,
                        }
                    } else {
                        CheckResult::Pass {
                            message: format!("JAX version {}", version),
                            duration_ms,
                        }
                    }
                }
                None => CheckResult::Warn {
                    message: format!("JAX version {} (unparseable)", version),
                    details: "Could not parse version for compatibility check".to_string(),
                    duration_ms,
                },
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("JAX version unavailable: {}", e),
        },
    }
}

/// Execute STK-002: libtpu Version
pub fn run_stk002() -> CheckResult {
    let start = Instant::now();

    match tpu::get_libtpu_version() {
        Ok(version) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            // Check if it's a development/nightly build
            if version.contains("dev") || version.contains("nightly") {
                CheckResult::Warn {
                    message: format!("libtpu version {}", version),
                    details: "Using development/nightly build".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("libtpu version {}", version),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("libtpu version unavailable: {}", e),
        },
    }
}

/// Execute STK-003: XLA Compiler Version
pub fn run_stk003() -> CheckResult {
    let start = Instant::now();

    match detect_xla_version() {
        Ok(version) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            CheckResult::Pass {
                message: format!("XLA version {}", version),
                duration_ms,
            }
        }
        Err(_) => CheckResult::Skip {
            reason: "XLA version not detectable (informational only)".to_string(),
        },
    }
}

/// Execute STK-004: Python Version
pub fn run_stk004() -> CheckResult {
    let start = Instant::now();

    match detect_python_version() {
        Ok(version) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            // Minimum required Python version
            let min_version = (3, 9, 0);

            match parse_version(&version) {
                Some(parsed) => {
                    if parsed < min_version {
                        CheckResult::Fail {
                            message: format!("Python version {} is too old", version),
                            details: format!(
                                "Minimum required version is {}.{}.{}",
                                min_version.0, min_version.1, min_version.2
                            ),
                            duration_ms,
                        }
                    } else {
                        CheckResult::Pass {
                            message: format!("Python version {}", version),
                            duration_ms,
                        }
                    }
                }
                None => CheckResult::Warn {
                    message: format!("Python version {} (unparseable)", version),
                    details: "Could not parse version for compatibility check".to_string(),
                    duration_ms,
                },
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("Python version unavailable: {}", e),
        },
    }
}

/// Execute STK-005: PJRT Plugin Status
pub fn run_stk005() -> CheckResult {
    let start = Instant::now();

    // Check TPU_LIBRARY_PATH environment variable
    let tpu_lib_path = linux::get_environment_variable("TPU_LIBRARY_PATH");
    let duration_ms = start.elapsed().as_millis() as u64;

    match tpu_lib_path {
        Some(path) => {
            // Verify the path exists
            if std::path::Path::new(&path).exists() {
                CheckResult::Pass {
                    message: format!("PJRT plugin found at {}", path),
                    duration_ms,
                }
            } else {
                CheckResult::Fail {
                    message: "TPU_LIBRARY_PATH points to non-existent location".to_string(),
                    details: format!("Path {} does not exist", path),
                    duration_ms,
                }
            }
        }
        None => {
            // Try to find libtpu.so in standard locations
            let standard_paths = [
                "/usr/local/lib/libtpu.so",
                "/usr/lib/libtpu.so",
            ];

            for path in standard_paths.iter() {
                if std::path::Path::new(path).exists() {
                    return CheckResult::Pass {
                        message: format!("PJRT plugin found at {}", path),
                        duration_ms,
                    };
                }
            }

            CheckResult::Warn {
                message: "TPU_LIBRARY_PATH not set".to_string(),
                details: "PJRT plugin location not specified".to_string(),
                duration_ms,
            }
        }
    }
}

/// Execute STK-006: Dependency Conflicts
pub fn run_stk006() -> CheckResult {
    let start = Instant::now();

    // Known conflicting package combinations
    let conflicts = check_known_conflicts();
    let duration_ms = start.elapsed().as_millis() as u64;

    if conflicts.is_empty() {
        CheckResult::Pass {
            message: "No known dependency conflicts".to_string(),
            duration_ms,
        }
    } else {
        CheckResult::Warn {
            message: format!("{} potential conflict(s) detected", conflicts.len()),
            details: conflicts.join("; "),
            duration_ms,
        }
    }
}

/// Execute STK-007: Environment Variables
pub fn run_stk007() -> CheckResult {
    let start = Instant::now();

    let mut missing_required = Vec::new();
    let mut missing_recommended = Vec::new();

    // Required for TPU operation
    let required_vars = ["TPU_NAME"];
    for var in required_vars.iter() {
        if linux::get_environment_variable(var).is_none() {
            missing_required.push(*var);
        }
    }

    // Recommended but not strictly required
    let recommended_vars = ["TPU_WORKER_ID", "PYTHONPATH"];
    for var in recommended_vars.iter() {
        if linux::get_environment_variable(var).is_none() {
            missing_recommended.push(*var);
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    if !missing_required.is_empty() {
        CheckResult::Fail {
            message: format!("Missing required environment variable(s): {}", missing_required.join(", ")),
            details: "These variables are required for TPU operation".to_string(),
            duration_ms,
        }
    } else if !missing_recommended.is_empty() {
        CheckResult::Warn {
            message: format!("Missing recommended variable(s): {}", missing_recommended.join(", ")),
            details: "These variables are recommended for optimal operation".to_string(),
            duration_ms,
        }
    } else {
        CheckResult::Pass {
            message: "All environment variables set".to_string(),
            duration_ms,
        }
    }
}

// Helper functions

fn detect_jax_version() -> Result<String, String> {
    // Try environment variable first
    if let Some(version) = linux::get_environment_variable("JAX_VERSION") {
        return Ok(version);
    }

    // Try to query JAX version via Python
    match std::process::Command::new("python3")
        .args(["-c", "import jax; print(jax.__version__)"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version.is_empty() {
                return Ok(version);
            }
        }
        _ => {}
    }

    // Try pip show as fallback
    match std::process::Command::new("pip3")
        .args(["show", "jax"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Version:") {
                    if let Some(version) = line.strip_prefix("Version:") {
                        return Ok(version.trim().to_string());
                    }
                }
            }
        }
        _ => {}
    }

    Err("JAX not installed or not detectable".to_string())
}

fn detect_xla_version() -> Result<String, String> {
    if let Some(version) = linux::get_environment_variable("XLA_VERSION") {
        return Ok(version);
    }

    // Try to get XLA version from jaxlib
    match std::process::Command::new("python3")
        .args(["-c", "import jaxlib; print(jaxlib.__version__)"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version.is_empty() {
                return Ok(format!("jaxlib {}", version));
            }
        }
        _ => {}
    }

    Err("XLA version not found".to_string())
}

fn detect_python_version() -> Result<String, String> {
    if let Some(version) = linux::get_environment_variable("PYTHON_VERSION") {
        return Ok(version);
    }

    // Try to execute python3 --version
    match std::process::Command::new("python3")
        .arg("--version")
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let version_str = if stdout.contains("Python") {
                stdout.to_string()
            } else {
                stderr.to_string()
            };

            // Parse "Python 3.11.5" -> "3.11.5"
            if let Some(version) = version_str.strip_prefix("Python ") {
                Ok(version.trim().to_string())
            } else {
                Err("Could not parse Python version".to_string())
            }
        }
        Err(e) => Err(format!("Failed to run python3: {}", e)),
    }
}

fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts.get(2).and_then(|p| {
            // Handle versions like "3.11.5rc1" by taking only the numeric part
            let numeric: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
            numeric.parse().ok()
        }).unwrap_or(0);
        Some((major, minor, patch))
    } else {
        None
    }
}

fn check_known_conflicts() -> Vec<String> {
    let mut conflicts = Vec::new();

    // Check for known conflicting package combinations
    // Query installed packages and compare versions

    // Get JAX version
    let jax_version = detect_jax_version().ok();

    // Get TensorFlow version if installed
    let tf_version = std::process::Command::new("python3")
        .args(["-c", "import tensorflow; print(tensorflow.__version__)"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    // Get NumPy version
    let numpy_version = std::process::Command::new("python3")
        .args(["-c", "import numpy; print(numpy.__version__)"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    // Check JAX + TensorFlow compatibility
    if let (Some(jax_v), Some(tf_v)) = (&jax_version, &tf_version) {
        // JAX 0.4.x may have issues with older TensorFlow
        if jax_v.starts_with("0.4") {
            if let Some(tf_major) = parse_major_version(tf_v) {
                if tf_major < 2 {
                    conflicts.push(format!(
                        "JAX {} with TensorFlow {} may cause conflicts",
                        jax_v, tf_v
                    ));
                }
            }
        }
    }

    // Check NumPy 2.x compatibility issues
    if let Some(np_v) = &numpy_version {
        if let Some(np_major) = parse_major_version(np_v) {
            if np_major >= 2 {
                // NumPy 2.x has breaking changes
                if let Some(jax_v) = &jax_version {
                    if let Some((maj, min, _)) = parse_version(jax_v) {
                        if maj == 0 && min < 4 {
                            conflicts.push(format!(
                                "JAX {} may not be compatible with NumPy {}",
                                jax_v, np_v
                            ));
                        }
                    }
                }
            }
        }
    }

    // Check for conflicting CUDA versions if applicable
    if let Ok(output) = std::process::Command::new("nvcc").arg("--version").output() {
        if output.status.success() {
            let nvcc_output = String::from_utf8_lossy(&output.stdout);
            // Check if using JAX with GPU but on TPU
            if jax_version.is_some() && nvcc_output.contains("cuda") {
                conflicts.push(
                    "CUDA toolkit detected - ensure using TPU-compatible JAX build".to_string()
                );
            }
        }
    }

    conflicts
}

fn parse_major_version(version: &str) -> Option<u32> {
    version.split('.').next()?.parse().ok()
}
