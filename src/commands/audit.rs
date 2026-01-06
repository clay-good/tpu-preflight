//! Configuration audit command
//!
//! Audits XLA, JAX, and system configuration for potential issues.

use crate::cli::args::{Args, OutputFormat};
use crate::TpuDocError;
use std::env;
use std::process::Command;

/// Audit result
#[derive(Debug)]
pub struct AuditResult {
    pub xla_audit: XlaAudit,
    pub jax_audit: JaxAudit,
    pub memory_audit: MemoryAudit,
    pub distributed_audit: DistributedAudit,
    pub logging_audit: LoggingAudit,
    pub overall_status: AuditStatus,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditStatus {
    Optimal,
    SubOptimal,
    Warning,
    Error,
}

#[derive(Debug)]
pub struct XlaAudit {
    pub xla_flags: Option<String>,
    pub issues: Vec<AuditIssue>,
    pub status: AuditStatus,
}

#[derive(Debug)]
pub struct JaxAudit {
    pub config: Vec<(String, String)>,
    pub issues: Vec<AuditIssue>,
    pub status: AuditStatus,
}

#[derive(Debug)]
pub struct MemoryAudit {
    pub preallocate: Option<String>,
    pub mem_fraction: Option<String>,
    pub issues: Vec<AuditIssue>,
    pub status: AuditStatus,
}

#[derive(Debug)]
pub struct DistributedAudit {
    pub is_multi_host: bool,
    pub coordinator_address: Option<String>,
    pub task_id: Option<String>,
    pub issues: Vec<AuditIssue>,
    pub status: AuditStatus,
}

#[derive(Debug)]
pub struct LoggingAudit {
    pub tf_log_level: Option<String>,
    pub jax_debug_nans: Option<String>,
    pub issues: Vec<AuditIssue>,
    pub status: AuditStatus,
}

#[derive(Debug)]
pub struct AuditIssue {
    pub check_id: String,
    pub severity: IssueSeverity,
    pub description: String,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

/// Run the audit command
pub fn run(args: &Args) -> Result<String, TpuDocError> {
    let result = run_audit();

    match args.format {
        OutputFormat::Json => Ok(format_json(&result)),
        _ => Ok(format_text(&result, args.verbose)),
    }
}

fn run_audit() -> AuditResult {
    let mut recommendations = Vec::new();

    // CFG-001: XLA Flags Audit
    let xla_audit = audit_xla_flags();

    // CFG-002: JAX Configuration Audit
    let jax_audit = audit_jax_config();

    // CFG-003: Memory Preallocation Check
    let memory_audit = audit_memory_config();

    // CFG-004: Distributed Configuration Check
    let distributed_audit = audit_distributed_config();

    // CFG-005: Logging Configuration Check
    let logging_audit = audit_logging_config();

    // Collect all recommendations
    for issue in &xla_audit.issues {
        if let Some(ref rec) = issue.recommendation {
            recommendations.push(rec.clone());
        }
    }
    for issue in &jax_audit.issues {
        if let Some(ref rec) = issue.recommendation {
            recommendations.push(rec.clone());
        }
    }
    for issue in &memory_audit.issues {
        if let Some(ref rec) = issue.recommendation {
            recommendations.push(rec.clone());
        }
    }
    for issue in &distributed_audit.issues {
        if let Some(ref rec) = issue.recommendation {
            recommendations.push(rec.clone());
        }
    }
    for issue in &logging_audit.issues {
        if let Some(ref rec) = issue.recommendation {
            recommendations.push(rec.clone());
        }
    }

    // Determine overall status
    let all_statuses = [
        &xla_audit.status,
        &jax_audit.status,
        &memory_audit.status,
        &distributed_audit.status,
        &logging_audit.status,
    ];

    let overall_status = if all_statuses.iter().any(|s| **s == AuditStatus::Error) {
        AuditStatus::Error
    } else if all_statuses.iter().any(|s| **s == AuditStatus::Warning) {
        AuditStatus::Warning
    } else if all_statuses.iter().any(|s| **s == AuditStatus::SubOptimal) {
        AuditStatus::SubOptimal
    } else {
        AuditStatus::Optimal
    };

    AuditResult {
        xla_audit,
        jax_audit,
        memory_audit,
        distributed_audit,
        logging_audit,
        overall_status,
        recommendations,
    }
}

fn audit_xla_flags() -> XlaAudit {
    let mut issues = Vec::new();
    let xla_flags = env::var("XLA_FLAGS").ok();

    if let Some(ref flags) = xla_flags {
        // Check for debug flags in production
        let debug_patterns = [
            "--xla_dump_to",
            "--xla_dump_hlo",
            "--xla_log_all",
            "--xla_dump_hlo_as_text",
        ];

        for pattern in &debug_patterns {
            if flags.contains(pattern) {
                issues.push(AuditIssue {
                    check_id: "CFG-001".to_string(),
                    severity: IssueSeverity::Warning,
                    description: format!("Debug flag {} is set (performance impact)", pattern),
                    recommendation: Some(format!("Remove {} for production workloads", pattern)),
                });
            }
        }

        // Check for disabled optimizations
        if flags.contains("--xla_disable_hlo_passes") {
            issues.push(AuditIssue {
                check_id: "CFG-001".to_string(),
                severity: IssueSeverity::Warning,
                description: "HLO passes are disabled".to_string(),
                recommendation: Some("Enable HLO passes for optimal performance".to_string()),
            });
        }

        // Check for experimental flags
        if flags.contains("--xla_experimental") {
            issues.push(AuditIssue {
                check_id: "CFG-001".to_string(),
                severity: IssueSeverity::Info,
                description: "Experimental XLA flags are set".to_string(),
                recommendation: Some("Review experimental flags for stability".to_string()),
            });
        }
    } else {
        issues.push(AuditIssue {
            check_id: "CFG-001".to_string(),
            severity: IssueSeverity::Info,
            description: "XLA_FLAGS not set (using defaults)".to_string(),
            recommendation: None,
        });
    }

    let status = if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Error)) {
        AuditStatus::Error
    } else if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Warning)) {
        AuditStatus::Warning
    } else {
        AuditStatus::Optimal
    };

    XlaAudit {
        xla_flags,
        issues,
        status,
    }
}

