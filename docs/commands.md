# Command Reference

Complete reference documentation for all tpu-doc commands.

## Command Summary

| Command | Description |
|---------|-------------|
| `check` | Run validation checks (default command) |
| `info` | Display complete environment information |
| `stack` | Analyze software stack compatibility |
| `cache` | Analyze XLA compilation cache |
| `snapshot` | Capture resource utilization snapshot |
| `audit` | Run configuration audit |
| `analyze` | AI-powered log analysis (requires --ai flag) |
| `list` | List all available checks |
| `version` | Print version information |

---

## check

Run validation checks against the TPU environment.

### Synopsis

```
tpu-doc check [OPTIONS]
tpu-doc [OPTIONS]
```

When no command is specified, `check` is the default.

### Description

The check command runs validation checks across six categories: Hardware, Stack, Performance, I/O, Security, and Configuration. Each check produces a pass, fail, warn, or skip result with an explanatory message.

### Options

**Category Selection:**

| Option | Description |
|--------|-------------|
| `--all` | Run all checks (default) |
| `--hardware` | Run hardware health checks only (HW-001 to HW-006) |
| `--stack` | Run software stack checks only (STK-001 to STK-007) |
| `--performance` | Run performance baseline checks only (PERF-001 to PERF-005) |
| `--io` | Run I/O throughput checks only (IO-001 to IO-006) |
| `--security` | Run security posture checks only (SEC-001 to SEC-007) |
| `--config-audit` | Run configuration audit checks only (CFG-001 to CFG-005) |

**Check Selection:**

| Option | Description |
|--------|-------------|
| `--skip <ID>` | Skip specific check by ID (repeatable) |
| `--only <ID>` | Run only specific checks by ID (repeatable) |

**Output Options:**

| Option | Description |
|--------|-------------|
| `--format <FMT>` | Output format: text (default), json, junit |
| `--quiet` | Only output failures and warnings |
| `--verbose` | Include detailed diagnostic information |
| `--no-color` | Disable colored output |

**Behavior Options:**

| Option | Description |
|--------|-------------|
| `--timeout <MS>` | Global timeout in milliseconds (default: 30000) |
| `--parallel` | Run checks in parallel where safe |
| `--fail-fast` | Stop on first failure |
| `--baseline <FILE>` | Compare results against baseline file |

### Examples

```bash
# Run all checks with default settings
tpu-doc check

# Run only hardware checks
tpu-doc check --hardware

# Run hardware and stack checks
tpu-doc check --hardware --stack

# Skip specific checks
tpu-doc check --skip HW-004 --skip PERF-001

# Run only specific checks
tpu-doc check --only HW-001 --only STK-001

# Output as JSON
tpu-doc check --format json > results.json

# Output as JUnit XML for CI/CD
tpu-doc check --format junit > results.xml

# Quiet mode - only show problems
tpu-doc check --quiet

# Verbose mode with all details
tpu-doc check --verbose

# Compare against baseline
tpu-doc check --baseline previous-results.json
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more checks failed |
| 2 | Warnings only (no failures) |
| 3 | Runtime error |

---

## info

Display complete environment information.

### Synopsis

```
tpu-doc info [OPTIONS]
```

### Description

The info command gathers and displays a complete fingerprint of the TPU environment without making pass/fail judgments. It answers the question "what exactly is this environment?"

### Options

| Option | Description |
|--------|-------------|
| `--format <FMT>` | Output format: text (default), json |
| `--no-color` | Disable colored output |

### Output Sections

**TPU Information:**
- TPU type and generation (v4, v5e, v5p, v6e)
- Chip count and topology
- HBM capacity per chip and total
- TPU VM machine type

**Software Stack:**
- Python version and path
- JAX version
- jaxlib version
- libtpu version
- NumPy version
- Key environment variables

**System Information:**
- Hostname
- Kernel version
- Total system memory
- CPU count

**GCP Information:**
- Project ID
- Zone
- Instance name
- Service account
- Scopes

**Network Information:**
- Internal IP
- External IP (if any)

### Examples

```bash
# Display environment info
tpu-doc info

