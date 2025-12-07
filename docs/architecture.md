# tpu-preflight Architecture

## Overview

tpu-preflight is a single-binary command-line tool that validates Google Cloud TPU environments before production deployment. The tool performs read-only validation across five domains: hardware health, software stack compatibility, performance baselines, I/O throughput, and security posture.

The architecture prioritizes:

1. Zero runtime dependencies (statically linked binary)
2. Fast execution (target: under 30 seconds for all checks)
3. Graceful degradation (works with partial information)
4. Deterministic behavior (same inputs produce same outputs)
5. Safety (read-only operations, no system modifications)

---

## System Architecture Diagram

```
+===========================================================================+
|                              tpu-preflight                                |
+===========================================================================+
|                                                                           |
|  +---------------------------------------------------------------------+  |
|  |                         CLI Layer (cli/)                            |  |
|  |                                                                     |  |
|  |  +------------------+  +------------------+  +------------------+   |  |
|  |  |   Argument       |  |   Help           |  |   Version        |   |  |
|  |  |   Parser         |  |   Generator      |  |   Info           |   |  |
|  |  |   (args.rs)      |  |   (args.rs)      |  |   (version.rs)   |   |  |
|  |  +------------------+  +------------------+  +------------------+   |  |
|  |                                                                     |  |
|  +---------------------------------------------------------------------+  |
|                                    |                                      |
|                                    v                                      |
|  +---------------------------------------------------------------------+  |
|  |                    Validation Engine (engine/)                      |  |
|  |                                                                     |  |
|  |  +---------------------------+  +-------------------------------+   |  |
|  |  |    Check Orchestrator     |  |     Result Aggregator         |   |  |
|  |  |    (orchestrator.rs)      |  |     (result.rs)               |   |  |
|  |  |                           |  |                               |   |  |
|  |  |  - Dependency resolution  |  |  - Collect results           |   |  |
|  |  |  - Sequential/parallel    |  |  - Generate summary          |   |  |
|  |  |  - Timeout handling       |  |  - Baseline comparison       |   |  |
|  |  |  - Fail-fast support      |  |  - Report generation         |   |  |
|  |  +---------------------------+  +-------------------------------+   |  |
|  |                                                                     |  |
|  +---------------------------------------------------------------------+  |
|            |              |              |             |                   |
|            v              v              v             v                   |
|  +---------------------------------------------------------------------+  |
|  |                    Check Modules (checks/)                          |  |
|  |                                                                     |  |
|  |  +------------+  +----------+  +-------------+  +------+  +------+  |  |
|  |  | Hardware   |  | Stack    |  | Performance |  | I/O  |  | Sec  |  |  |
|  |  | (HW-001-   |  | (STK-001-|  | (PERF-001-  |  |(IO-  |  |(SEC- |  |  |
|  |  |  HW-006)   |  |  STK-007)|  |  PERF-005)  |  | 001- |  | 001- |  |  |
|  |  |            |  |          |  |             |  | 006) |  | 007) |  |  |
|  |  | - TPU      |  | - JAX    |  | - MXU util  |  |      |  |      |  |  |
|  |  |   detect   |  |   ver    |  | - HBM bw    |  | - GCS|  | - IAM|  |  |
|  |  | - HBM mem  |  | - libtpu |  | - Latency   |  | - DNS|  | - Net|  |  |
|  |  | - Thermal  |  | - Python |  | - Compile   |  | - Disk |  | - WI|  |  |
|  |  | - Errors   |  | - PJRT   |  |   time      |  |      |  |      |  |  |
|  |  | - ICI      |  | - Deps   |  |             |  |      |  |      |  |  |
|  |  | - Driver   |  | - Env    |  |             |  |      |  |      |  |  |
|  |  +------------+  +----------+  +-------------+  +------+  +------+  |  |
|  |                                                                     |  |
|  +---------------------------------------------------------------------+  |
|                                    |                                      |
|                                    v                                      |
|  +---------------------------------------------------------------------+  |
|  |                Platform Abstraction Layer (platform/)               |  |
|  |                                                                     |  |
|  |  +---------------+  +---------------+  +---------------+            |  |
|  |  |  TPU Device   |  |  Linux System |  |  GCP Metadata |            |  |
|  |  |  Interface    |  |  Interface    |  |  Interface    |            |  |
|  |  |  (tpu.rs)     |  |  (linux.rs)   |  |  (gcp.rs)     |            |  |
|  |  |               |  |               |  |               |            |  |
|  |  | - libtpu FFI  |  | - /proc       |  | - HTTP client |            |  |
|  |  | - sysfs       |  | - /sys        |  | - Instance    |            |  |
|  |  | - env vars    |  | - syscalls    |  |   metadata    |            |  |
|  |  +---------------+  +---------------+  +---------------+            |  |
|  |                                                                     |  |
|  |  +--------------------------------------------------------------+   |  |
|  |  |                    Network Interface (network.rs)            |   |  |
|  |  |                                                              |   |  |
|  |  |  - DNS resolution    - TCP connectivity    - HTTP requests   |   |  |
|  |  +--------------------------------------------------------------+   |  |
|  |                                                                     |  |
|  +---------------------------------------------------------------------+  |
|                                    |                                      |
|                                    v                                      |
|  +---------------------------------------------------------------------+  |
|  |                    Output Formatters (output/)                      |  |
|  |                                                                     |  |
|  |  +------------------+  +------------------+  +------------------+   |  |
|  |  |    Terminal      |  |      JSON        |  |     JUnit        |   |  |
|  |  |    Formatter     |  |    Formatter     |  |   XML Formatter  |   |  |
|  |  |  (terminal.rs)   |  |    (json.rs)     |  |    (junit.rs)    |   |  |
|  |  |                  |  |                  |  |                  |   |  |
|  |  | - Human-readable |  | - Machine-parse  |  | - CI/CD native   |   |  |
|  |  | - Color/mono     |  | - Structured     |  | - Test results   |   |  |
|  |  | - Progress       |  |   data           |  |   integration    |   |  |
|  |  +------------------+  +------------------+  +------------------+   |  |
|  |                                                                     |  |
|  +---------------------------------------------------------------------+  |
|                                                                           |
+===========================================================================+
```

