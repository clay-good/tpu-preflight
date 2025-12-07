# tpu-preflight

Pre-deployment validation tool for Google Cloud TPU environments.

## Overview

tpu-preflight is a single-binary CLI tool that answers one critical question before production deployment: **"Is this TPU environment ready for production?"**

Infrastructure teams at AI companies spend significant time debugging TPU deployment issues that could have been caught before production. Failed TPU chip detection, incompatible software versions, degraded hardware performance, I/O bottlenecks, and security misconfigurations waste expensive compute time and delay rollouts.

tpu-preflight provides comprehensive pre-deployment validation across five domains: hardware health, software stack compatibility, performance baselines, I/O throughput, and security posture. Run it before deploying inference workloads to catch problems early, not after your production traffic starts failing.

## The Problem

When organizations provision TPU capacity, they face a critical validation gap:

- No standardized way to verify TPU hardware is healthy before deployment
- No automated check that software versions (JAX, libtpu, XLA) are compatible
- No baseline performance validation to detect degraded hardware
- No pre-deployment I/O verification for checkpoint loading
- No security posture assessment before routing production traffic

Teams cobble together ad-hoc scripts, rely on tribal knowledge, or discover problems only after deployment fails.

## The Solution

tpu-preflight addresses this gap with:

- **Single static binary** - Zero runtime dependencies, deploys anywhere
- **Comprehensive validation** - 31 checks across 5 categories in under 30 seconds
- **CI/CD native** - JUnit XML output, exit codes, baseline comparison
- **Read-only and safe** - Never modifies system state, safe for production

## Quick Start

```bash
# Download the binary (or build from source)
# ./tpu-preflight

# Run all validation checks
tpu-preflight

# Run specific category
tpu-preflight check --hardware

# Output JSON for programmatic use
tpu-preflight check --format json > results.json

# List all available checks
tpu-preflight list
```

## How It Works

tpu-preflight validates TPU environments through five modules:

**Hardware Checks (HW-001 to HW-006)**
- TPU device detection and chip count verification
- HBM memory availability and capacity
- Thermal status monitoring
- Hardware error counters
- ICI interconnect status (multi-chip)
- Driver status and version

**Stack Checks (STK-001 to STK-007)**
- JAX version compatibility
- libtpu version validation
- XLA compiler detection
- Python version requirements
- PJRT plugin availability
- Dependency conflict detection
- Environment variable verification

**Performance Checks (PERF-001 to PERF-005)**
- MXU utilization baseline
- HBM bandwidth measurement
- Chip-to-chip latency (multi-chip)
- XLA compilation latency
- Memory pressure testing

**I/O Checks (IO-001 to IO-006)**
- GCS read throughput
- Local disk throughput
- GCS connectivity
- Checkpoint directory access
- Network latency to GCP services
- DNS resolution

**Security Checks (SEC-001 to SEC-007)**
- Service account permissions
- Network exposure audit
- Workload identity status
- Encryption configuration
- Metadata access controls
- SSH key management
- Firewall guidance

## System Architecture

```
+===========================================================================+
|                              tpu-preflight                                |
+===========================================================================+
|                                                                           |
|  +---------------------------------------------------------------------+  |
|  |                         CLI Layer                                   |  |
|  |   Argument Parser  |  Help Generator  |  Version Info               |  |
|  +---------------------------------------------------------------------+  |
|                                    |                                      |
|                                    v                                      |
|  +---------------------------------------------------------------------+  |
|  |                    Validation Engine                                |  |
|  |   Check Orchestrator (dependency resolution, parallel execution)    |  |
|  |   Result Aggregator (summary statistics, baseline comparison)       |  |
|  +---------------------------------------------------------------------+  |
|            |              |              |             |                   |
|            v              v              v             v                   |
|  +---------------------------------------------------------------------+  |
|  |                    Check Modules                                    |  |
|  |  Hardware | Stack | Performance | I/O | Security                    |  |
|  +---------------------------------------------------------------------+  |
|                                    |                                      |
|                                    v                                      |
|  +---------------------------------------------------------------------+  |
|  |                Platform Abstraction Layer                           |  |
|  |  TPU Device | Linux System | GCP Metadata | Network                 |  |
|  +---------------------------------------------------------------------+  |
|                                    |                                      |
|                                    v                                      |
|  +---------------------------------------------------------------------+  |
|  |                    Output Formatters                                |  |
|  |  Terminal (human-readable) | JSON | JUnit XML                       |  |
|  +---------------------------------------------------------------------+  |
+===========================================================================+
```

