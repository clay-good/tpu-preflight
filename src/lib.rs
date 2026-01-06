//! tpu-doc library
//!
//! TPU environment diagnostics, discovery, and troubleshooting tool.
//!
//! This library provides comprehensive functionality for TPU environments:
//! - Validation checks for hardware health, software stack, performance, I/O, and security
//! - Environment discovery and fingerprinting
//! - XLA cache analysis
//! - Resource utilization snapshots
//! - Configuration auditing
//! - AI-powered log analysis (optional, requires "ai" feature)
//!
//! # Example
//!
//! ```no_run
//! use tpu_doc::{run_checks, TpuDocConfig};
//!
//! let config = TpuDocConfig::default();
//! let report = run_checks(config).expect("Validation failed");
//! println!("Checks passed: {}", report.summary().passed);
//! ```

pub mod ai;
pub mod checks;
pub mod cli;
pub mod commands;
pub mod data;
pub mod engine;
pub mod platform;
pub mod version;

use cli::args::{Args, CategoryFilter};
use engine::orchestrator::{create_all_checks, CheckOrchestrator, OrchestratorConfig};
use engine::result::ValidationReport;
use std::fmt;

// Re-exports for public API
pub use engine::orchestrator::CheckOrchestrator as Orchestrator;
pub use engine::result::{ResultSummary, ValidationReport as Report};

/// Check result indicating the outcome of a validation check.
#[derive(Debug, Clone)]
pub enum CheckResult {
    /// Check passed successfully
    Pass {
        message: String,
        duration_ms: u64,
    },
    /// Check passed with warnings
    Warn {
        message: String,
        details: String,
        duration_ms: u64,
    },
    /// Check failed
    Fail {
        message: String,
        details: String,
        duration_ms: u64,
    },
    /// Check was skipped
    Skip {
        reason: String,
    },
}

impl fmt::Display for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckResult::Pass { message, .. } => write!(f, "PASS: {}", message),
            CheckResult::Warn { message, details, .. } => {
                write!(f, "WARN: {} ({})", message, details)
            }
            CheckResult::Fail { message, details, .. } => {
                write!(f, "FAIL: {} ({})", message, details)
            }
            CheckResult::Skip { reason } => write!(f, "SKIP: {}", reason),
        }
    }
}

/// Check category for grouping related checks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CheckCategory {
    /// Hardware health checks (TPU devices, memory, thermal)
    Hardware,
    /// Software stack checks (JAX, libtpu, Python versions)
    Stack,
    /// Performance baseline checks (MXU utilization, bandwidth)
    Performance,
    /// I/O throughput checks (GCS, disk, network)
    Io,
    /// Security posture checks (IAM, network exposure)
    Security,
    /// Configuration audit checks (XLA flags, JAX config)
    Config,
}

impl fmt::Display for CheckCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckCategory::Hardware => write!(f, "Hardware"),
            CheckCategory::Stack => write!(f, "Stack"),
            CheckCategory::Performance => write!(f, "Performance"),
            CheckCategory::Io => write!(f, "I/O"),
            CheckCategory::Security => write!(f, "Security"),
            CheckCategory::Config => write!(f, "Config"),
        }
    }
}

/// A validation check with its result.
#[derive(Debug, Clone)]
pub struct Check {
    /// Unique identifier (e.g., "HW-001")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Check category
    pub category: CheckCategory,
    /// Description of what this check validates
    pub description: String,
    /// Result of the check (None if not yet executed)
    pub result: Option<CheckResult>,
}

impl Default for Check {
    fn default() -> Self {
        Check {
            id: String::new(),
            name: String::new(),
            category: CheckCategory::Hardware,
            description: String::new(),
            result: None,
        }
    }
}

/// Error types for tpu-doc operations.
#[derive(Debug, Clone)]
pub enum TpuDocError {
    /// Not running on a TPU VM
    NotOnTpu,
    /// Permission denied for a resource
    PermissionDenied {
        resource: String,
    },
    /// Operation timed out
    Timeout {
        operation: String,
        timeout_ms: u64,
    },
    /// I/O error
    IoError {
        context: String,
        message: String,
    },
    /// Parse error
    ParseError {
        context: String,
        message: String,
    },
    /// Check failed
    CheckFailed {
        check_id: String,
        reason: String,
    },
    /// Command error
    CommandError {
        command: String,
        message: String,
    },
}

