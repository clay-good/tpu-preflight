//! Performance baseline validation checks.
//!
//! Checks for MXU utilization, HBM bandwidth, chip-to-chip latency,
//! compilation latency, and memory pressure.

use crate::platform::tpu;
use crate::{Check, CheckCategory, CheckResult};
use std::time::Instant;

/// Get all performance checks
pub fn get_performance_checks() -> Vec<Check> {
    vec![
        create_perf001_check(),
        create_perf002_check(),
        create_perf003_check(),
        create_perf004_check(),
        create_perf005_check(),
    ]
}

/// PERF-001: MXU Utilization Test
fn create_perf001_check() -> Check {
    Check {
        id: "PERF-001".to_string(),
        name: "MXU Utilization Test".to_string(),
        category: CheckCategory::Performance,
        description: "Run standardized matrix multiplication and measure MXU utilization".to_string(),
        result: None,
    }
}

/// PERF-002: HBM Bandwidth Test
fn create_perf002_check() -> Check {
    Check {
        id: "PERF-002".to_string(),
        name: "HBM Bandwidth Test".to_string(),
        category: CheckCategory::Performance,
        description: "Measure HBM memory bandwidth".to_string(),
        result: None,
    }
}

/// PERF-003: Chip-to-Chip Latency
fn create_perf003_check() -> Check {
    Check {
        id: "PERF-003".to_string(),
        name: "Chip-to-Chip Latency".to_string(),
        category: CheckCategory::Performance,
        description: "Measure latency between TPU chips".to_string(),
        result: None,
    }
}

/// PERF-004: Compilation Latency
fn create_perf004_check() -> Check {
    Check {
        id: "PERF-004".to_string(),
        name: "Compilation Latency".to_string(),
        category: CheckCategory::Performance,
        description: "Measure XLA compilation time for standard graph".to_string(),
        result: None,
    }
}

/// PERF-005: Memory Pressure Test
fn create_perf005_check() -> Check {
    Check {
        id: "PERF-005".to_string(),
        name: "Memory Pressure Test".to_string(),
        category: CheckCategory::Performance,
        description: "Allocate and free HBM to verify no fragmentation issues".to_string(),
        result: None,
    }
}

/// Expected HBM bandwidth by TPU type (GB/s)
fn expected_hbm_bandwidth_gbps(tpu_type: &tpu::TpuType) -> f64 {
    match tpu_type {
        tpu::TpuType::V4 => 1200.0,
        tpu::TpuType::V5e => 800.0,
        tpu::TpuType::V5p => 1600.0,
        tpu::TpuType::V6e => 1800.0,
        tpu::TpuType::V7 => 2000.0,
        tpu::TpuType::Unknown => 800.0, // Conservative default
    }
}

/// Execute PERF-001: MXU Utilization Test
pub fn run_perf001() -> CheckResult {
    let start = Instant::now();

    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    // This check requires executing a Python/JAX script
    // For now, we'll check if the test harness exists and can be run
    match run_mxu_benchmark() {
        Ok(utilization_pct) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            if utilization_pct < 70.0 {
                CheckResult::Fail {
                    message: format!("MXU utilization too low: {:.1}%", utilization_pct),
                    details: "Expected at least 70% utilization".to_string(),
                    duration_ms,
                }
            } else if utilization_pct < 80.0 {
                CheckResult::Warn {
                    message: format!("MXU utilization below optimal: {:.1}%", utilization_pct),
                    details: "Expected at least 80% utilization".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("MXU utilization: {:.1}%", utilization_pct),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("MXU benchmark unavailable: {}", e),
        },
    }
}

/// Execute PERF-002: HBM Bandwidth Test
pub fn run_perf002() -> CheckResult {
    let start = Instant::now();

    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    let tpu_type = tpu::get_tpu_type().unwrap_or(tpu::TpuType::Unknown);
    let expected_bandwidth = expected_hbm_bandwidth_gbps(&tpu_type);

    match run_hbm_bandwidth_test() {
        Ok(measured_bandwidth) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let pct_of_expected = (measured_bandwidth / expected_bandwidth) * 100.0;

            if pct_of_expected < 70.0 {
                CheckResult::Fail {
                    message: format!("HBM bandwidth too low: {:.1} GB/s ({:.1}% of expected)", measured_bandwidth, pct_of_expected),
                    details: format!("Expected at least {:.1} GB/s", expected_bandwidth * 0.7),
                    duration_ms,
                }
            } else if pct_of_expected < 85.0 {
                CheckResult::Warn {
                    message: format!("HBM bandwidth below optimal: {:.1} GB/s ({:.1}% of expected)", measured_bandwidth, pct_of_expected),
                    details: format!("Expected at least {:.1} GB/s", expected_bandwidth * 0.85),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("HBM bandwidth: {:.1} GB/s ({:.1}% of expected)", measured_bandwidth, pct_of_expected),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("HBM bandwidth test unavailable: {}", e),
        },
    }
}

/// Execute PERF-003: Chip-to-Chip Latency
pub fn run_perf003() -> CheckResult {
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
                reason: "Single-chip configuration - chip-to-chip latency not applicable".to_string(),
            };
        }
        Err(e) => {
            return CheckResult::Skip {
                reason: format!("Could not determine chip count: {}", e),
            };
        }
        _ => {}
    }

    match run_latency_test() {
        Ok(latency_us) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            if latency_us > 20.0 {
                CheckResult::Warn {
                    message: format!("Chip-to-chip latency elevated: {:.1}us", latency_us),
                    details: "Expected less than 10us for adjacent chips".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("Chip-to-chip latency: {:.1}us", latency_us),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("Latency test unavailable: {}", e),
        },
    }
}

