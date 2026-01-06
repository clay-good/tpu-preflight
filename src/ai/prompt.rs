//! Prompt construction for AI analysis.
//!
//! This module provides utilities for building effective prompts
//! for AI-powered log analysis, including context from the TPU
//! environment and check results.

use crate::commands::info::EnvironmentInfo;
use crate::engine::result::ValidationReport;
use crate::CheckResult;

/// Maximum log content size (in characters) to include in prompts
const MAX_LOG_SIZE: usize = 100_000;

/// Maximum lines to show from beginning/end when truncating
const TRUNCATE_LINES: usize = 500;

/// System prompt for TPU log analysis
pub const TPU_ANALYSIS_SYSTEM_PROMPT: &str = r#"You are an expert TPU (Tensor Processing Unit) diagnostic assistant for Google Cloud TPU environments. Your role is to analyze logs, error messages, and system state to help users diagnose and resolve issues with their TPU workloads.

You have deep expertise in:
- Google Cloud TPU v4, v5e, v5p, and v6e hardware
- JAX, TensorFlow, and PyTorch TPU backends
- XLA compiler and HLO optimization
- libtpu and PJRT plugin architecture
- Distributed training with TPU pods
- Common TPU error patterns and their resolutions

When analyzing logs:
1. Identify the root cause of any errors
2. Explain what the error means in plain language
3. Provide specific, actionable remediation steps
4. Note any performance concerns or warnings
5. Suggest best practices where relevant

Be concise but thorough. Focus on actionable insights."#;

/// Builder for constructing analysis prompts
#[derive(Debug, Default)]
pub struct PromptBuilder {
    environment_context: Option<String>,
    check_results: Option<String>,
    log_content: Option<String>,
    user_question: Option<String>,
}

impl PromptBuilder {
    /// Create a new prompt builder
    pub fn new() -> Self {
        PromptBuilder::default()
    }

    /// Add environment context from EnvironmentInfo
    pub fn with_environment(mut self, info: &EnvironmentInfo) -> Self {
        self.environment_context = Some(format_environment_context(info));
        self
    }

    /// Add check results from ValidationReport
    pub fn with_check_results(mut self, report: &ValidationReport) -> Self {
        self.check_results = Some(format_check_results(report));
        self
    }

    /// Add log content to analyze
    pub fn with_log_content(mut self, log: &str) -> Self {
        self.log_content = Some(truncate_log_content(log));
        self
    }

    /// Add a specific user question
    pub fn with_question(mut self, question: &str) -> Self {
        self.user_question = Some(question.to_string());
        self
    }

    /// Build the final prompt
    pub fn build(self) -> String {
        let mut prompt = String::new();

        // Start with context sections
        if let Some(env) = self.environment_context {
            prompt.push_str("## TPU Environment\n\n");
            prompt.push_str(&env);
            prompt.push_str("\n\n");
        }

        if let Some(checks) = self.check_results {
            prompt.push_str("## Validation Check Results\n\n");
            prompt.push_str(&checks);
            prompt.push_str("\n\n");
        }

        if let Some(log) = self.log_content {
            prompt.push_str("## Log Content\n\n");
            prompt.push_str("```\n");
            prompt.push_str(&log);
            prompt.push_str("\n```\n\n");
        }

        // Add the question or default request
        prompt.push_str("## Request\n\n");
        if let Some(question) = self.user_question {
            prompt.push_str(&question);
        } else {
            prompt.push_str(
                "Please analyze the above log content and environment information. \
                 Identify any errors, warnings, or issues. Explain what's happening \
                 and provide specific steps to resolve any problems.",
            );
        }

        prompt
    }

    /// Get the system prompt
    pub fn system_prompt() -> &'static str {
        TPU_ANALYSIS_SYSTEM_PROMPT
    }
}

/// Format environment information for the prompt
fn format_environment_context(info: &EnvironmentInfo) -> String {
    let mut context = String::new();

    // TPU info
    context.push_str(&format!("- TPU Type: {}\n", info.tpu.tpu_type));
    if let Some(chips) = info.tpu.chip_count {
        context.push_str(&format!("- Chip Count: {}\n", chips));
    }
    if let Some(ref topo) = info.tpu.topology {
        context.push_str(&format!("- Topology: {}\n", topo));
    }
    if let Some(hbm) = info.tpu.hbm_capacity_gb {
        context.push_str(&format!("- HBM Capacity: {} GB\n", hbm));
    }

    // Software stack
    if let Some(ref python) = info.software.python_version {
        context.push_str(&format!("- Python: {}\n", python));
    }
    if let Some(ref jax) = info.software.jax_version {
        context.push_str(&format!("- JAX: {}\n", jax));
    }
    if let Some(ref jaxlib) = info.software.jaxlib_version {
        context.push_str(&format!("- jaxlib: {}\n", jaxlib));
    }
    if let Some(ref libtpu) = info.software.libtpu_version {
        context.push_str(&format!("- libtpu: {}\n", libtpu));
    }

    // System info
    context.push_str(&format!("- Hostname: {}\n", info.system.hostname));
    context.push_str(&format!("- Kernel: {}\n", info.system.kernel_version));
    context.push_str(&format!("- Memory: {:.1} GB\n", info.system.total_memory_gb));

    // GCP info
    if let Some(ref project) = info.gcp.project_id {
        context.push_str(&format!("- GCP Project: {}\n", project));
    }
    if let Some(ref zone) = info.gcp.zone {
        context.push_str(&format!("- Zone: {}\n", zone));
    }

    // Relevant environment variables
    if !info.software.env_vars.is_empty() {
        context.push_str("\nRelevant Environment Variables:\n");
        for (key, value) in &info.software.env_vars {
            // Truncate long values
            let display_value = if value.len() > 100 {
                format!("{}...", &value[..100])
            } else {
                value.clone()
            };
            context.push_str(&format!("- {}={}\n", key, display_value));
        }
    }

    context
}

