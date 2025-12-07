//! CLI integration tests.
//!
//! Tests for argument parsing and command handling.

use tpu_preflight::cli::args::{Args, CategoryFilter, Command, OutputFormat};

#[test]
fn test_default_args() {
    let args = Args::parse_from(&[]).unwrap();
    assert_eq!(args.command, Command::Check);
    assert_eq!(args.category, CategoryFilter::All);
    assert_eq!(args.format, OutputFormat::Text);
    assert!(!args.quiet);
    assert!(!args.verbose);
    assert!(!args.parallel);
    assert!(!args.fail_fast);
    assert_eq!(args.timeout_ms, 30000);
}

#[test]
fn test_version_command() {
    let args = Args::parse_from(&["version".to_string()]).unwrap();
    assert_eq!(args.command, Command::Version);
}

#[test]
fn test_list_command() {
    let args = Args::parse_from(&["list".to_string()]).unwrap();
    assert_eq!(args.command, Command::List);
}

#[test]
fn test_check_command() {
    let args = Args::parse_from(&["check".to_string()]).unwrap();
    assert_eq!(args.command, Command::Check);
}

#[test]
fn test_hardware_category() {
    let args = Args::parse_from(&["--hardware".to_string()]).unwrap();
    assert_eq!(args.category, CategoryFilter::Hardware);
}

#[test]
fn test_stack_category() {
    let args = Args::parse_from(&["--stack".to_string()]).unwrap();
    assert_eq!(args.category, CategoryFilter::Stack);
}

#[test]
fn test_performance_category() {
    let args = Args::parse_from(&["--performance".to_string()]).unwrap();
    assert_eq!(args.category, CategoryFilter::Performance);
}

#[test]
fn test_io_category() {
    let args = Args::parse_from(&["--io".to_string()]).unwrap();
    assert_eq!(args.category, CategoryFilter::Io);
}

#[test]
fn test_security_category() {
    let args = Args::parse_from(&["--security".to_string()]).unwrap();
    assert_eq!(args.category, CategoryFilter::Security);
}

#[test]
fn test_json_format() {
    let args = Args::parse_from(&["--format".to_string(), "json".to_string()]).unwrap();
    assert_eq!(args.format, OutputFormat::Json);
}

#[test]
fn test_junit_format() {
    let args = Args::parse_from(&["--format".to_string(), "junit".to_string()]).unwrap();
    assert_eq!(args.format, OutputFormat::Junit);
}

#[test]
fn test_format_equals_syntax() {
    let args = Args::parse_from(&["--format=json".to_string()]).unwrap();
    assert_eq!(args.format, OutputFormat::Json);
}

#[test]
fn test_quiet_flag() {
    let args = Args::parse_from(&["--quiet".to_string()]).unwrap();
    assert!(args.quiet);
}

#[test]
fn test_verbose_flag() {
    let args = Args::parse_from(&["--verbose".to_string()]).unwrap();
    assert!(args.verbose);
}

#[test]
fn test_no_color_flag() {
    let args = Args::parse_from(&["--no-color".to_string()]).unwrap();
    assert!(args.no_color);
}

#[test]
fn test_parallel_flag() {
    let args = Args::parse_from(&["--parallel".to_string()]).unwrap();
    assert!(args.parallel);
}

#[test]
fn test_fail_fast_flag() {
    let args = Args::parse_from(&["--fail-fast".to_string()]).unwrap();
    assert!(args.fail_fast);
}

#[test]
fn test_timeout_option() {
    let args = Args::parse_from(&["--timeout".to_string(), "60000".to_string()]).unwrap();
    assert_eq!(args.timeout_ms, 60000);
}

#[test]
fn test_timeout_equals_syntax() {
    let args = Args::parse_from(&["--timeout=45000".to_string()]).unwrap();
    assert_eq!(args.timeout_ms, 45000);
}

#[test]
fn test_skip_option() {
    let args = Args::parse_from(&[
        "--skip".to_string(),
        "HW-001".to_string(),
        "--skip".to_string(),
        "HW-002".to_string(),
    ])
    .unwrap();
    assert_eq!(args.skip, vec!["HW-001", "HW-002"]);
}

#[test]
fn test_skip_equals_syntax() {
    let args = Args::parse_from(&["--skip=SEC-001".to_string()]).unwrap();
    assert_eq!(args.skip, vec!["SEC-001"]);
}

#[test]
fn test_only_option() {
    let args = Args::parse_from(&[
        "--only".to_string(),
        "HW-001".to_string(),
        "--only".to_string(),
        "IO-006".to_string(),
    ])
    .unwrap();
    assert_eq!(args.only, vec!["HW-001", "IO-006"]);
}

#[test]
fn test_config_option() {
    let args = Args::parse_from(&["--config".to_string(), "/path/to/config.toml".to_string()]).unwrap();
    assert_eq!(args.config, Some("/path/to/config.toml".to_string()));
}

#[test]
fn test_baseline_option() {
    let args = Args::parse_from(&["--baseline".to_string(), "/path/to/baseline.json".to_string()]).unwrap();
    assert_eq!(args.baseline, Some("/path/to/baseline.json".to_string()));
}

#[test]
fn test_help_flag() {
    let args = Args::parse_from(&["--help".to_string()]).unwrap();
    assert!(args.help);
}

#[test]
fn test_short_help_flag() {
    let args = Args::parse_from(&["-h".to_string()]).unwrap();
    assert!(args.help);
}

#[test]
fn test_short_version_flag() {
    let args = Args::parse_from(&["-V".to_string()]).unwrap();
    assert_eq!(args.command, Command::Version);
}

#[test]
fn test_combined_flags() {
    let args = Args::parse_from(&[
        "check".to_string(),
        "--hardware".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--verbose".to_string(),
        "--parallel".to_string(),
        "--timeout".to_string(),
        "60000".to_string(),
    ])
    .unwrap();

    assert_eq!(args.command, Command::Check);
    assert_eq!(args.category, CategoryFilter::Hardware);
    assert_eq!(args.format, OutputFormat::Json);
    assert!(args.verbose);
    assert!(args.parallel);
    assert_eq!(args.timeout_ms, 60000);
}

#[test]
fn test_unknown_option_error() {
    let result = Args::parse_from(&["--unknown".to_string()]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown option"));
}

#[test]
fn test_invalid_format_error() {
    let result = Args::parse_from(&["--format".to_string(), "invalid".to_string()]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown output format"));
}

#[test]
fn test_missing_timeout_value_error() {
    let result = Args::parse_from(&["--timeout".to_string()]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires"));
}

#[test]
fn test_invalid_timeout_value_error() {
    let result = Args::parse_from(&["--timeout".to_string(), "not_a_number".to_string()]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid timeout"));
}

#[test]
fn test_missing_skip_value_error() {
    let result = Args::parse_from(&["--skip".to_string()]);
    assert!(result.is_err());
}

#[test]
fn test_missing_only_value_error() {
    let result = Args::parse_from(&["--only".to_string()]);
    assert!(result.is_err());
}

#[test]
fn test_missing_format_value_error() {
    let result = Args::parse_from(&["--format".to_string()]);
    assert!(result.is_err());
}