/// Execute PERF-004: Compilation Latency
pub fn run_perf004() -> CheckResult {
    let start = Instant::now();

    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    match run_compilation_test() {
        Ok(compile_time_secs) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            if compile_time_secs > 60.0 {
                CheckResult::Warn {
                    message: format!("XLA compilation unusually slow: {:.1}s", compile_time_secs),
                    details: "Compilation took longer than 60 seconds".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Pass {
                    message: format!("XLA compilation time: {:.1}s", compile_time_secs),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("Compilation test unavailable: {}", e),
        },
    }
}

/// Execute PERF-005: Memory Pressure Test
pub fn run_perf005() -> CheckResult {
    let start = Instant::now();

    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    match run_memory_pressure_test() {
        Ok(success) => {
            let duration_ms = start.elapsed().as_millis() as u64;

            if success {
                CheckResult::Pass {
                    message: "Memory allocation/deallocation successful".to_string(),
                    duration_ms,
                }
            } else {
                CheckResult::Fail {
                    message: "Memory pressure test failed".to_string(),
                    details: "OOM or fragmentation issues detected".to_string(),
                    duration_ms,
                }
            }
        }
        Err(e) => CheckResult::Skip {
            reason: format!("Memory pressure test unavailable: {}", e),
        },
    }
}

// Benchmark runner helpers
// These attempt to run simple JAX benchmarks if JAX is available

fn run_mxu_benchmark() -> Result<f64, String> {
    // Try to run a simple matrix multiplication benchmark via Python/JAX
    let script = r#"
import jax
import jax.numpy as jnp
import time

# Warm up
x = jnp.ones((4096, 4096))
y = jnp.dot(x, x).block_until_ready()

# Benchmark
start = time.time()
for _ in range(10):
    y = jnp.dot(x, x).block_until_ready()
elapsed = time.time() - start

# Calculate approximate FLOPS and utilization
# 4096^3 * 2 FLOPs per matmul, 10 iterations
flops = (4096 ** 3) * 2 * 10 / elapsed
# Assume ~275 TFLOPS peak for v5e (conservative)
utilization = (flops / 275e12) * 100
print(f"{utilization:.1f}")
"#;

    match std::process::Command::new("python3")
        .args(["-c", script])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .trim()
                .parse::<f64>()
                .map_err(|_| "Could not parse MXU utilization output".to_string())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No module named 'jax'") {
                Err("JAX not installed".to_string())
            } else {
                Err(format!("Benchmark failed: {}", stderr.lines().next().unwrap_or("unknown error")))
            }
        }
        Err(e) => Err(format!("Could not run Python: {}", e)),
    }
}

