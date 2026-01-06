//! Zero-dependency argument parser for tpu-doc.
//!
//! Handles command line argument parsing without external dependencies.

use std::env;

#[cfg(feature = "ai")]
use crate::ai::AiProvider;

// When ai feature is not enabled, provide a stub
#[cfg(not(feature = "ai"))]
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AiProvider {
    #[default]
    Anthropic,
    Google,
}

#[cfg(not(feature = "ai"))]
impl AiProvider {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "anthropic" | "claude" => Ok(AiProvider::Anthropic),
            "google" | "gemini" => Ok(AiProvider::Google),
            _ => Err(format!(
                "Unknown AI provider: '{}'. Valid providers: anthropic, google",
                s
            )),
        }
    }
}

/// Command to execute
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Command {
    /// Run validation checks (default)
    #[default]
    Check,
    /// Print version information
    Version,
    /// List all available checks
    List,
    /// Display environment information
    Info,
    /// Analyze software stack compatibility
    Stack,
    /// Analyze XLA cache
    Cache,
    /// Capture resource snapshot
    Snapshot,
    /// Run configuration audit
    Audit,
    /// AI-powered log analysis
    Analyze,
}

/// Output format selection
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OutputFormat {
    /// Human-readable terminal output
    #[default]
    Text,
    /// Machine-readable JSON
    Json,
    /// JUnit XML for CI/CD integration
    Junit,
}

impl OutputFormat {
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "junit" => Ok(OutputFormat::Junit),
            _ => Err(format!("Unknown output format: '{}'. Valid formats: text, json, junit", s)),
        }
    }
}

/// Check category filter
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CategoryFilter {
    /// Run all checks
    #[default]
    All,
    /// Run only hardware checks
    Hardware,
    /// Run only stack checks
    Stack,
    /// Run only performance checks
    Performance,
    /// Run only I/O checks
    Io,
    /// Run only security checks
    Security,
    /// Run only configuration audit checks
    Config,
}

/// Parsed command line arguments
#[derive(Debug, Clone)]
pub struct Args {
    /// Command to execute
    pub command: Command,
    /// Category filter
    pub category: CategoryFilter,
    /// Specific checks to skip (by ID)
    pub skip: Vec<String>,
    /// Specific checks to run (by ID)
    pub only: Vec<String>,
    /// Output format
    pub format: OutputFormat,
    /// Quiet mode (only failures and warnings)
    pub quiet: bool,
    /// Verbose mode (detailed diagnostics)
    pub verbose: bool,
    /// Disable colored output
    pub no_color: bool,
    /// Global timeout in milliseconds
    pub timeout_ms: u64,
    /// Run checks in parallel
    pub parallel: bool,
    /// Stop on first failure
    pub fail_fast: bool,
    /// Configuration file path
    pub config: Option<String>,
    /// Baseline file path for comparison
    pub baseline: Option<String>,
    /// Show help
    pub help: bool,
    /// Show compatibility matrix (for stack command)
    pub show_matrix: bool,
    /// Continuous refresh interval in seconds (for snapshot command)
    pub continuous: u32,
    /// Enable AI-powered analysis (for analyze command)
    pub ai_enabled: bool,
    /// AI provider to use
    pub ai_provider: Option<AiProvider>,
    /// AI model to use
    pub ai_model: Option<String>,
    /// User question for AI analysis
    pub ai_question: Option<String>,
    /// Log file path (for analyze command)
    pub log_file: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            command: Command::default(),
            category: CategoryFilter::default(),
            skip: Vec::new(),
            only: Vec::new(),
            format: OutputFormat::default(),
            quiet: false,
            verbose: false,
            no_color: false,
            timeout_ms: 30000,
            parallel: false,
            fail_fast: false,
            config: None,
            baseline: None,
            help: false,
            show_matrix: false,
            continuous: 0,
            ai_enabled: false,
            ai_provider: None,
            ai_model: None,
            ai_question: None,
            log_file: None,
        }
    }
}

impl Args {
    /// Parse command line arguments from std::env::args()
    pub fn parse() -> Result<Self, String> {
        let args: Vec<String> = env::args().collect();
        Self::parse_from(&args[1..])
    }

