# Configuration Reference

Complete reference for all tpu-preflight configuration options.

## Command Line Arguments

### Commands

```
tpu-preflight [COMMAND] [OPTIONS]
```

| Command | Description |
|---------|-------------|
| `check` | Run validation checks (default if no command specified) |
| `version` | Print version information |
| `list` | List all available checks |

### Check Category Options

Run checks for specific categories only:

| Option | Description |
|--------|-------------|
| `--all` | Run all checks (default) |
| `--hardware` | Run hardware health checks only (HW-001 to HW-006) |
| `--stack` | Run software stack checks only (STK-001 to STK-007) |
| `--performance` | Run performance baseline checks only (PERF-001 to PERF-005) |
| `--io` | Run I/O throughput checks only (IO-001 to IO-006) |
| `--security` | Run security posture checks only (SEC-001 to SEC-007) |

### Check Selection Options

Fine-grained control over which checks to run:

| Option | Description |
|--------|-------------|
| `--skip <ID>` | Skip specific check by ID (can be repeated) |
| `--only <ID>` | Run only specific check by ID (can be repeated) |

Examples:
```bash
# Skip specific checks
tpu-preflight check --skip HW-001 --skip SEC-007

# Run only specific checks
tpu-preflight check --only HW-001 --only HW-002

# Using equals syntax
tpu-preflight check --skip=HW-001 --skip=SEC-007
```

### Output Options

| Option | Description |
|--------|-------------|
| `--format <FMT>` | Output format: `text` (default), `json`, `junit` |
| `--quiet` | Only output failures and warnings |
| `--verbose` | Include detailed diagnostic information |
| `--no-color` | Disable colored output |

Examples:
```bash
# JSON output for programmatic use
tpu-preflight check --format json > results.json

# JUnit XML for CI/CD integration
tpu-preflight check --format junit > results.xml

# Quiet mode for scripts
tpu-preflight check --quiet

# Verbose mode for debugging
tpu-preflight check --verbose
```

### Behavior Options

| Option | Description | Default |
|--------|-------------|---------|
| `--timeout <MS>` | Global timeout in milliseconds | 30000 |
| `--parallel` | Run checks in parallel where safe | false |
| `--fail-fast` | Stop on first failure | false |

Examples:
```bash
# Increase timeout to 60 seconds
tpu-preflight check --timeout 60000

# Run checks in parallel for faster execution
tpu-preflight check --parallel

# Stop immediately on first failure
tpu-preflight check --fail-fast
```

### Configuration File Options

| Option | Description |
|--------|-------------|
| `--config <FILE>` | Load configuration from TOML file |
| `--baseline <FILE>` | Compare against baseline file |

Examples:
```bash
# Use configuration file
tpu-preflight check --config /etc/tpu-preflight/config.toml

# Compare against baseline
tpu-preflight check --baseline /var/lib/tpu-preflight/baseline.json
```

### Help and Version

| Option | Description |
|--------|-------------|
| `-h`, `--help` | Show help message |
| `-V`, `--version` | Print version information |

---

## Configuration File Format

tpu-preflight supports TOML configuration files for persistent settings.

### File Location

Configuration files can be specified via:
1. `--config <FILE>` command line option
2. `TPU_PREFLIGHT_CONFIG` environment variable
3. Default locations (checked in order):
   - `./tpu-preflight.toml`
   - `~/.config/tpu-preflight/config.toml`
   - `/etc/tpu-preflight/config.toml`

### Complete Configuration Example