fn audit_jax_config() -> JaxAudit {
    let mut issues = Vec::new();
    let mut config = Vec::new();

    // Try to get JAX config via Python
    if let Ok(output) = Command::new("python3")
        .args(["-c", "import jax; print(jax.config.jax_enable_x64); print(jax.config.jax_default_matmul_precision)"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();

            if lines.len() >= 2 {
                config.push(("jax_enable_x64".to_string(), lines[0].to_string()));
                config.push(("jax_default_matmul_precision".to_string(), lines[1].to_string()));

                // Check x64 mode
                if lines[0] == "True" {
                    issues.push(AuditIssue {
                        check_id: "CFG-002".to_string(),
                        severity: IssueSeverity::Info,
                        description: "x64 mode is enabled (2x memory for floats)".to_string(),
                        recommendation: Some("Use float32 for TPU efficiency unless float64 is required".to_string()),
                    });
                }

                // Check matmul precision
                if lines[1] != "default" && lines[1] != "highest" {
                    issues.push(AuditIssue {
                        check_id: "CFG-002".to_string(),
                        severity: IssueSeverity::Info,
                        description: format!("Non-default matmul precision: {}", lines[1]),
                        recommendation: None,
                    });
                }
            }
        }
    }

    // Check JAX platform
    if let Ok(platforms) = env::var("JAX_PLATFORMS") {
        config.push(("JAX_PLATFORMS".to_string(), platforms.clone()));
        if !platforms.contains("tpu") {
            issues.push(AuditIssue {
                check_id: "CFG-002".to_string(),
                severity: IssueSeverity::Warning,
                description: "JAX_PLATFORMS does not include 'tpu'".to_string(),
                recommendation: Some("Set JAX_PLATFORMS=tpu,cpu for TPU workloads".to_string()),
            });
        }
    }

    let status = if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Error)) {
        AuditStatus::Error
    } else if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Warning)) {
        AuditStatus::Warning
    } else {
        AuditStatus::Optimal
    };

    JaxAudit {
        config,
        issues,
        status,
    }
}

