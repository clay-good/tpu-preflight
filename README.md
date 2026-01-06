# tpu-doc

**tpu-doc** is a single-binary diagnostic tool for Google Cloud TPU environments. It validates hardware health, discovers environment configuration, and provides optional AI-powered troubleshooting. Built for ML engineers and infrastructure teams who need reliable TPU deployments without wasting expensive compute time debugging preventable issues.

Run it before deploying workloads. Run it when something breaks. Get answers in seconds.

## The Problem

When organizations provision TPU capacity, they face a critical diagnostic gap:

- **No standardized validation** - Is the TPU hardware actually healthy? Are the chips detected? Is HBM available?
- **Version compatibility mysteries** - Which JAX version works with which libtpu? Will my code run?
- **Silent performance degradation** - Is the TPU throttling? Are there uncorrectable errors accumulating?
- **I/O bottlenecks** - Can the VM actually reach GCS at expected speeds? Is DNS working?
- **Security blind spots** - What permissions does the service account have? What ports are exposed?
- **Configuration landmines** - Are XLA flags set correctly? Is memory preallocation configured?
- **Cryptic error messages** - Training crashed. Now what?

Teams cobble together ad-hoc scripts, rely on tribal knowledge, or discover problems only after deployment fails. Expensive TPU time ($3-10+/hour) is wasted debugging issues that could have been caught automatically.

## The Solution

tpu-doc addresses this with:

- **Single static binary** - Zero runtime dependencies. Copy it to any TPU VM and run it.
- **36 validation checks** across 6 categories - Hardware, Stack, Performance, I/O, Security, Configuration
- **Complete environment discovery** - Full fingerprint of TPU type, software versions, and configuration
- **Optional AI-powered analysis** - Send error logs to Claude or Gemini for diagnosis (bring your own API key)
- **CI/CD ready** - JUnit XML output, meaningful exit codes, baseline comparison
- **Read-only and safe** - Never modifies system state. Safe to run in production.

## Installation

### Download Pre-built Binary

```bash
# Download latest release
curl -LO https://github.com/clay-good/tpu-doc/releases/latest/download/tpu-doc-linux-x86_64
chmod +x tpu-doc-linux-x86_64
mv tpu-doc-linux-x86_64 /usr/local/bin/tpu-doc

# Verify installation
tpu-doc version
```

### Build from Source

```bash
# Clone and build
git clone https://github.com/clay-good/tpu-doc.git
cd tpu-doc
cargo build --release

# Binary at target/release/tpu-doc
./target/release/tpu-doc --help

# Build with AI features (adds TLS support)
cargo build --release --features ai
```

### Requirements

- **To run**: Linux x86_64 or ARM64 (the binary is self-contained)
- **To build**: Rust 1.70+ and Cargo
- **For AI features**: API key for Anthropic or Google

## Quick Start

```bash
# Run all validation checks
tpu-doc check

# Run only hardware checks
tpu-doc check --hardware

# Show complete environment information
tpu-doc info

# Analyze software stack compatibility
tpu-doc stack

# Check XLA compilation cache
tpu-doc cache

# Capture resource utilization snapshot
tpu-doc snapshot

# Run configuration audit
tpu-doc audit

# AI-powered log analysis (requires --ai flag and API key)
ANTHROPIC_API_KEY=sk-ant-... tpu-doc analyze training.log --ai
```

## How It Works

tpu-doc answers three questions:

### 1. Is this TPU environment healthy?

The `check` command runs 36 validation checks across 6 categories:

```bash
tpu-doc check
```

```
================================================================================
                         TPU-DOC VALIDATION REPORT
================================================================================

Environment: v5e-8 | 8 chips | 128 GB HBM | us-central2-b
Timestamp: 2025-01-04T12:00:00Z

--------------------------------------------------------------------------------
HARDWARE (HW-001 to HW-006)
--------------------------------------------------------------------------------
[PASS] HW-001 TPU Device Detection      8 chips detected
[PASS] HW-002 Chip Count Verification   8/8 chips available
[PASS] HW-003 HBM Memory Check          128 GB total, 121 GB available
[PASS] HW-004 Thermal Status            Normal (62C)
[PASS] HW-005 Hardware Error Counters   0 correctable, 0 uncorrectable
[PASS] HW-006 ICI Interconnect Status   All links healthy

--------------------------------------------------------------------------------
SUMMARY: 36 passed, 0 warnings, 0 failed, 0 skipped
--------------------------------------------------------------------------------
```

