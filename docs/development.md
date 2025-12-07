# Development Guide

Guide for developing and contributing to tpu-preflight.

## Project Structure

```
tpu-preflight/
├── Cargo.toml              # Project manifest
├── Cargo.lock              # Dependency lock file
├── build.rs                # Build script (version info, libtpu detection)
├── README.md               # Project documentation
├── docs/
│   ├── architecture.md     # System architecture
│   ├── checks.md           # Check reference
│   ├── configuration.md    # Configuration reference
│   ├── ci-integration.md   # CI/CD integration guide
│   └── development.md      # This file
├── src/
│   ├── main.rs             # CLI entry point
│   ├── lib.rs              # Library entry point, core types
│   ├── version.rs          # Version and build information
│   ├── cli/
│   │   ├── mod.rs          # CLI module
│   │   ├── args.rs         # Argument parsing
│   │   └── output.rs       # Output formatting
│   ├── checks/
│   │   ├── mod.rs          # Checks module
│   │   ├── hardware.rs     # Hardware checks (HW-001 to HW-006)
│   │   ├── stack.rs        # Stack checks (STK-001 to STK-007)
│   │   ├── performance.rs  # Performance checks (PERF-001 to PERF-005)
│   │   ├── io.rs           # I/O checks (IO-001 to IO-006)
│   │   └── security.rs     # Security checks (SEC-001 to SEC-007)
│   ├── platform/
│   │   ├── mod.rs          # Platform module
│   │   ├── linux.rs        # Linux system interface
│   │   ├── gcp.rs          # GCP metadata interface
│   │   ├── tpu.rs          # TPU device interface
│   │   └── network.rs      # Network interface
│   ├── engine/
│   │   ├── mod.rs          # Engine module
│   │   ├── orchestrator.rs # Check orchestration
│   │   └── result.rs       # Result aggregation
│   └── output/
│       └── mod.rs          # Output formatters
├── tests/
│   ├── tests.rs            # Test runner
│   ├── mocks/
│   │   ├── mod.rs          # Mock module
│   │   └── platform.rs     # Mock platform layer
│   └── integration/
│       ├── mod.rs          # Integration test module
│       ├── cli_tests.rs    # CLI tests
│       ├── output_tests.rs # Output formatter tests
│       └── full_run_tests.rs # Full run tests
└── benches/
    └── check_performance.rs # Performance benchmarks
```

---

## Building from Source

### Prerequisites

- Rust toolchain (1.70.0 or later recommended)
- Git

Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Build Commands

```bash
# Clone repository
git clone https://github.com/clay-good/tpu-preflight.git
cd tpu-preflight

# Debug build (fast compilation, includes debug symbols)
cargo build

# Release build (optimized, stripped)
cargo build --release

# Check without building
cargo check

# Build with specific features
cargo build --release --features full
```

### Build Profiles

| Profile | Command | Use Case |
|---------|---------|----------|
| Debug | `cargo build` | Development, debugging |
| Release | `cargo build --release` | Production, distribution |

Release profile optimizations (from Cargo.toml):
- `lto = true` - Link-time optimization
- `codegen-units = 1` - Single codegen unit for better optimization
- `strip = true` - Strip symbols for smaller binary
- `opt-level = "z"` - Optimize for size
- `panic = "abort"` - Abort on panic (smaller binary)

### Cross-Compilation

For Linux x86_64 (from macOS or other platforms):
```bash
# Add target
rustup target add x86_64-unknown-linux-gnu

# Build (requires linker setup)
cargo build --release --target x86_64-unknown-linux-gnu
```

For fully static Linux binary (using musl):
```bash
# Add musl target
rustup target add x86_64-unknown-linux-musl

# Build static binary
cargo build --release --target x86_64-unknown-linux-musl
```

---

## Running Tests

### Test Commands

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test cli_tests
cargo test output_tests
cargo test full_run_tests

# Run specific test by name
cargo test test_parse_version_command

# Run tests in release mode
cargo test --release

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test tests
```

### Test Categories

| Category | Location | Description |
|----------|----------|-------------|
| Unit tests | `src/**/*.rs` | Module-level tests |
| Integration tests | `tests/integration/` | Cross-module tests |
| Doc tests | `src/lib.rs` | Documentation examples |

### Mock Platform Layer

Tests use mock implementations in `tests/mocks/platform.rs` to simulate TPU environments without hardware:

```rust
// Example: Create a healthy v5e-8 mock configuration
let config = MockTpuConfig::healthy_v5e_8();