fn audit_memory_config() -> MemoryAudit {
    let mut issues = Vec::new();

    let preallocate = env::var("XLA_PYTHON_CLIENT_PREALLOCATE").ok();
    let mem_fraction = env::var("XLA_PYTHON_CLIENT_MEM_FRACTION").ok();

    // Check preallocation setting
    if let Some(ref val) = preallocate {
        if val == "false" {
            issues.push(AuditIssue {
                check_id: "CFG-003".to_string(),
                severity: IssueSeverity::Info,
                description: "Memory preallocation is disabled".to_string(),
                recommendation: Some("Preallocation can improve performance for fixed-size models".to_string()),
            });
        }
    }

    // Check memory fraction
    if let Some(ref val) = mem_fraction {
        if let Ok(fraction) = val.parse::<f64>() {
            if fraction > 0.95 {
                issues.push(AuditIssue {
                    check_id: "CFG-003".to_string(),
                    severity: IssueSeverity::Warning,
                    description: format!("High memory fraction: {} (risk of OOM)", fraction),
                    recommendation: Some("Consider lowering XLA_PYTHON_CLIENT_MEM_FRACTION to 0.9".to_string()),
                });
            } else if fraction < 0.5 {
                issues.push(AuditIssue {
                    check_id: "CFG-003".to_string(),
                    severity: IssueSeverity::Info,
                    description: format!("Low memory fraction: {} (underutilization)", fraction),
                    recommendation: Some("Consider increasing for larger models".to_string()),
                });
            }
        }
    }

    let status = if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Error)) {
        AuditStatus::Error
    } else if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Warning)) {
        AuditStatus::Warning
    } else if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Info)) {
        AuditStatus::SubOptimal
    } else {
        AuditStatus::Optimal
    };

    MemoryAudit {
        preallocate,
        mem_fraction,
        issues,
        status,
    }
}

fn audit_distributed_config() -> DistributedAudit {
    let mut issues = Vec::new();

    let coordinator_address = env::var("JAX_COORDINATOR_ADDRESS").ok();
    let task_id = env::var("CLOUD_TPU_TASK_ID").ok();
    let worker_hostnames = env::var("TPU_WORKER_HOSTNAMES").ok();

    // Determine if multi-host
    let is_multi_host = coordinator_address.is_some() ||
        worker_hostnames.as_ref().map(|h| h.contains(',')).unwrap_or(false);

    if is_multi_host {
        // Check coordinator address
        if coordinator_address.is_none() {
            issues.push(AuditIssue {
                check_id: "CFG-004".to_string(),
                severity: IssueSeverity::Error,
                description: "Multi-host detected but JAX_COORDINATOR_ADDRESS not set".to_string(),
                recommendation: Some("Set JAX_COORDINATOR_ADDRESS for distributed training".to_string()),
            });
        }

        // Check task ID
        if task_id.is_none() {
            issues.push(AuditIssue {
                check_id: "CFG-004".to_string(),
                severity: IssueSeverity::Warning,
                description: "CLOUD_TPU_TASK_ID not set for multi-host".to_string(),
                recommendation: Some("Ensure CLOUD_TPU_TASK_ID is set correctly".to_string()),
            });
        }
    }

    let status = if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Error)) {
        AuditStatus::Error
    } else if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Warning)) {
        AuditStatus::Warning
    } else {
        AuditStatus::Optimal
    };

    DistributedAudit {
        is_multi_host,
        coordinator_address,
        task_id,
        issues,
        status,
    }
}

fn audit_logging_config() -> LoggingAudit {
    let mut issues = Vec::new();

    let tf_log_level = env::var("TF_CPP_MIN_LOG_LEVEL").ok();
    let jax_debug_nans = env::var("JAX_DEBUG_NANS").ok();

    // Check TensorFlow log level
    if let Some(ref level) = tf_log_level {
        match level.as_str() {
            "0" => {
                issues.push(AuditIssue {
                    check_id: "CFG-005".to_string(),
                    severity: IssueSeverity::Warning,
                    description: "TF_CPP_MIN_LOG_LEVEL=0 (verbose logging)".to_string(),
                    recommendation: Some("Set TF_CPP_MIN_LOG_LEVEL=2 for production".to_string()),
                });
            }
            "3" => {
                issues.push(AuditIssue {
                    check_id: "CFG-005".to_string(),
                    severity: IssueSeverity::Info,
                    description: "TF logging is suppressed (level 3)".to_string(),
                    recommendation: Some("Lower level for debugging if issues occur".to_string()),
                });
            }
            _ => {}
        }
    }

    // Check JAX debug NaNs
    if let Some(ref debug_nans) = jax_debug_nans {
        if debug_nans == "True" || debug_nans == "1" {
            issues.push(AuditIssue {
                check_id: "CFG-005".to_string(),
                severity: IssueSeverity::Warning,
                description: "JAX_DEBUG_NANS is enabled (performance impact)".to_string(),
                recommendation: Some("Disable JAX_DEBUG_NANS for production".to_string()),
            });
        }
    }

    // Check for other debug settings
    if env::var("JAX_TRACEBACK_FILTERING").ok().as_deref() == Some("off") {
        issues.push(AuditIssue {
            check_id: "CFG-005".to_string(),
            severity: IssueSeverity::Info,
            description: "JAX traceback filtering is disabled".to_string(),
            recommendation: None,
        });
    }

    let status = if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Error)) {
        AuditStatus::Error
    } else if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Warning)) {
        AuditStatus::Warning
    } else {
        AuditStatus::Optimal
    };

    LoggingAudit {
        tf_log_level,
        jax_debug_nans,
        issues,
        status,
    }
}