### 2. What exactly is this environment?

The `info`, `stack`, `cache`, and `snapshot` commands provide detailed discovery:

```bash
tpu-doc info
```

```
================================================================================
                         TPU ENVIRONMENT INFORMATION
================================================================================

TPU INFORMATION
---------------
  TPU Type:        v5e-8
  Chip Count:      8
  HBM Capacity:    128 GB
  Zone:            us-central2-b

SOFTWARE STACK
--------------
  Python:          3.10.12
  JAX:             0.4.35
  jaxlib:          0.4.35
  libtpu:          0.1.dev20241028
  NumPy:           1.26.4
```

### 3. What is wrong and how do I fix it?

The `audit` command reviews configuration settings:

```bash
tpu-doc audit
```

The `analyze` command uses AI to diagnose log files:

```bash
ANTHROPIC_API_KEY=sk-ant-... tpu-doc analyze error.log --ai --question "Why is training hanging?"
```

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                 tpu-doc                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                          CLI Layer                                   │   │
│  │   Argument Parsing  │  Command Dispatch  │  Help Generation          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       Command Handlers                               │   │
│  │   check │ info │ stack │ cache │ snapshot │ audit │ analyze │ list  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         Check Engine                                 │   │
│  │   Orchestrator  │  Result Aggregator  │  Dependency Resolution       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│           ┌────────────────────────┼────────────────────────┐              │
│           ▼                        ▼                        ▼              │
│  ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐          │
│  │  Check Modules  │   │Platform Abstrac.│   │ Output Formats  │          │
│  │  HW│STK│PERF│IO │   │  TPU │ Linux    │   │ Text│JSON│JUnit │          │
│  │  SEC │ CFG      │   │  GCP │ Network  │   │                 │          │
│  └─────────────────┘   └─────────────────┘   └─────────────────┘          │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    AI Layer (Optional, feature-gated)                │   │
│  │          Anthropic Claude  │  Google Gemini  │  Prompt Builder       │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Deterministic Logic vs AI

tpu-doc maintains a strict separation:

**Deterministic Core (always available)**
- All 36 validation checks use deterministic logic
- Same system state produces same output every time
- Fully auditable - you can read the source code for every check
- No network calls for pass/fail decisions
- Works offline on air-gapped systems

**AI Features (opt-in, requires `--ai` flag)**
- Only used for the `analyze` command
- Requires explicit `--ai` flag - never runs automatically
- Requires your own API key (BYOK) - we don't provide one
- Never affects pass/fail decisions of validation checks
- Built separately with `--features ai` to keep base binary small

## CLI Reference

### Commands

| Command | Description |
|---------|-------------|
| `check` | Run validation checks (default if no command specified) |
| `info` | Display complete environment information |
| `stack` | Analyze software stack compatibility |
| `cache` | Analyze XLA compilation cache |
| `snapshot` | Capture resource utilization snapshot |
| `audit` | Run configuration audit |
| `analyze` | AI-powered log analysis (requires `--ai`) |
| `list` | List all available checks |
| `version` | Print version information |

### Check Command Options

```bash
# Category filters (can combine multiple)
tpu-doc check --hardware        # HW-001 to HW-006
tpu-doc check --stack           # STK-001 to STK-007
tpu-doc check --performance     # PERF-001 to PERF-005
tpu-doc check --io              # IO-001 to IO-006
tpu-doc check --security        # SEC-001 to SEC-007
tpu-doc check --config-audit    # CFG-001 to CFG-005

# Individual check selection
tpu-doc check --only HW-001 --only HW-002
tpu-doc check --skip PERF-001 --skip PERF-002

# Output formats
tpu-doc check --format text     # Human-readable (default)
tpu-doc check --format json     # Machine-readable JSON
tpu-doc check --format junit    # JUnit XML for CI/CD

# Output modifiers
tpu-doc check --quiet           # Only show failures and warnings
tpu-doc check --verbose         # Include timing and extra details
tpu-doc check --no-color        # Disable ANSI colors

# Behavior
tpu-doc check --timeout 60000   # Timeout in milliseconds
tpu-doc check --parallel        # Run checks in parallel
tpu-doc check --fail-fast       # Stop on first failure
```