# Output as JSON
tpu-doc info --format json

# Save to file
tpu-doc info --format json > environment.json
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 3 | Runtime error |

---

## stack

Analyze software stack compatibility.

### Synopsis

```
tpu-doc stack [OPTIONS]
```

### Description

The stack command provides detailed software stack analysis. It detects installed versions, checks compatibility against a known matrix, and provides upgrade recommendations.

### Options

| Option | Description |
|--------|-------------|
| `--format <FMT>` | Output format: text (default), json |
| `--no-color` | Disable colored output |

### Output Sections

**Version Detection:**
- All relevant package versions
- Detection method used (import, pip, env var)
- Flags for versions that couldn't be detected

**Compatibility Analysis:**
- Pairwise compatibility checks
- Compatibility status: fully compatible, compatible with warnings, incompatible
- Explanation of problematic combinations

**Recommendations:**
- Suggested upgrades if newer compatible versions exist
- "Blessed" version combination for detected TPU type
- Note if already on latest compatible versions

### Examples

```bash
# Analyze software stack
tpu-doc stack

# Output as JSON
tpu-doc stack --format json
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Stack is compatible |
| 1 | Incompatible versions detected |
| 2 | Warnings (potential issues) |
| 3 | Runtime error |

---

## cache

Analyze XLA compilation cache.

### Synopsis

```
tpu-doc cache [OPTIONS]
```

### Description

The cache command analyzes the XLA compilation cache status. It reports on cache configuration, contents, and health.

### Options

| Option | Description |
|--------|-------------|
| `--format <FMT>` | Output format: text (default), json |
| `--no-color` | Disable colored output |

### Output Sections

**Cache Location:**
- Configured cache directory
- Whether directory exists and is writable

**Cache Contents:**
- Number of cached compilations
- Total cache size
- Oldest and newest entries

**Cache Health:**
- Disk space available
- Permission status
- Staleness detection

**Recommendations:**
- Configuration suggestions if cache not set up
- Warnings for large or stale caches

### Examples

```bash
# Analyze XLA cache
tpu-doc cache

# Output as JSON
tpu-doc cache --format json
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Cache is healthy |
| 1 | Cache issues detected |
| 2 | Warnings (cache not configured, etc.) |
| 3 | Runtime error |

---

## snapshot

Capture resource utilization snapshot.

### Synopsis

```
tpu-doc snapshot [OPTIONS]
```

### Description

The snapshot command captures a point-in-time view of resource utilization. It answers "what are my resources doing right now?"

### Options

| Option | Description |
|--------|-------------|
| `--format <FMT>` | Output format: text (default), json |
| `--no-color` | Disable colored output |

### Output Sections

**TPU Resources:**
- HBM utilization per chip (if available)
- TPU duty cycle (if available)
- Temperature readings

**System Resources:**
- CPU utilization
- System memory usage
- Swap usage

**Process Information:**
- Top processes by memory
- Python processes and their memory usage

**I/O Status:**
- Disk I/O rates
- Network I/O rates

### Examples

```bash
# Capture snapshot
tpu-doc snapshot

# Output as JSON for monitoring
tpu-doc snapshot --format json
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 3 | Runtime error |

---

## audit

Run configuration audit.

### Synopsis

```
tpu-doc audit [OPTIONS]
```

### Description

The audit command examines configuration settings and provides recommendations for optimization. It checks XLA flags, JAX configuration, memory settings, and logging configuration.

### Options

| Option | Description |
|--------|-------------|
| `--format <FMT>` | Output format: text (default), json |
| `--no-color` | Disable colored output |

### Audit Categories

**XLA Flags (CFG-001):**
- Parse XLA_FLAGS environment variable
- Check for known anti-patterns
- Flag debug settings in production

**JAX Configuration (CFG-002):**
- x64 mode settings
- Default matmul precision
- Memory preallocation

**Memory Settings (CFG-003):**
- XLA_PYTHON_CLIENT_PREALLOCATE
- XLA_PYTHON_CLIENT_MEM_FRACTION

**Distributed Configuration (CFG-004):**
- JAX_COORDINATOR_ADDRESS
- CLOUD_TPU_TASK_ID

**Logging Configuration (CFG-005):**
- TF_CPP_MIN_LOG_LEVEL
- JAX_DEBUG_NANS

### Examples

```bash
# Run configuration audit
tpu-doc audit