fn format_text(result: &AuditResult, verbose: bool) -> String {
    let mut output = String::new();

    output.push_str("================================================================================\n");
    output.push_str("                         CONFIGURATION AUDIT\n");
    output.push_str("================================================================================\n\n");

    // Overall status
    let status_str = match result.overall_status {
        AuditStatus::Optimal => "OPTIMAL",
        AuditStatus::SubOptimal => "SUBOPTIMAL",
        AuditStatus::Warning => "WARNING",
        AuditStatus::Error => "ERROR",
    };
    output.push_str(&format!("Overall Status: {}\n\n", status_str));

    // CFG-001: XLA Flags
    output.push_str("CFG-001: XLA FLAGS AUDIT\n");
    output.push_str("------------------------\n");
    if let Some(ref flags) = result.xla_audit.xla_flags {
        let display_flags = if flags.len() > 60 && !verbose {
            format!("{}...", &flags[..60])
        } else {
            flags.clone()
        };
        output.push_str(&format!("  XLA_FLAGS: {}\n", display_flags));
    } else {
        output.push_str("  XLA_FLAGS: (not set)\n");
    }
    for issue in &result.xla_audit.issues {
        output.push_str(&format_issue(issue));
    }
    output.push('\n');

    // CFG-002: JAX Configuration
    output.push_str("CFG-002: JAX CONFIGURATION AUDIT\n");
    output.push_str("--------------------------------\n");
    for (key, value) in &result.jax_audit.config {
        output.push_str(&format!("  {}: {}\n", key, value));
    }
    for issue in &result.jax_audit.issues {
        output.push_str(&format_issue(issue));
    }
    output.push('\n');

    // CFG-003: Memory Configuration
    output.push_str("CFG-003: MEMORY CONFIGURATION\n");
    output.push_str("-----------------------------\n");
    output.push_str(&format!("  XLA_PYTHON_CLIENT_PREALLOCATE: {}\n",
        result.memory_audit.preallocate.as_deref().unwrap_or("(not set)")));
    output.push_str(&format!("  XLA_PYTHON_CLIENT_MEM_FRACTION: {}\n",
        result.memory_audit.mem_fraction.as_deref().unwrap_or("(not set)")));
    for issue in &result.memory_audit.issues {
        output.push_str(&format_issue(issue));
    }
    output.push('\n');

    // CFG-004: Distributed Configuration
    output.push_str("CFG-004: DISTRIBUTED CONFIGURATION\n");
    output.push_str("----------------------------------\n");
    output.push_str(&format!("  Multi-host: {}\n", if result.distributed_audit.is_multi_host { "Yes" } else { "No" }));
    if result.distributed_audit.is_multi_host {
        output.push_str(&format!("  JAX_COORDINATOR_ADDRESS: {}\n",
            result.distributed_audit.coordinator_address.as_deref().unwrap_or("(not set)")));
        output.push_str(&format!("  CLOUD_TPU_TASK_ID: {}\n",
            result.distributed_audit.task_id.as_deref().unwrap_or("(not set)")));
    }
    for issue in &result.distributed_audit.issues {
        output.push_str(&format_issue(issue));
    }
    output.push('\n');

    // CFG-005: Logging Configuration
    output.push_str("CFG-005: LOGGING CONFIGURATION\n");
    output.push_str("------------------------------\n");
    output.push_str(&format!("  TF_CPP_MIN_LOG_LEVEL: {}\n",
        result.logging_audit.tf_log_level.as_deref().unwrap_or("(not set)")));
    output.push_str(&format!("  JAX_DEBUG_NANS: {}\n",
        result.logging_audit.jax_debug_nans.as_deref().unwrap_or("(not set)")));
    for issue in &result.logging_audit.issues {
        output.push_str(&format_issue(issue));
    }
    output.push('\n');

    // Recommendations
    if !result.recommendations.is_empty() {
        output.push_str("RECOMMENDATIONS\n");
        output.push_str("---------------\n");
        for rec in &result.recommendations {
            output.push_str(&format!("  * {}\n", rec));
        }
        output.push('\n');
    }

    output.push_str("================================================================================\n");

    output
}