### Analyze Command Options

```bash
# Basic usage (requires --ai flag)
tpu-doc analyze training.log --ai

# Choose provider
tpu-doc analyze error.log --ai --provider anthropic  # Default
tpu-doc analyze error.log --ai --provider google

# Ask specific question
tpu-doc analyze error.log --ai --question "Why did OOM occur?"

# Specify model
tpu-doc analyze error.log --ai --model claude-3-haiku-20240307
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more checks failed |
| 2 | Warnings only (no failures) |
| 3 | Runtime error |

## Safety Guarantees

tpu-doc is designed to be safe for production environments:

### Read-Only Operations
- **Never writes** to filesystem (except explicit `--output` files)
- **Never modifies** system configuration
- **Never changes** TPU state
- **Never installs** packages or dependencies

### Network Activity
- **GCP metadata server** (169.254.169.254) - for environment discovery
- **GCP services** (storage.googleapis.com) - for connectivity checks only
- **AI API endpoints** - only when `--ai` flag is explicitly used

### Credential Handling
- Uses ambient credentials of the TPU VM
- Never stores or transmits system credentials
- AI API keys read from environment variables, never logged
- No credential caching

### Minimal Privileges
- Runs with invoking user's privileges
- Does not require root for any check
- No setuid, no capabilities, no privileged operations

## Limitations

An honest assessment of what tpu-doc cannot do:

### Must Run on TPU VM
The binary must execute directly on the TPU VM. It cannot validate TPUs remotely or from your laptop. This is by design - it needs access to TPU device files, sysfs entries, and the GCP metadata server.

### Hardware Detection Limitations
- **TPU type detection** relies on environment variables (`TPU_NAME`) and GCP metadata. May return "Unknown" on non-standard configurations.
- **HBM memory values** are estimates based on TPU type specs. Without libtpu FFI bindings, we cannot query actual HBM usage.
- **Thermal readings** are synthetic if sysfs thermal zones are unavailable (returns assumed normal 65°C).
- **Error counters** read from environment variables. Defaults to 0 if not set by TPU runtime.
- **ICI status** is inferred from TPU type, not directly queried from hardware.

### Software Detection Limitations
- **JAX version detection** requires Python in PATH. Falls back to pip queries, then environment variables.
- **Performance checks** (PERF-001 to PERF-005) require JAX to be installed. They skip otherwise.
- **Dependency conflict detection** covers known problematic combinations, not exhaustive scanning.

### I/O and Network Limitations
- **GCS throughput test** requires explicit bucket configuration. Skips if not configured.
- **HTTPS endpoints** are checked via TCP connectivity only. Does not verify TLS certificates.
- **Disk throughput** uses dd to `/tmp`. May be affected by filesystem caching.

### Security Check Limitations
- **IAM policies** cannot be queried from within the VM. Only metadata server scopes are checked.
- **Firewall rules** cannot be read from within the instance. Informational only.
- **Multi-host pods** - only validates the local VM, not pod-wide configuration.

### AI Features Limitations
- **Requires API key** - you must provide your own Anthropic or Google API key
- **Incurs costs** - API usage is billed by the provider
- **Network required** - cannot work offline
- **10MB log limit** - larger files are truncated
- **No conversation memory** - each call is independent

### What This Tool Does NOT Do
- Modify any system state
- Install or update packages
- Manage TPU lifecycle (create, delete, start, stop)
- Replace Google Cloud monitoring
- Provide real-time continuous monitoring
- Manage credentials or authentication
- Work on non-TPU VMs (most checks will skip)

## Documentation

- [docs/architecture.md](docs/architecture.md) - System architecture and design
- [docs/checks.md](docs/checks.md) - Complete reference for all 36 checks
- [docs/commands.md](docs/commands.md) - Detailed command reference
- [docs/configuration.md](docs/configuration.md) - Configuration options
- [docs/ai-integration.md](docs/ai-integration.md) - AI features setup and usage
- [docs/ci-integration.md](docs/ci-integration.md) - CI/CD integration guide
- [docs/development.md](docs/development.md) - Development and contribution guide