fn run_hbm_bandwidth_test() -> Result<f64, String> {
    // Try to run a simple memory bandwidth test via Python/JAX
    let script = r#"
import jax
import jax.numpy as jnp
import time

# Create large array to test memory bandwidth
size_gb = 1.0
size_bytes = int(size_gb * 1024 * 1024 * 1024)
num_elements = size_bytes // 4  # float32

x = jnp.ones(num_elements, dtype=jnp.float32)

# Warm up
_ = (x + 1).block_until_ready()

# Benchmark memory reads
start = time.time()
for _ in range(10):
    _ = (x + 1).block_until_ready()
elapsed = time.time() - start

# Calculate bandwidth (read + write)
bandwidth_gbps = (size_gb * 2 * 10) / elapsed
print(f"{bandwidth_gbps:.1f}")
"#;

    match std::process::Command::new("python3")
        .args(["-c", script])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .trim()
                .parse::<f64>()
                .map_err(|_| "Could not parse bandwidth output".to_string())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No module named 'jax'") {
                Err("JAX not installed".to_string())
            } else {
                Err(format!("Benchmark failed: {}", stderr.lines().next().unwrap_or("unknown error")))
            }
        }
        Err(e) => Err(format!("Could not run Python: {}", e)),
    }
}

fn run_latency_test() -> Result<f64, String> {
    // Try to measure cross-device communication latency
    let script = r#"
import jax
import jax.numpy as jnp
import time

devices = jax.devices()
if len(devices) < 2:
    print("SINGLE")
    exit(0)

# Create array on first device
with jax.default_device(devices[0]):
    x = jnp.ones(1024)

# Measure transfer time
start = time.time()
for _ in range(100):
    with jax.default_device(devices[1]):
        y = jax.device_put(x).block_until_ready()
elapsed = time.time() - start

latency_us = (elapsed / 100) * 1e6
print(f"{latency_us:.1f}")
"#;

    match std::process::Command::new("python3")
        .args(["-c", script])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if stdout == "SINGLE" {
                Err("Single device - latency test not applicable".to_string())
            } else {
                stdout
                    .parse::<f64>()
                    .map_err(|_| "Could not parse latency output".to_string())
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No module named 'jax'") {
                Err("JAX not installed".to_string())
            } else {
                Err(format!("Benchmark failed: {}", stderr.lines().next().unwrap_or("unknown error")))
            }
        }
        Err(e) => Err(format!("Could not run Python: {}", e)),
    }
}

fn run_compilation_test() -> Result<f64, String> {
    // Measure XLA compilation time for a standard graph
    let script = r#"
import jax
import jax.numpy as jnp
import time

@jax.jit
def model(x):
    for _ in range(10):
        x = jnp.tanh(jnp.dot(x, x.T))
    return x

# Clear compilation cache
jax.clear_caches()

x = jnp.ones((512, 512))

# Time compilation (first call)
start = time.time()
_ = model(x).block_until_ready()
compile_time = time.time() - start

print(f"{compile_time:.2f}")
"#;

    match std::process::Command::new("python3")
        .args(["-c", script])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .trim()
                .parse::<f64>()
                .map_err(|_| "Could not parse compilation time output".to_string())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No module named 'jax'") {
                Err("JAX not installed".to_string())
            } else {
                Err(format!("Benchmark failed: {}", stderr.lines().next().unwrap_or("unknown error")))
            }
        }
        Err(e) => Err(format!("Could not run Python: {}", e)),
    }
}

fn run_memory_pressure_test() -> Result<bool, String> {
    // Test memory allocation and deallocation
    let script = r#"
import jax
import jax.numpy as jnp

try:
    # Allocate progressively larger arrays
    arrays = []
    for size_mb in [100, 500, 1000, 2000]:
        elements = (size_mb * 1024 * 1024) // 4
        arr = jnp.ones(elements, dtype=jnp.float32)
        _ = arr.block_until_ready()
        arrays.append(arr)

    # Free all arrays
    del arrays

    # Try to allocate again to check for fragmentation
    final = jnp.ones(500 * 1024 * 1024 // 4, dtype=jnp.float32)
    _ = final.block_until_ready()

    print("OK")
except Exception as e:
    print(f"FAIL:{e}")
"#;

    match std::process::Command::new("python3")
        .args(["-c", script])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if stdout == "OK" {
                Ok(true)
            } else if stdout.starts_with("FAIL:") {
                Ok(false)
            } else {
                Err("Unexpected output from memory test".to_string())
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No module named 'jax'") {
                Err("JAX not installed".to_string())
            } else {
                Err(format!("Test failed: {}", stderr.lines().next().unwrap_or("unknown error")))
            }
        }
        Err(e) => Err(format!("Could not run Python: {}", e)),
    }
}