// Example: Create a TPU with thermal warning
let config = MockTpuConfig::thermal_warning();

// Example: Create a TPU with HBM errors
let config = MockTpuConfig::hbm_errors();
```

Available mock configurations:
- `MockTpuConfig::healthy_v5e_8()` - Healthy 8-chip v5e
- `MockTpuConfig::healthy_v6e_4()` - Healthy 4-chip v6e
- `MockTpuConfig::thermal_warning()` - TPU with thermal warning
- `MockTpuConfig::hbm_errors()` - TPU with HBM errors
- `MockTpuConfig::non_tpu_vm()` - Non-TPU VM environment

---

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench check_performance
```

Benchmark targets:
- Check execution overhead (< 10ms per check)
- Output formatting performance (< 100ms for full report)

---

## Adding New Checks

### Step 1: Create Check Function

Add the check to the appropriate module in `src/checks/`:

```rust
// src/checks/hardware.rs

/// HW-007: New Hardware Check
fn create_hw007_check() -> Check {
    Check {
        id: "HW-007".to_string(),
        name: "New Hardware Check".to_string(),
        category: CheckCategory::Hardware,
        description: "Description of what this check validates".to_string(),
        result: None,
    }
}

/// Execute HW-007: New Hardware Check
pub fn run_hw007() -> CheckResult {
    let start = Instant::now();

    // Check if we're on a TPU VM
    if !tpu::is_tpu_vm() {
        return CheckResult::Skip {
            reason: "Not running on a TPU VM".to_string(),
        };
    }

    // Implement check logic
    let duration_ms = start.elapsed().as_millis() as u64;

    // Return appropriate result
    CheckResult::Pass {
        message: "Check passed".to_string(),
        duration_ms,
    }
}
```

### Step 2: Register the Check

Add the check to the `get_*_checks()` function:

```rust
// src/checks/hardware.rs

pub fn get_hardware_checks() -> Vec<Check> {
    vec![
        create_hw001_check(),
        create_hw002_check(),
        // ... existing checks ...
        create_hw007_check(),  // Add new check
    ]
}
```

### Step 3: Add to Orchestrator

Register the check runner in `src/engine/orchestrator.rs`:

```rust
// In create_all_checks() function
RegisteredCheck {
    id: "HW-007".to_string(),
    name: "New Hardware Check".to_string(),
    category: CheckCategory::Hardware,
    description: "Description".to_string(),
    check_fn: Box::new(|| {
        let mut check = checks::hardware::create_hw007_check();
        check.result = Some(checks::hardware::run_hw007());
        check
    }),
    dependencies: vec![],  // Add dependencies if needed
    estimated_duration_ms: 100,
},
```

### Step 4: Add Documentation

Update `docs/checks.md` with the new check:

```markdown
### HW-007: New Hardware Check

**Category:** Hardware
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Description of what this check validates.

**What It Validates:**
- Point 1
- Point 2

**Method:**
1. Step 1
2. Step 2

**Pass Criteria:**
- Condition for pass

**Warning Criteria:**
- Condition for warning

**Fail Criteria:**
- Condition for failure

**Skip Conditions:**
- When the check is skipped

**Troubleshooting:**
- How to fix common issues
```

### Step 5: Add Tests

Add unit tests in the check module:

```rust
// src/checks/hardware.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hw007_check_creation() {
        let check = create_hw007_check();
        assert_eq!(check.id, "HW-007");
        assert_eq!(check.category, CheckCategory::Hardware);
    }
}
```

Add integration tests:

```rust
// tests/integration/hardware_tests.rs

#[test]
fn test_hw007_on_mock_tpu() {
    // Test with mock platform
}
```

---

## Platform Layer Development

### Adding New Platform Support

The platform layer abstracts hardware and system interfaces. To add new platform support:

1. Create new module in `src/platform/`
2. Implement required functions
3. Add conditional compilation if needed

Example for AWS Trainium support:

```rust
// src/platform/trainium.rs

/// Check if running on Trainium instance
pub fn is_trainium_vm() -> bool {
    // Check for Trainium indicators
    std::path::Path::new("/dev/neuron0").exists()
}

/// Get Trainium device count
pub fn get_neuron_device_count() -> Result<u32, PreflightError> {
    // Implementation
}
```

### Mock Implementation Guidelines

When adding new platform functions, also add mock implementations:

```rust
// tests/mocks/platform.rs

impl MockPlatform {
    pub fn is_trainium_vm(&self) -> bool {
        self.trainium_config.is_some()
    }
}
```

---

## Output Format Development