```toml
# tpu-preflight.toml
# Configuration file for tpu-preflight validation tool

[checks]
# Checks to skip (by ID)
skip = ["SEC-001", "SEC-007"]

# Run only these checks (empty = run all non-skipped)
only = []

[thresholds]
# MXU utilization thresholds (percentage)
mxu_utilization_warn = 80
mxu_utilization_fail = 70

# HBM bandwidth thresholds (percentage of expected)
hbm_bandwidth_warn = 85
hbm_bandwidth_fail = 70

# HBM availability thresholds (percentage)
hbm_availability_warn = 90
hbm_availability_fail = 50

# Thermal thresholds (Celsius)
thermal_warn_celsius = 75
thermal_fail_celsius = 85

# Network latency threshold (milliseconds)
network_latency_warn_ms = 10

# Disk space threshold for checkpoint directory (GB)
checkpoint_space_warn_gb = 100

[timeouts]
# Global timeout for all checks (milliseconds)
global_ms = 30000

# Per-check timeout (milliseconds)
per_check_ms = 60000

# Network operation timeout (milliseconds)
network_ms = 5000

# DNS resolution timeout (milliseconds)
dns_ms = 3000

[output]
# Default output format: text, json, junit
format = "text"

# Enable colored output (false to disable)
color = true

# Verbose output by default
verbose = false

# Quiet mode (only failures and warnings)
quiet = false

[behavior]
# Run checks in parallel
parallel = false

# Stop on first failure
fail_fast = false

[baseline]
# Path to baseline file for comparison
path = ""

# Fail if results regress from baseline
fail_on_regression = false
```

### Configuration Sections

#### [checks]

Control which checks are executed:

| Key | Type | Description |
|-----|------|-------------|
| `skip` | array | List of check IDs to skip |
| `only` | array | List of check IDs to run exclusively |

#### [thresholds]

Customize pass/warn/fail thresholds:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `mxu_utilization_warn` | integer | 80 | MXU warning threshold (%) |
| `mxu_utilization_fail` | integer | 70 | MXU failure threshold (%) |
| `hbm_bandwidth_warn` | integer | 85 | HBM bandwidth warning (% of expected) |
| `hbm_bandwidth_fail` | integer | 70 | HBM bandwidth failure (% of expected) |
| `hbm_availability_warn` | integer | 90 | HBM availability warning (%) |
| `hbm_availability_fail` | integer | 50 | HBM availability failure (%) |
| `thermal_warn_celsius` | integer | 75 | Thermal warning threshold (C) |
| `thermal_fail_celsius` | integer | 85 | Thermal failure threshold (C) |
| `network_latency_warn_ms` | integer | 10 | Network latency warning (ms) |
| `checkpoint_space_warn_gb` | integer | 100 | Checkpoint directory space warning (GB) |

#### [timeouts]

Configure timeout values:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `global_ms` | integer | 30000 | Global timeout for all checks |
| `per_check_ms` | integer | 60000 | Maximum time per individual check |
| `network_ms` | integer | 5000 | Network operation timeout |
| `dns_ms` | integer | 3000 | DNS resolution timeout |

#### [output]

Output formatting options:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `format` | string | "text" | Default output format |
| `color` | boolean | true | Enable colored output |
| `verbose` | boolean | false | Enable verbose output |
| `quiet` | boolean | false | Only show failures/warnings |

#### [behavior]

Execution behavior:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `parallel` | boolean | false | Run checks in parallel |
| `fail_fast` | boolean | false | Stop on first failure |

#### [baseline]

Baseline comparison settings:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `path` | string | "" | Path to baseline file |
| `fail_on_regression` | boolean | false | Fail if results regress |

---

## Environment Variables

tpu-preflight respects the following environment variables:

### Tool Configuration

| Variable | Description |
|----------|-------------|
| `TPU_PREFLIGHT_CONFIG` | Path to configuration file |
| `TPU_PREFLIGHT_FORMAT` | Default output format (text, json, junit) |
| `TPU_PREFLIGHT_VERBOSE` | Enable verbose output (set to any value) |
| `NO_COLOR` | Disable colored output (standard convention) |

### TPU Environment

These environment variables affect check behavior:

| Variable | Description | Used By |
|----------|-------------|---------|
| `TPU_NAME` | TPU resource name | STK-007 |
| `TPU_WORKER_ID` | Worker ID for multi-host | STK-007 |
| `TPU_CHIPS_PER_HOST` | Expected chip count | HW-001 |
| `TPU_LIBRARY_PATH` | Path to libtpu.so | STK-005 |
| `CHECKPOINT_DIR` | Checkpoint directory path | IO-004 |

### Software Version Detection

| Variable | Description | Used By |
|----------|-------------|---------|
| `JAX_VERSION` | JAX version override | STK-001 |
| `XLA_VERSION` | XLA version override | STK-003 |
| `PYTHON_VERSION` | Python version override | STK-004 |
| `LIBTPU_VERSION` | libtpu version override | STK-002 |
| `PYTHONPATH` | Python module paths | STK-007 |

### Example Usage

```bash
# Set default format to JSON
export TPU_PREFLIGHT_FORMAT=json

# Enable verbose output
export TPU_PREFLIGHT_VERBOSE=1

# Disable colors
export NO_COLOR=1

# Set configuration file
export TPU_PREFLIGHT_CONFIG=/etc/tpu-preflight/config.toml

# Run with environment configuration
tpu-preflight check
```

---

## Baseline Files

Baseline files store validation results for comparison across runs.

### Generating a Baseline

```bash
# Generate baseline from current run
tpu-preflight check --format json > baseline.json
```

### Comparing Against Baseline

```bash
# Compare current results to baseline
tpu-preflight check --baseline baseline.json
```

### Baseline File Format

Baseline files use JSON format matching the standard JSON output:

```json
{
  "timestamp": 1733580000,
  "hostname": "tpu-vm-001",
  "tpu_type": "v5e",
  "checks": [
    {
      "id": "HW-001",
      "name": "TPU Device Detection",
      "category": "Hardware",
      "result": {
        "status": "pass",
        "message": "8 chips detected",
        "duration_ms": 45
      }
    }
  ],
  "summary": {
    "passed": 28,
    "warned": 2,
    "failed": 0,
    "skipped": 1,
    "total": 31,
    "total_duration_ms": 12345
  }
}
```

### Comparison Output

When comparing against a baseline, the tool reports:

- **New failures**: Checks that newly failed
- **New warnings**: Checks that newly warned
- **Resolved**: Checks that improved from baseline
- **Regressions**: Checks that degraded from baseline
- **Unchanged**: Checks with same result

---

## Precedence Rules

When the same setting is specified in multiple places, the following precedence applies (highest to lowest):

1. Command line arguments
2. Environment variables
3. Configuration file
4. Default values

Example:
```bash
# Config file sets format=text
# Environment sets TPU_PREFLIGHT_FORMAT=json
# Command line wins with --format=junit
tpu-preflight check --format junit  # Uses junit
```

---

## Common Configuration Scenarios

### CI/CD Pipeline

```toml
# ci-config.toml
[output]
format = "junit"
color = false

[behavior]
fail_fast = true

[checks]
skip = ["SEC-007"]  # Skip informational firewall check
```

```bash
tpu-preflight check --config ci-config.toml > results.xml
```

### Production Validation

```toml
# production.toml
[thresholds]
mxu_utilization_warn = 85
mxu_utilization_fail = 75
hbm_bandwidth_warn = 90
hbm_bandwidth_fail = 80

[behavior]
parallel = true

[timeouts]
global_ms = 60000
```

### Quick Health Check

```bash
# Run only critical hardware checks
tpu-preflight check --hardware --fail-fast --quiet
```

### Comprehensive Audit

```bash
# Run all checks with verbose output and baseline comparison
tpu-preflight check --verbose --baseline last-known-good.json
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more checks failed |
| 2 | Warnings only (no failures) |
| 3 | Runtime error (not a check failure) |

Use exit codes in scripts:
```bash
tpu-preflight check --quiet
case $? in
  0) echo "All checks passed" ;;
  1) echo "Failures detected" ;;
  2) echo "Warnings only" ;;
  3) echo "Runtime error" ;;
esac
```
