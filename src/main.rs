//! tpu-preflight CLI entry point
//!
//! Pre-deployment validation tool for Google Cloud TPU environments.

use tpu_preflight::cli::args::{Args, Command};
use tpu_preflight::cli::output::get_formatter;
use tpu_preflight::version::get_build_info;
use tpu_preflight::{run_preflight, PreflightConfig};

use std::process::ExitCode;

fn main() -> ExitCode {
    // Parse command line arguments
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Run 'tpu-preflight --help' for usage information.");
            return ExitCode::from(3);
        }
    };

    // Handle help flag
    if args.help {
        print_help();
        return ExitCode::SUCCESS;
    }

    // Handle commands
    match args.command {
        Command::Version => {
            print_version();
            ExitCode::SUCCESS
        }
        Command::List => {
            print_check_list();
            ExitCode::SUCCESS
        }
        Command::Check => run_checks(&args),
    }
}

fn print_version() {
    let info = get_build_info();
    println!("{}", info);
}

fn print_help() {
    println!(
        r#"tpu-preflight - Pre-deployment validation for Google Cloud TPU environments

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

GENERAL:
    -h, --help      Print this help message
    -V, --version   Print version information

EXIT CODES:
    0   All checks passed
    1   One or more checks failed
    2   Warnings only (no failures)
    3   Runtime error

EXAMPLES:
    tpu-preflight                    Run all checks with default settings
    tpu-preflight check --hardware   Run only hardware checks
    tpu-preflight check --format json --quiet > results.json
    tpu-preflight list               List all available checks"#
    );
}

fn print_check_list() {
    println!("Available checks:");
    println!();
    println!("HARDWARE CHECKS:");
    println!("  HW-001   TPU Device Detection");
    println!("  HW-002   HBM Memory Availability");
    println!("  HW-003   TPU Thermal Status");
    println!("  HW-004   TPU Error Counters");
    println!("  HW-005   ICI Interconnect Status");
    println!("  HW-006   Driver Status");
    println!();
    println!("STACK CHECKS:");
    println!("  STK-001  JAX Version");
    println!("  STK-002  libtpu Version");
    println!("  STK-003  XLA Compiler Version");
    println!("  STK-004  Python Version");
    println!("  STK-005  PJRT Plugin Status");
    println!("  STK-006  Dependency Conflicts");
    println!("  STK-007  Environment Variables");
    println!();
    println!("PERFORMANCE CHECKS:");
    println!("  PERF-001 MXU Utilization Test");
    println!("  PERF-002 HBM Bandwidth Test");
    println!("  PERF-003 Chip-to-Chip Latency");
    println!("  PERF-004 Compilation Latency");
    println!("  PERF-005 Memory Pressure Test");
    println!();
    println!("I/O CHECKS:");
    println!("  IO-001   GCS Read Throughput");
    println!("  IO-002   Local Disk Throughput");
    println!("  IO-003   GCS Connectivity");
    println!("  IO-004   Checkpoint Directory Access");
    println!("  IO-005   Network Latency to GCP Services");
    println!("  IO-006   DNS Resolution");
    println!();
    println!("SECURITY CHECKS:");
    println!("  SEC-001  Service Account Permissions");
    println!("  SEC-002  Network Exposure");
    println!("  SEC-003  Workload Identity Status");
    println!("  SEC-004  Encryption Status");
    println!("  SEC-005  Instance Metadata Access");
    println!("  SEC-006  SSH Key Management");
    println!("  SEC-007  Firewall Rules");
}

fn run_checks(args: &Args) -> ExitCode {
    // Build configuration from arguments
    let config = PreflightConfig::from_args(args);

    // Run preflight checks
    let report = match run_preflight(config) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("Error running checks: {}", e);
            return ExitCode::from(3);
        }
    };

    // Get appropriate formatter
    let formatter = get_formatter(&args.format, args.no_color, args.verbose, args.quiet);

    // Format and print output
    let output = formatter.format(&report);
    println!("{}", output);

    // Determine exit code based on results
    let summary = report.summary();
    if summary.failed > 0 {
        ExitCode::from(1)
    } else if summary.warned > 0 {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}