impl fmt::Display for TpuDocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TpuDocError::NotOnTpu => {
                write!(f, "Not running on a TPU VM")
            }
            TpuDocError::PermissionDenied { resource } => {
                write!(f, "Permission denied: {}", resource)
            }
            TpuDocError::Timeout { operation, timeout_ms } => {
                write!(f, "Timeout after {}ms: {}", timeout_ms, operation)
            }
            TpuDocError::IoError { context, message } => {
                write!(f, "I/O error in {}: {}", context, message)
            }
            TpuDocError::ParseError { context, message } => {
                write!(f, "Parse error in {}: {}", context, message)
            }
            TpuDocError::CheckFailed { check_id, reason } => {
                write!(f, "Check {} failed: {}", check_id, reason)
            }
            TpuDocError::CommandError { command, message } => {
                write!(f, "Command '{}' error: {}", command, message)
            }
        }
    }
}

impl std::error::Error for TpuDocError {}

/// Configuration for running validation checks.
#[derive(Debug, Clone)]
pub struct TpuDocConfig {
    /// Categories to run (None = all)
    pub categories: Option<Vec<CheckCategory>>,
    /// Specific checks to skip (by ID)
    pub skip_checks: Vec<String>,
    /// Specific checks to run (by ID)
    pub only_checks: Vec<String>,
    /// Run checks in parallel
    pub parallel: bool,
    /// Stop on first failure
    pub fail_fast: bool,
    /// Global timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for TpuDocConfig {
    fn default() -> Self {
        TpuDocConfig {
            categories: None,
            skip_checks: Vec::new(),
            only_checks: Vec::new(),
            parallel: false,
            fail_fast: false,
            timeout_ms: 30000,
        }
    }
}

impl TpuDocConfig {
    /// Create configuration from command line arguments
    pub fn from_args(args: &Args) -> Self {
        let categories = match args.category {
            CategoryFilter::All => None,
            CategoryFilter::Hardware => Some(vec![CheckCategory::Hardware]),
            CategoryFilter::Stack => Some(vec![CheckCategory::Stack]),
            CategoryFilter::Performance => Some(vec![CheckCategory::Performance]),
            CategoryFilter::Io => Some(vec![CheckCategory::Io]),
            CategoryFilter::Security => Some(vec![CheckCategory::Security]),
            CategoryFilter::Config => Some(vec![CheckCategory::Config]),
        };

        TpuDocConfig {
            categories,
            skip_checks: args.skip.clone(),
            only_checks: args.only.clone(),
            parallel: args.parallel,
            fail_fast: args.fail_fast,
            timeout_ms: args.timeout_ms,
        }
    }
}

/// Run validation checks.
///
/// This is the main entry point for running validation checks.
///
/// # Arguments
///
/// * `config` - Configuration specifying which checks to run and how
///
/// # Returns
///
/// Returns a `ValidationReport` containing results of all executed checks,
/// or a `TpuDocError` if there was a problem running the checks.
///
/// # Example
///
/// ```no_run
/// use tpu_doc::{run_checks, TpuDocConfig, CheckCategory};
///
/// // Run only hardware checks
/// let config = TpuDocConfig {
///     categories: Some(vec![CheckCategory::Hardware]),
///     ..Default::default()
/// };
///
/// match run_checks(config) {
///     Ok(report) => {
///         let summary = report.summary();
///         println!("Passed: {}, Failed: {}", summary.passed, summary.failed);
///     }
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn run_checks(config: TpuDocConfig) -> Result<ValidationReport, TpuDocError> {
    // Create orchestrator
    let orch_config = OrchestratorConfig {
        parallel: config.parallel,
        fail_fast: config.fail_fast,
        timeout_ms: config.timeout_ms,
        max_parallel: 4,
    };

    let mut orchestrator = CheckOrchestrator::new(orch_config);

    // Register all checks
    orchestrator.register_checks(create_all_checks());

    // Determine which checks to run
    let report = if !config.only_checks.is_empty() {
        // Run only specified checks
        orchestrator.run_specific(&config.only_checks)
    } else if !config.skip_checks.is_empty() {
        // Run all except skipped
        orchestrator.run_excluding(&config.skip_checks)
    } else if let Some(ref categories) = config.categories {
        // Run specific categories (supports multiple)
        if categories.is_empty() {
            orchestrator.run_all()
        } else {
            orchestrator.run_categories(categories)
        }
    } else {
        // Run all checks
        orchestrator.run_all()
    };

    Ok(report)
}

/// Alias for run_checks (for backward compatibility)
#[deprecated(since = "0.2.0", note = "Use run_checks instead")]
pub fn run_validation(config: TpuDocConfig) -> Result<ValidationReport, TpuDocError> {
    run_checks(config)
}