### Adding New Output Formats

1. Implement the formatter trait in `src/cli/output.rs`:

```rust
/// YAML output formatter
pub struct YamlFormatter;

impl YamlFormatter {
    pub fn format(report: &ValidationReport, _verbose: bool) -> String {
        // Implement YAML formatting
        let mut output = String::new();
        output.push_str("---\n");
        output.push_str(&format!("timestamp: {}\n", report.timestamp));
        // ... continue formatting
        output
    }
}
```

2. Add to `OutputFormat` enum in `src/cli/args.rs`:

```rust
pub enum OutputFormat {
    Text,
    Json,
    Junit,
    Yaml,  // New format
}
```

3. Update format selection in `src/main.rs`

---

## Code Style

### Formatting

```bash
# Format all code
cargo fmt

# Check formatting without changing files
cargo fmt -- --check
```

### Linting

```bash
# Run clippy lints
cargo clippy

# Run clippy with all targets
cargo clippy --all-targets

# Fix clippy warnings automatically
cargo clippy --fix
```

### Guidelines

1. **No external dependencies** - All functionality implemented manually
2. **No panics** - Use Result types, handle errors gracefully
3. **No unsafe code** - Unless absolutely necessary with justification
4. **Document public items** - All public functions, structs, enums
5. **Test coverage** - Add tests for new functionality
6. **Consistent naming** - Follow Rust naming conventions

---

## Performance Considerations

### Memory Usage

- Avoid unnecessary allocations
- Use references where possible
- Reuse buffers for I/O operations

### Execution Time

- Target < 30 seconds for all checks
- Individual checks should complete in < 5 seconds
- Use timeouts for all blocking operations

### Binary Size

Current optimizations achieve ~500KB binary. To further reduce:
- Use `opt-level = "z"` (size optimization)
- Enable LTO and strip symbols
- Minimize string literals
- Avoid generic bloat

---

## Release Process

### Version Bumping

1. Update version in `Cargo.toml`
2. Update CHANGELOG (if maintained)
3. Commit changes

```bash
# Update Cargo.toml version
sed -i 's/version = "0.1.0"/version = "0.2.0"/' Cargo.toml

# Commit
git add Cargo.toml
git commit -m "Bump version to 0.2.0"
```

### Building Release Binary

```bash
# Build optimized release
cargo build --release

# Binary location
ls -la target/release/tpu-preflight

# Check binary size
du -h target/release/tpu-preflight
```

### Creating Release Artifacts

```bash
# Create release directory
mkdir -p release

# Copy binary
cp target/release/tpu-preflight release/tpu-preflight-linux-x86_64

# Generate checksums
cd release
sha256sum tpu-preflight-* > SHA256SUMS
```

### Tagging Release

```bash
# Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0"

# Push tag
git push origin v0.2.0
```

---

## Debugging

### Debug Build

```bash
# Build with debug symbols
cargo build

# Run with debug output
RUST_BACKTRACE=1 ./target/debug/tpu-preflight check --verbose
```

### Logging

Add debug output during development:

```rust
// Debug print (removed in release)
#[cfg(debug_assertions)]
eprintln!("Debug: value = {:?}", value);
```

### Common Issues

**Build fails with libtpu warning**
- This is expected on non-TPU systems
- The warning indicates libtpu-dependent features will be limited

**Tests fail with permission errors**
- Some tests require specific file permissions
- Run tests with appropriate user permissions

**Binary too large**
- Ensure release profile optimizations are enabled
- Check for debug symbols: `file target/release/tpu-preflight`

---

## Testing on TPU Hardware

### Provisioning Test TPU

```bash
# Create TPU VM
gcloud compute tpus tpu-vm create preflight-test \
  --zone=us-east5-a \
  --accelerator-type=v5litepod-8 \
  --version=v2-alpha-tpuv5-lite

# Copy binary
gcloud compute tpus tpu-vm scp \
  target/release/tpu-preflight \
  preflight-test:~/ \
  --zone=us-east5-a

# SSH and test
gcloud compute tpus tpu-vm ssh preflight-test --zone=us-east5-a
./tpu-preflight check --verbose
```

### Cleanup

```bash
# Delete test TPU
gcloud compute tpus tpu-vm delete preflight-test --zone=us-east5-a
```

---

## Continuous Integration

The project uses GitHub Actions for CI. See `.github/workflows/` for workflow definitions.

### Local CI Simulation

```bash
# Run what CI runs
cargo fmt -- --check
cargo clippy --all-targets
cargo test
cargo build --release
```