    /// Parse command line arguments from a slice (for testing)
    pub fn parse_from(args: &[String]) -> Result<Self, String> {
        let mut result = Args::default();
        let mut i = 0;

        // Check for NO_COLOR environment variable
        if env::var("NO_COLOR").is_ok() {
            result.no_color = true;
        }

        // Check for environment variable overrides
        if let Ok(format) = env::var("TPU_DOC_FORMAT") {
            result.format = OutputFormat::from_str(&format)?;
        }
        if env::var("TPU_DOC_VERBOSE").is_ok() {
            result.verbose = true;
        }
        if let Ok(config) = env::var("TPU_DOC_CONFIG") {
            result.config = Some(config);
        }

        while i < args.len() {
            let arg = &args[i];

            match arg.as_str() {
                // Commands
                "check" => result.command = Command::Check,
                "version" => result.command = Command::Version,
                "list" => result.command = Command::List,
                "info" => result.command = Command::Info,
                "stack" => result.command = Command::Stack,
                "cache" => result.command = Command::Cache,
                "snapshot" => result.command = Command::Snapshot,
                "audit" => result.command = Command::Audit,
                "analyze" => result.command = Command::Analyze,

                // Help flags
                "-h" | "--help" => result.help = true,
                "-V" | "--version" => result.command = Command::Version,

                // Category filters
                "--all" => result.category = CategoryFilter::All,
                "--hardware" => result.category = CategoryFilter::Hardware,
                "--stack" => result.category = CategoryFilter::Stack,
                "--performance" => result.category = CategoryFilter::Performance,
                "--io" => result.category = CategoryFilter::Io,
                "--security" => result.category = CategoryFilter::Security,
                "--config-audit" => result.category = CategoryFilter::Config,

                // Skip/only with value
                "--skip" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--skip requires a check ID".to_string());
                    }
                    result.skip.push(args[i].clone());
                }
                "--only" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--only requires a check ID".to_string());
                    }
                    result.only.push(args[i].clone());
                }

                // Output options
                "--format" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--format requires a format name".to_string());
                    }
                    result.format = OutputFormat::from_str(&args[i])?;
                }
                "-q" | "--quiet" => result.quiet = true,
                "-v" | "--verbose" => result.verbose = true,
                "--no-color" => result.no_color = true,

                // Behavior options
                "--timeout" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--timeout requires a value in milliseconds".to_string());
                    }
                    result.timeout_ms = args[i]
                        .parse()
                        .map_err(|_| format!("Invalid timeout value: '{}'", args[i]))?;
                }
                "--parallel" => result.parallel = true,
                "--fail-fast" => result.fail_fast = true,

                // Configuration options
                "--config" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--config requires a file path".to_string());
                    }
                    result.config = Some(args[i].clone());
                }
                "--baseline" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--baseline requires a file path".to_string());
                    }
                    result.baseline = Some(args[i].clone());
                }

                // Stack command options
                "--matrix" => result.show_matrix = true,

                // Snapshot command options
                "--continuous" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--continuous requires refresh interval in seconds".to_string());
                    }
                    result.continuous = args[i]
                        .parse()
                        .map_err(|_| format!("Invalid continuous value: '{}'", args[i]))?;
                }

                // AI analyze command options
                "--ai" => result.ai_enabled = true,
                "--provider" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--provider requires a provider name (anthropic, google)".to_string());
                    }
                    result.ai_provider = Some(AiProvider::from_str(&args[i])?);
                }
                "--model" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--model requires a model name".to_string());
                    }
                    result.ai_model = Some(args[i].clone());
                }
                "--question" => {
                    i += 1;
                    if i >= args.len() {
                        return Err("--question requires a question string".to_string());
                    }
                    result.ai_question = Some(args[i].clone());
                }

                // Handle --option=value syntax
                _ if arg.starts_with("--skip=") => {
                    result.skip.push(arg[7..].to_string());
                }
                _ if arg.starts_with("--only=") => {
                    result.only.push(arg[7..].to_string());
                }
                _ if arg.starts_with("--format=") => {
                    let format = &arg[9..];
                    result.format = OutputFormat::from_str(format)?;
                }
                _ if arg.starts_with("--timeout=") => {
                    result.timeout_ms = arg[10..]
                        .parse()
                        .map_err(|_| format!("Invalid timeout value: '{}'", &arg[10..]))?;
                }
                _ if arg.starts_with("--config=") => {
                    result.config = Some(arg[9..].to_string());
                }
                _ if arg.starts_with("--baseline=") => {
                    result.baseline = Some(arg[11..].to_string());
                }
                _ if arg.starts_with("--continuous=") => {
                    result.continuous = arg[13..]
                        .parse()
                        .map_err(|_| format!("Invalid continuous value: '{}'", &arg[13..]))?;
                }
                _ if arg.starts_with("--provider=") => {
                    result.ai_provider = Some(AiProvider::from_str(&arg[11..])?);
                }
                _ if arg.starts_with("--model=") => {
                    result.ai_model = Some(arg[8..].to_string());
                }
                _ if arg.starts_with("--question=") => {
                    result.ai_question = Some(arg[11..].to_string());
                }

                // Unknown argument
                _ if arg.starts_with('-') => {
                    return Err(format!("Unknown option: '{}'", arg));
                }
                // Positional arguments (like log file for analyze command)
                _ => {
                    if result.command == Command::Analyze && result.log_file.is_none() {
                        result.log_file = Some(arg.clone());
                    } else {
                        return Err(format!("Unexpected argument: '{}'", arg));
                    }
                }
            }

            i += 1;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = Args::default();
        assert_eq!(args.command, Command::Check);
        assert_eq!(args.category, CategoryFilter::All);
        assert_eq!(args.format, OutputFormat::Text);
        assert!(!args.quiet);
        assert!(!args.verbose);
        assert_eq!(args.timeout_ms, 30000);
    }

    #[test]
    fn test_parse_version_command() {
        let args = Args::parse_from(&["version".to_string()]).unwrap();
        assert_eq!(args.command, Command::Version);
    }

    #[test]
    fn test_parse_list_command() {
        let args = Args::parse_from(&["list".to_string()]).unwrap();
        assert_eq!(args.command, Command::List);
    }

    #[test]
    fn test_parse_info_command() {
        let args = Args::parse_from(&["info".to_string()]).unwrap();
        assert_eq!(args.command, Command::Info);
    }

    #[test]
    fn test_parse_stack_command() {
        let args = Args::parse_from(&["stack".to_string()]).unwrap();
        assert_eq!(args.command, Command::Stack);
    }

    #[test]
    fn test_parse_cache_command() {
        let args = Args::parse_from(&["cache".to_string()]).unwrap();
        assert_eq!(args.command, Command::Cache);
    }

    #[test]
    fn test_parse_snapshot_command() {
        let args = Args::parse_from(&["snapshot".to_string()]).unwrap();
        assert_eq!(args.command, Command::Snapshot);
    }

    #[test]
    fn test_parse_audit_command() {
        let args = Args::parse_from(&["audit".to_string()]).unwrap();
        assert_eq!(args.command, Command::Audit);
    }

    #[test]
    fn test_parse_category_filter() {
        let args = Args::parse_from(&["--hardware".to_string()]).unwrap();
        assert_eq!(args.category, CategoryFilter::Hardware);
    }

    #[test]
    fn test_parse_config_audit_filter() {
        let args = Args::parse_from(&["--config-audit".to_string()]).unwrap();
        assert_eq!(args.category, CategoryFilter::Config);
    }

    #[test]
    fn test_parse_skip_option() {
        let args = Args::parse_from(&["--skip".to_string(), "HW-001".to_string()]).unwrap();
        assert_eq!(args.skip, vec!["HW-001"]);
    }

    #[test]
    fn test_parse_format_option() {
        let args = Args::parse_from(&["--format".to_string(), "json".to_string()]).unwrap();
        assert_eq!(args.format, OutputFormat::Json);
    }

    #[test]
    fn test_parse_timeout_option() {
        let args = Args::parse_from(&["--timeout".to_string(), "60000".to_string()]).unwrap();
        assert_eq!(args.timeout_ms, 60000);
    }

    #[test]
    fn test_parse_matrix_option() {
        let args = Args::parse_from(&["stack".to_string(), "--matrix".to_string()]).unwrap();
        assert_eq!(args.command, Command::Stack);
        assert!(args.show_matrix);
    }

    #[test]
    fn test_parse_continuous_option() {
        let args = Args::parse_from(&["snapshot".to_string(), "--continuous".to_string(), "5".to_string()]).unwrap();
        assert_eq!(args.command, Command::Snapshot);
        assert_eq!(args.continuous, 5);
    }

    #[test]
    fn test_parse_unknown_option() {
        let result = Args::parse_from(&["--unknown".to_string()]);
        assert!(result.is_err());
    }
}