# Output as JSON
tpu-doc audit --format json
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Configuration is optimal |
| 1 | Misconfigurations detected |
| 2 | Warnings (suboptimal settings) |
| 3 | Runtime error |

---

## analyze

AI-powered log analysis.

### Synopsis

```
tpu-doc analyze <LOG_FILE> --ai [OPTIONS]
```

### Description

The analyze command uses AI to diagnose issues in log files. It combines environment context with log content to provide intelligent analysis and recommendations.

**Note:** This command requires the `--ai` flag and an API key. Build with `--features ai` to enable.

### Options

| Option | Description |
|--------|-------------|
| `--ai` | Enable AI analysis (required) |
| `--provider <P>` | AI provider: anthropic (default), google |
| `--model <M>` | Model to use (provider-specific) |
| `--question <Q>` | Specific question to ask about the log |
| `--format <FMT>` | Output format: text (default), json |
| `--no-color` | Disable colored output |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | API key for Anthropic Claude |
| `GOOGLE_API_KEY` | API key for Google Gemini |

### Examples

```bash
# Analyze a log file with Anthropic Claude
ANTHROPIC_API_KEY=your-key tpu-doc analyze training.log --ai

# Use Google Gemini instead
GOOGLE_API_KEY=your-key tpu-doc analyze training.log --ai --provider google

# Ask a specific question
tpu-doc analyze error.log --ai --question "Why is training hanging?"

# Specify a different model
tpu-doc analyze training.log --ai --model claude-3-haiku-20240307
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Analysis completed successfully |
| 1 | Analysis identified critical issues |
| 3 | Runtime error (API failure, missing key, etc.) |

---

## list

List all available checks.

### Synopsis

```
tpu-doc list [OPTIONS]
```

### Description

The list command displays all available validation checks with their IDs, names, and categories.

### Options

| Option | Description |
|--------|-------------|
| `--format <FMT>` | Output format: text (default), json |

### Examples

```bash
# List all checks
tpu-doc list

# Output as JSON
tpu-doc list --format json
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |

---

## version

Print version information.

### Synopsis

```
tpu-doc version
tpu-doc --version
tpu-doc -V
```

### Description

Displays the tpu-doc version and build information.

### Examples

```bash
# Show version
tpu-doc version
tpu-doc --version
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |

---

## Global Options

These options can be used with any command:

| Option | Description |
|--------|-------------|
| `--help`, `-h` | Show help message |
| `--version`, `-V` | Show version |
| `--no-color` | Disable colored output |

---

## Common Workflows

### Pre-deployment Validation

```bash
# Run full validation before deploying
tpu-doc check

# If issues found, get more details
tpu-doc check --verbose

# Save results for comparison
tpu-doc check --format json > baseline.json
```

### CI/CD Integration

```bash
# Output JUnit for test reporting
tpu-doc check --format junit > results.xml

# Fail fast in CI
tpu-doc check --fail-fast

# Compare against known-good baseline
tpu-doc check --baseline baseline.json
```

### Troubleshooting

```bash
# Gather environment info
tpu-doc info > environment.txt

# Check software compatibility
tpu-doc stack

# Analyze error logs with AI
ANTHROPIC_API_KEY=$KEY tpu-doc analyze error.log --ai --question "Why did OOM occur?"
```

### Monitoring

```bash
# Capture resource snapshot
tpu-doc snapshot --format json >> metrics.jsonl

# Quick health check
tpu-doc check --hardware --quiet
```