## Deterministic Logic

tpu-preflight uses **NO LLMs or AI** for validation. All checks are deterministic:

- Same system state produces same output
- Fully auditable validation logic
- No external API calls for decision-making
- Reproducible results across runs

## CLI Reference

```
USAGE:
    tpu-preflight [COMMAND] [OPTIONS]

COMMANDS:
    check       Run validation checks (default)
    version     Print version information
    list        List all available checks

CHECK OPTIONS:
    --all           Run all checks (default)
    --hardware      Run hardware health checks only
    --stack         Run software stack checks only
    --performance   Run performance baseline checks only
    --io            Run I/O throughput checks only
    --security      Run security posture checks only
    --skip <ID>     Skip specific check by ID (repeatable)
    --only <ID>     Run only specific check by ID (repeatable)

OUTPUT OPTIONS:
    --format <FMT>  Output format: text (default), json, junit
    --quiet         Only output failures and warnings
    --verbose       Include detailed diagnostic information
    --no-color      Disable colored output

BEHAVIOR OPTIONS:
    --timeout <MS>  Global timeout in milliseconds (default: 30000)
    --parallel      Run checks in parallel where safe
    --fail-fast     Stop on first failure

CONFIGURATION:
    --config <FILE>   Load configuration from TOML file
    --baseline <FILE> Compare against baseline file

EXIT CODES:
    0   All checks passed
    1   One or more checks failed
    2   Warnings only (no failures)
    3   Runtime error
```

## Safety Guarantees

tpu-preflight is designed to be safe for production environments:

- **Read-only operations** - Never writes to filesystem (except explicit output), never modifies system configuration, never changes TPU state
- **No network exfiltration** - Only connects to GCP metadata server and GCP services for connectivity checks
- **No credential storage** - Uses ambient credentials of the TPU VM, never stores or transmits credentials
- **Minimal privileges** - Runs with invoking user's privileges, does not require root for most checks
- **Deterministic behavior** - No randomness, no external dependencies that could change behavior

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/clay-good/tpu-preflight.git
cd tpu-preflight

# Build release binary
cargo build --release

# Binary is at target/release/tpu-preflight
./target/release/tpu-preflight --help
```

### Download Binary

Check the [Releases](https://github.com/clay-good/tpu-preflight/releases) page for pre-built binaries.

## Testing

The project includes comprehensive testing infrastructure that can run without actual TPU hardware.

### Running Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test cli_tests
cargo test output_tests
cargo test full_run_tests
```

### Test Categories

- **Unit tests** - Core library functionality tests
- **CLI tests** - Argument parsing, category filtering, output format selection
- **Output tests** - Terminal, JSON, and JUnit XML formatter validation
- **Full run tests** - Orchestrator execution, result aggregation, exit code handling

### Mock Platform Layer

Tests use a mock platform layer (`tests/mocks/platform.rs`) that simulates TPU environments without requiring actual hardware. The mock layer provides:

- Configurable TPU types (v4, v5e, v5p, v6e)
- Simulated chip counts and health states
- Mock HBM memory and thermal conditions
- Simulated GCP metadata and network responses
- Error injection for testing failure scenarios

Example mock configurations:
- `MockTpuConfig::healthy_v5e_8()` - Healthy 8-chip v5e pod
- `MockTpuConfig::thermal_warning()` - TPU with thermal throttling
- `MockTpuConfig::hbm_errors()` - TPU with HBM memory errors

### Benchmarks