---

## Data Flow

The following diagram illustrates the flow of data from invocation to output:

```
                            User Invocation
                                   |
                                   v
                    +-----------------------------+
                    |      Argument Parsing       |
                    |  (std::env::args -> Args)   |
                    +-----------------------------+
                                   |
                                   v
                    +-----------------------------+
                    |    Configuration Loading    |
                    |  (CLI args + config file)   |
                    +-----------------------------+
                                   |
                                   v
                    +-----------------------------+
                    |     Platform Detection      |
                    |  - Is this a TPU VM?        |
                    |  - Which TPU type?          |
                    |  - What capabilities?       |
                    +-----------------------------+
                                   |
                                   v
                    +-----------------------------+
                    |      Check Registration     |
                    |  - Filter by category       |
                    |  - Apply skip/only          |
                    |  - Resolve dependencies     |
                    +-----------------------------+
                                   |
                                   v
                    +-----------------------------+
                    |      Check Execution        |
                    |  (sequential or parallel)   |
                    |                             |
                    |  For each check:            |
                    |    1. Start timer           |
                    |    2. Execute check fn      |
                    |    3. Capture result        |
                    |    4. Handle timeout        |
                    +-----------------------------+
                                   |
                                   v
                    +-----------------------------+
                    |     Result Aggregation      |
                    |  - Collect all results      |
                    |  - Calculate summary        |
                    |  - Compare to baseline      |
                    +-----------------------------+
                                   |
                                   v
                    +-----------------------------+
                    |     Report Generation       |
                    |  - Build ValidationReport   |
                    |  - Add metadata             |
                    +-----------------------------+
                                   |
                                   v
                    +-----------------------------+
                    |     Output Formatting       |
                    |  - Select formatter         |
                    |  - Format report            |
                    |  - Write to stdout          |
                    +-----------------------------+
                                   |
                                   v
                    +-----------------------------+
                    |        Exit Code            |
                    |  0 = pass                   |
                    |  1 = failures               |
                    |  2 = warnings only          |
                    |  3 = runtime error          |
                    +-----------------------------+
```