fn format_issue(issue: &AuditIssue) -> String {
    let icon = match issue.severity {
        IssueSeverity::Error => "[ERROR]",
        IssueSeverity::Warning => "[WARN] ",
        IssueSeverity::Info => "[INFO] ",
    };
    format!("  {} {}\n", icon, issue.description)
}

fn format_json(result: &AuditResult) -> String {
    let mut json = String::new();
    json.push_str("{\n");

    json.push_str(&format!("  \"overall_status\": \"{:?}\",\n", result.overall_status));

    // XLA Audit
    json.push_str("  \"xla_audit\": {\n");
    json.push_str(&format!("    \"xla_flags\": {},\n",
        result.xla_audit.xla_flags.as_ref().map(|f| format!("\"{}\"", f.replace('\"', "\\\""))).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"status\": \"{:?}\",\n", result.xla_audit.status));
    json.push_str("    \"issues\": ");
    json.push_str(&format_issues_json(&result.xla_audit.issues));
    json.push_str("\n  },\n");

    // JAX Audit
    json.push_str("  \"jax_audit\": {\n");
    json.push_str(&format!("    \"status\": \"{:?}\",\n", result.jax_audit.status));
    json.push_str("    \"config\": {\n");
    for (i, (key, value)) in result.jax_audit.config.iter().enumerate() {
        let comma = if i < result.jax_audit.config.len() - 1 { "," } else { "" };
        json.push_str(&format!("      \"{}\": \"{}\"{}\n", key, value, comma));
    }
    json.push_str("    },\n");
    json.push_str("    \"issues\": ");
    json.push_str(&format_issues_json(&result.jax_audit.issues));
    json.push_str("\n  },\n");

    // Memory Audit
    json.push_str("  \"memory_audit\": {\n");
    json.push_str(&format!("    \"preallocate\": {},\n",
        result.memory_audit.preallocate.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"mem_fraction\": {},\n",
        result.memory_audit.mem_fraction.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"status\": \"{:?}\",\n", result.memory_audit.status));
    json.push_str("    \"issues\": ");
    json.push_str(&format_issues_json(&result.memory_audit.issues));
    json.push_str("\n  },\n");

    // Distributed Audit
    json.push_str("  \"distributed_audit\": {\n");
    json.push_str(&format!("    \"is_multi_host\": {},\n", result.distributed_audit.is_multi_host));
    json.push_str(&format!("    \"coordinator_address\": {},\n",
        result.distributed_audit.coordinator_address.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"task_id\": {},\n",
        result.distributed_audit.task_id.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"status\": \"{:?}\",\n", result.distributed_audit.status));
    json.push_str("    \"issues\": ");
    json.push_str(&format_issues_json(&result.distributed_audit.issues));
    json.push_str("\n  },\n");

    // Logging Audit
    json.push_str("  \"logging_audit\": {\n");
    json.push_str(&format!("    \"tf_log_level\": {},\n",
        result.logging_audit.tf_log_level.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"jax_debug_nans\": {},\n",
        result.logging_audit.jax_debug_nans.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"status\": \"{:?}\",\n", result.logging_audit.status));
    json.push_str("    \"issues\": ");
    json.push_str(&format_issues_json(&result.logging_audit.issues));
    json.push_str("\n  },\n");

    // Recommendations
    json.push_str("  \"recommendations\": [\n");
    for (i, rec) in result.recommendations.iter().enumerate() {
        let comma = if i < result.recommendations.len() - 1 { "," } else { "" };
        json.push_str(&format!("    \"{}\"{}\n", rec, comma));
    }
    json.push_str("  ]\n");

    json.push_str("}\n");
    json
}

fn format_issues_json(issues: &[AuditIssue]) -> String {
    if issues.is_empty() {
        return "[]".to_string();
    }

    let mut json = String::new();
    json.push_str("[\n");
    for (i, issue) in issues.iter().enumerate() {
        json.push_str("      {\n");
        json.push_str(&format!("        \"check_id\": \"{}\",\n", issue.check_id));
        json.push_str(&format!("        \"severity\": \"{:?}\",\n", issue.severity));
        json.push_str(&format!("        \"description\": \"{}\",\n", issue.description));
        json.push_str(&format!("        \"recommendation\": {}\n",
            issue.recommendation.as_ref().map(|r| format!("\"{}\"", r)).unwrap_or_else(|| "null".to_string())));
        json.push_str("      }");
        if i < issues.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("    ]");
    json
}