/// Format check results for the prompt
fn format_check_results(report: &ValidationReport) -> String {
    let mut results = String::new();
    let summary = report.summary();

    // Summary
    results.push_str(&format!(
        "Summary: {} passed, {} failed, {} warnings, {} skipped\n\n",
        summary.passed, summary.failed, summary.warned, summary.skipped
    ));

    // Group by status (failures and warnings first)
    let mut failures = Vec::new();
    let mut warnings = Vec::new();
    let mut passes = Vec::new();

    for check in &report.checks {
        if let Some(ref result) = check.result {
            match result {
                CheckResult::Fail { message, details, .. } => {
                    failures.push(format!(
                        "- [FAIL] {} ({}): {} - {}",
                        check.id, check.name, message, details
                    ));
                }
                CheckResult::Warn { message, details, .. } => {
                    warnings.push(format!(
                        "- [WARN] {} ({}): {} - {}",
                        check.id, check.name, message, details
                    ));
                }
                CheckResult::Pass { message, .. } => {
                    passes.push(format!("- [PASS] {} ({}): {}", check.id, check.name, message));
                }
                CheckResult::Skip { reason } => {
                    // Skip skipped checks in the prompt to reduce noise
                    if reason.contains("error") || reason.contains("fail") {
                        warnings.push(format!(
                            "- [SKIP] {} ({}): {}",
                            check.id, check.name, reason
                        ));
                    }
                }
            }
        }
    }

    if !failures.is_empty() {
        results.push_str("Failures:\n");
        for f in &failures {
            results.push_str(f);
            results.push('\n');
        }
        results.push('\n');
    }

    if !warnings.is_empty() {
        results.push_str("Warnings:\n");
        for w in &warnings {
            results.push_str(w);
            results.push('\n');
        }
        results.push('\n');
    }

    // Only include passes if there are few failures/warnings
    if failures.len() + warnings.len() < 5 && !passes.is_empty() {
        results.push_str("Passing Checks:\n");
        for p in passes.iter().take(10) {
            results.push_str(p);
            results.push('\n');
        }
        if passes.len() > 10 {
            results.push_str(&format!("... and {} more passing checks\n", passes.len() - 10));
        }
    }

    results
}

/// Truncate log content to fit within token limits
fn truncate_log_content(log: &str) -> String {
    if log.len() <= MAX_LOG_SIZE {
        return log.to_string();
    }

    // Split into lines
    let lines: Vec<&str> = log.lines().collect();

    if lines.len() <= TRUNCATE_LINES * 2 {
        // Just truncate by characters
        let half = MAX_LOG_SIZE / 2;
        return format!(
            "{}...\n\n[TRUNCATED - {} characters removed]\n\n...{}",
            &log[..half],
            log.len() - MAX_LOG_SIZE,
            &log[log.len() - half..]
        );
    }

    // Take first and last N lines
    let mut result = String::new();

    // First TRUNCATE_LINES lines
    for line in lines.iter().take(TRUNCATE_LINES) {
        result.push_str(line);
        result.push('\n');
    }

    result.push_str(&format!(
        "\n... [TRUNCATED - {} lines removed] ...\n\n",
        lines.len() - TRUNCATE_LINES * 2
    ));

    // Last TRUNCATE_LINES lines
    for line in lines.iter().skip(lines.len() - TRUNCATE_LINES) {
        result.push_str(line);
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_builder_basic() {
        let prompt = PromptBuilder::new()
            .with_question("What is wrong with my TPU?")
            .build();

        assert!(prompt.contains("## Request"));
        assert!(prompt.contains("What is wrong with my TPU?"));
    }

    #[test]
    fn test_prompt_builder_with_log() {
        let log = "Error: TPU initialization failed\nStack trace...";
        let prompt = PromptBuilder::new().with_log_content(log).build();

        assert!(prompt.contains("## Log Content"));
        assert!(prompt.contains("TPU initialization failed"));
    }

    #[test]
    fn test_truncate_log_content_short() {
        let log = "Short log content";
        let result = truncate_log_content(log);
        assert_eq!(result, log);
    }

    #[test]
    fn test_truncate_log_content_long() {
        let log = "x".repeat(MAX_LOG_SIZE + 1000);
        let result = truncate_log_content(&log);

        assert!(result.len() < log.len());
        assert!(result.contains("TRUNCATED"));
    }

    #[test]
    fn test_system_prompt() {
        let prompt = PromptBuilder::system_prompt();
        assert!(prompt.contains("TPU"));
        assert!(prompt.contains("JAX"));
        assert!(prompt.contains("XLA"));
    }
}