---

## Component Descriptions

### CLI Layer

The CLI layer handles all user interaction. It parses command-line arguments without external dependencies using a hand-written parser that processes std::env::args(). This layer converts user intent into a structured configuration that drives the rest of the system.

Key responsibilities:
- Parse and validate command-line arguments
- Generate help text and version information
- Handle unknown arguments with clear error messages
- Support both short (-v) and long (--verbose) option forms

### Validation Engine

The validation engine orchestrates check execution and aggregates results. The orchestrator manages check dependencies, parallel execution, timeouts, and fail-fast behavior. The result aggregator collects individual check results, calculates summary statistics, and supports baseline comparison for regression detection.

Key responsibilities:
- Resolve check dependencies (check A requires check B)
- Execute checks sequentially or in parallel
- Enforce global and per-check timeouts
- Catch panics in check functions and convert to failures
- Aggregate results and generate reports

### Check Modules

Each check module contains a collection of related validation checks. Checks are pure functions that inspect system state and return a result (Pass, Warn, Fail, or Skip). Checks never modify system state and must complete within their timeout.

Module responsibilities:
- Hardware: Validate TPU chip presence, memory, thermal status, error counters
- Stack: Validate software versions and compatibility
- Performance: Validate baseline performance characteristics
- I/O: Validate storage and network throughput
- Security: Validate security configuration and posture

### Platform Abstraction Layer

The platform abstraction layer provides a consistent interface to system information regardless of the underlying implementation. This layer enables testing without TPU hardware through mock implementations and supports graceful degradation when information is unavailable.

Key abstractions:
- TPU device information (chip count, type, health)
- Linux system information (memory, CPU, disk)
- GCP metadata (project, zone, service account)
- Network connectivity (DNS, HTTP, TCP)

### Output Formatters

Output formatters transform the validation report into various formats for different consumers. The terminal formatter provides human-readable output with optional color. The JSON formatter provides machine-readable output for programmatic processing. The JUnit formatter provides CI/CD integration through standard test result format.

---

## Module Dependency Graph

```
main.rs
    |
    +---> cli/args.rs
    |         |
    |         +---> (no dependencies)
    |
    +---> lib.rs
              |
              +---> engine/orchestrator.rs
              |         |
              |         +---> checks/*.rs
              |         |         |
              |         |         +---> platform/*.rs
              |         |
              |         +---> engine/result.rs
              |
              +---> output/*.rs
                        |
                        +---> engine/result.rs (ValidationReport)
```

---

## Platform Layer Design

The platform layer uses a trait-based design that enables testing and extensibility:

```
trait TpuPlatform {
    fn is_tpu_vm(&self) -> bool;
    fn get_tpu_type(&self) -> Result<TpuType, PreflightError>;
    fn get_chip_count(&self) -> Result<u32, PreflightError>;
    fn get_hbm_info(&self) -> Result<HbmInfo, PreflightError>;
    fn get_health(&self) -> Result<TpuHealth, PreflightError>;
    // ...
}

struct RealTpuPlatform { /* uses libtpu, sysfs, env vars */ }
struct MockTpuPlatform { /* returns configured test values */ }
```

This design allows:
- Unit testing without TPU hardware
- Integration testing with mocked failures
- Future support for other accelerators (Trainium, etc.)
- Graceful degradation when data sources unavailable

---

## Error Handling Strategy

tpu-preflight uses a layered error handling approach:

Layer 1 - Platform Errors: Low-level errors from system calls, file I/O, and network operations. These are wrapped in PreflightError with context about what operation failed.

Layer 2 - Check Errors: Errors during check execution. These are converted to Fail results rather than propagated, allowing other checks to continue.

Layer 3 - Orchestration Errors: Errors in the validation engine itself. These result in exit code 3 (runtime error) and are reported clearly to the user.