```bash
# Run performance benchmarks
cargo bench
```

Benchmarks validate:
- Check execution overhead (< 10ms per check)
- Output formatting performance (< 100ms for full report)

## Limitations

### Critical Limitations

- **Must run on TPU VM** - Cannot validate TPUs remotely; the binary must execute directly on the TPU VM
- **No actual TPU hardware queries** - Without libtpu FFI bindings, hardware checks rely on environment variables, sysfs entries, and GCP metadata rather than direct TPU API calls
- **Performance benchmarks require JAX** - PERF-001 through PERF-005 (MXU utilization, HBM bandwidth, chip-to-chip latency, compilation latency, memory pressure) attempt to run via Python/JAX; if JAX is not installed, these checks skip
- **GCS throughput test requires configuration** - IO-001 skips unless a test GCS bucket is configured
- **No TOML config file parser** - The --config flag is documented but TOML parsing is not implemented; configuration must be done via CLI flags or environment variables

### Platform Detection Limitations

- **TPU type detection** - Relies on environment variables (TPU_NAME) or GCP metadata; may return "Unknown" on non-standard configurations
- **HBM memory values are estimates** - Without libtpu, HBM availability is estimated at 95% of theoretical capacity based on TPU type
- **Thermal readings are synthetic** - If sysfs thermal zones are unavailable, returns assumed normal temperature (65C)
- **Error counters from environment** - Hardware error counters read from TPU_CORRECTABLE_ERRORS/TPU_UNCORRECTABLE_ERRORS env vars; defaults to 0 if not set
- **ICI status inferred** - Inter-chip interconnect health is inferred from TPU type, not directly queried

### Software Detection Limitations

- **JAX version detection** - Attempts to query JAX via Python import, falls back to pip show, then environment variable; requires Python available in PATH
- **XLA version detection** - Attempts to query jaxlib version via Python import; falls back to XLA_VERSION environment variable
- **Dependency conflict detection** - Checks known problematic version combinations (JAX/TensorFlow, NumPy 2.x compatibility, CUDA on TPU); does not perform exhaustive package scanning

### Network and I/O Limitations

- **No TLS/HTTPS inspection** - HTTPS endpoints are checked via TCP connectivity only; cannot verify certificate validity
- **Disk throughput uses dd** - Local disk test writes a 100MB file to /tmp; may be affected by filesystem caching
- **GCS connectivity is TCP-only** - Verifies TCP connection to storage.googleapis.com:443 but not actual GCS API access

### Security Check Limitations

- **Cannot query IAM policies** - Service account permissions checked via metadata server scopes only, not actual IAM bindings
- **Firewall rules informational only** - SEC-007 cannot query VPC firewall rules from within the instance
- **Metadata protection detection is heuristic** - Checks if metadata server returns 403 without headers, which may not indicate full protection

### Architectural Limitations

- **Parallel execution** - Uses scoped threads for parallel check execution within dependency batches; limited to max 4 concurrent checks per batch
- **No persistent configuration** - Each run is stateless; baseline comparison requires explicit --baseline flag
- **macOS/Windows build** - Compiles on non-Linux but most checks will skip or fail due to missing /proc, /sys filesystems

### What This Tool Does NOT Do

- Does not modify any system state (by design)
- Does not integrate with GCP APIs (IAM, Compute, etc.) beyond metadata server
- Does not provide remediation - only detection and reporting
- Does not replace Google Cloud's TPU health monitoring
- Does not validate multi-host TPU pod configurations beyond single-VM checks

## Documentation

- [docs/architecture.md](docs/architecture.md) - System architecture and design
- [docs/checks.md](docs/checks.md) - Complete check reference (31 checks documented)
- [docs/configuration.md](docs/configuration.md) - Configuration options and TOML file format
- [docs/ci-integration.md](docs/ci-integration.md) - CI/CD integration guide (GitHub Actions, GitLab CI, Jenkins, Kubernetes)
- [docs/development.md](docs/development.md) - Development guide (building, testing, adding checks)
