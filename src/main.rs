//! tpu-doc CLI entry point
//!
//! TPU environment diagnostics, discovery, and troubleshooting tool.

use tpu_doc::cli::args::{Args, Command};
use tpu_doc::cli::output::get_formatter;
use tpu_doc::commands;
use tpu_doc::version::get_build_info;
use tpu_doc::{run_checks as run_validation, TpuDocConfig};

use std::process::ExitCode;

fn main() -> ExitCode {
    // Parse command line arguments
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Run 'tpu-doc --help' for usage information.");
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
        Command::Info => run_info(&args),
        Command::Stack => run_stack(&args),
        Command::Cache => run_cache(&args),
        Command::Snapshot => run_snapshot(&args),
        Command::Audit => run_audit(&args),
        Command::Analyze => run_analyze(&args),
    }
}

fn print_version() {
    let info = get_build_info();
    println!("{}", info);
}

fn print_help() {
    println!(
        r#"tpu-doc - TPU environment diagnostics, discovery, and troubleshooting

USAGE:
    tpu-doc [COMMAND] [OPTIONS]

COMMANDS:
    check       Run validation checks (default)
    info        Display complete environment information
    stack       Analyze software stack compatibility
    cache       Analyze XLA compilation cache
    snapshot    Capture resource utilization snapshot
    audit       Run configuration audit
    analyze     AI-powered log analysis (requires --ai flag)
    version     Print version information
    list        List all available checks

CHECK OPTIONS:
    --all           Run all checks (default)
    --hardware      Run hardware health checks only
    --stack         Run software stack checks only
    --performance   Run performance baseline checks only
    --io            Run I/O throughput checks only
    --security      Run security posture checks only
    --config-audit  Run configuration audit checks only
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

INFO OPTIONS:
    (uses global --format option)

STACK OPTIONS:
    --matrix        Display full compatibility matrix

SNAPSHOT OPTIONS:
    --continuous <N>  Refresh every N seconds

ANALYZE OPTIONS:
    --ai              Enable AI analysis (required)
    --provider <P>    AI provider: anthropic, google (default: anthropic)
    --model <M>       Model to use (provider-specific)
    --question <Q>    Specific question to answer about the log

GENERAL:
    -h, --help      Print this help message
    -V, --version   Print version information

EXIT CODES:
    0   All checks passed
    1   One or more checks failed
    2   Warnings only (no failures)
    3   Runtime error

EXAMPLES:
    tpu-doc                           Run all checks with default settings
    tpu-doc check --hardware          Run only hardware checks
    tpu-doc info                      Display environment information
    tpu-doc info --format json        Environment info as JSON
    tpu-doc stack                     Analyze software stack
    tpu-doc stack --matrix            Show compatibility matrix
    tpu-doc cache                     Analyze XLA cache status
    tpu-doc snapshot                  Capture resource snapshot
    tpu-doc snapshot --continuous 5   Refresh every 5 seconds
    tpu-doc audit                     Run configuration audit
    tpu-doc analyze error.log --ai    AI analysis of log file
    tpu-doc check --format json --quiet > results.json
    tpu-doc list                      List all available checks"#
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
    println!("  STK-001  JAX Version Check");
    println!("  STK-002  libtpu Version Check");
    println!("  STK-003  XLA Compiler Check");
    println!("  STK-004  Python Version Check");
    println!("  STK-005  PJRT Plugin Check");
    println!("  STK-006  Dependency Conflict Check");
    println!("  STK-007  Environment Variables Check");
    println!();
    println!("PERFORMANCE CHECKS:");
    println!("  PERF-001 MXU Utilization Baseline");
    println!("  PERF-002 HBM Bandwidth Test");
    println!("  PERF-003 Chip-to-Chip Latency");
    println!("  PERF-004 XLA Compilation Latency");
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
    println!();
    println!("CONFIGURATION AUDIT CHECKS:");
    println!("  CFG-001  XLA Flags Audit");
    println!("  CFG-002  JAX Configuration Audit");
    println!("  CFG-003  Memory Preallocation Check");
    println!("  CFG-004  Distributed Configuration Check");
    println!("  CFG-005  Logging Configuration Check");
}

fn run_checks(args: &Args) -> ExitCode {
    // Build configuration from arguments
    let config = TpuDocConfig::from_args(args);

    // Run validation checks
    let report = match run_validation(config) {
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

fn run_info(args: &Args) -> ExitCode {
    match commands::info::run(args) {
        Ok(output) => {
            println!("{}", output);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error gathering environment info: {}", e);
            ExitCode::from(3)
        }
    }
}

fn run_stack(args: &Args) -> ExitCode {
    match commands::stack::run(args) {
        Ok(output) => {
            println!("{}", output);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error analyzing stack: {}", e);
            ExitCode::from(3)
        }
    }
}

fn run_cache(args: &Args) -> ExitCode {
    match commands::cache::run(args) {
        Ok(output) => {
            println!("{}", output);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error analyzing cache: {}", e);
            ExitCode::from(3)
        }
    }
}

fn run_snapshot(args: &Args) -> ExitCode {
    match commands::snapshot::run(args) {
        Ok(output) => {
            println!("{}", output);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error capturing snapshot: {}", e);
            ExitCode::from(3)
        }
    }
}

fn run_audit(args: &Args) -> ExitCode {
    match commands::audit::run(args) {
        Ok(output) => {
            println!("{}", output);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error running audit: {}", e);
            ExitCode::from(3)
        }
    }
}

fn run_analyze(args: &Args) -> ExitCode {
    match commands::analyze::run(args) {
        Ok(output) => {
            println!("{}", output);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error analyzing log: {}", e);
            ExitCode::from(3)
        }
    }
}