Principle: A single failing check should not prevent other checks from running. Users see the complete picture, not just the first failure.

---

## Security Model

tpu-preflight is designed to be safe to run in production environments:

Read-Only Operations: The tool never writes to the filesystem (except explicit output), never modifies system configuration, and never changes TPU state.

No Network Exfiltration: The tool does not send data to external servers. Network operations are limited to GCP metadata server and connectivity checks to GCP services.

No Credential Storage: The tool does not store, cache, or transmit credentials. It uses the ambient credentials of the TPU VM.

Minimal Privileges: The tool runs with the privileges of the invoking user. It does not require root access for most checks. Checks that require elevated privileges will Skip gracefully if unavailable.

Deterministic Behavior: Given the same system state, the tool produces the same output. There is no randomness, no external dependencies that could change behavior, and no AI/ML components.

---

## Performance Characteristics

Target performance for a complete validation run:

| Component | Target | Notes |
|-----------|--------|-------|
| Argument parsing | < 1ms | No I/O |
| Platform detection | < 100ms | Cached after first call |
| Hardware checks | < 5s | Parallel where safe |
| Stack checks | < 3s | Version parsing |
| Performance checks | < 15s | Includes micro-benchmarks |
| I/O checks | < 5s | Network latency dependent |
| Security checks | < 2s | Metadata queries |
| Output formatting | < 100ms | Even for large reports |
| Total | < 30s | Typical case |

Memory usage: < 50MB peak (no large allocations, streaming where possible)

Binary size: < 5MB (static linking, LTO optimization)

---

## Future Extensibility

The architecture supports future extensions:

New Check Types: Add new checks by implementing a function in the appropriate module and registering it with the orchestrator.

New TPU Generations: Add TPU type variants and baseline expectations without changing the check logic.

New Output Formats: Implement the OutputFormatter trait for new formats.

New Platforms: Implement platform traits for new cloud providers or accelerators.

Remote Execution: The library API (run_preflight function) enables integration into larger systems that might run validation remotely.

---

## File Organization

```
tpu-preflight/
|
+-- Cargo.toml              # Project manifest
+-- build.rs                # Build script (version info, libtpu detection)
+-- README.md               # User-facing documentation
|
+-- src/
|   +-- main.rs             # Binary entry point
|   +-- lib.rs              # Library entry point and public API
|   +-- version.rs          # Version and build information
|   |
|   +-- cli/
|   |   +-- mod.rs          # Module exports
|   |   +-- args.rs         # Argument parsing
|   |   +-- output.rs       # Output selection
|   |
|   +-- checks/
|   |   +-- mod.rs          # Check registration
|   |   +-- hardware.rs     # HW-001 through HW-006
|   |   +-- stack.rs        # STK-001 through STK-007
|   |   +-- performance.rs  # PERF-001 through PERF-005
|   |   +-- io.rs           # IO-001 through IO-006
|   |   +-- security.rs     # SEC-001 through SEC-007
|   |
|   +-- engine/
|   |   +-- mod.rs          # Module exports
|   |   +-- orchestrator.rs # Check execution
|   |   +-- result.rs       # Result aggregation
|   |
|   +-- platform/
|   |   +-- mod.rs          # Module exports and traits
|   |   +-- tpu.rs          # TPU device interface
|   |   +-- linux.rs        # Linux system interface
|   |   +-- gcp.rs          # GCP metadata interface
|   |   +-- network.rs      # Network interface
|   |
|   +-- output/
|       +-- mod.rs          # Formatter trait and factory
|       +-- terminal.rs     # Human-readable output
|       +-- json.rs         # JSON output
|       +-- junit.rs        # JUnit XML output
|
+-- tests/
|   +-- integration/        # Integration tests with mocks
|   +-- unit/               # Unit tests
|
+-- benches/
|   +-- check_performance.rs # Performance benchmarks
|
+-- docs/
    +-- architecture.md     # This document
    +-- checks.md           # Check reference
    +-- configuration.md    # Configuration options
    +-- ci-integration.md   # CI/CD integration
    +-- development.md      # Development guide
```
