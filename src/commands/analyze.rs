//! AI-powered log analysis command.
//!
//! This command uses AI to analyze log files and provide diagnostic insights.
//! It requires the "ai" feature to be enabled and an API key to be set.
//!
//! # Usage
//!
//! ```sh
//! # Analyze a log file with Anthropic (default)
//! tpu-doc analyze error.log --ai
//!
//! # Use Google Gemini instead
//! tpu-doc analyze error.log --ai --provider google
//!
//! # Ask a specific question
//! tpu-doc analyze error.log --ai --question "Why is my training hanging?"
//! ```

use crate::cli::args::Args;
use crate::TpuDocError;
use std::fs;

#[cfg(feature = "ai")]
use crate::ai::{
    anthropic::AnthropicClient,
    google::GeminiClient,
    prompt::PromptBuilder,
    AiProvider,
};

#[cfg(feature = "ai")]
use crate::commands::info;

/// Maximum log file size to read (10MB)
const MAX_LOG_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Run the analyze command
pub fn run(args: &Args) -> Result<String, TpuDocError> {
    // Check if AI is enabled
    if !args.ai_enabled {
        return Err(TpuDocError::CommandError {
            command: "analyze".to_string(),
            message: "The --ai flag is required for the analyze command. \
                     This enables AI-powered analysis using your API key."
                .to_string(),
        });
    }

    #[cfg(not(feature = "ai"))]
    {
        return Err(TpuDocError::CommandError {
            command: "analyze".to_string(),
            message: "AI features are not enabled. Rebuild with: cargo build --features ai"
                .to_string(),
        });
    }

    #[cfg(feature = "ai")]
    {
        run_ai_analysis(args)
    }
}

#[cfg(feature = "ai")]
fn run_ai_analysis(args: &Args) -> Result<String, TpuDocError> {
    let log_path = args.log_file.as_ref().ok_or_else(|| TpuDocError::CommandError {
        command: "analyze".to_string(),
        message: "Log file path is required. Usage: tpu-doc analyze <log_file> --ai".to_string(),
    })?;

    // Read the log file
    let log_content = read_log_file(log_path)?;

    // Gather environment context
    let env_info = info::gather_environment_info_internal();

    // Build the prompt
    let mut prompt_builder = PromptBuilder::new()
        .with_environment(&env_info)
        .with_log_content(&log_content);

    if let Some(ref question) = args.ai_question {
        prompt_builder = prompt_builder.with_question(question);
    }

    let prompt = prompt_builder.build();
    let system_prompt = PromptBuilder::system_prompt();

    // Get the AI provider
    let provider = args.ai_provider.clone().unwrap_or_default();

    // Call the appropriate AI provider
    let response = match provider {
        AiProvider::Anthropic => {
            let client = AnthropicClient::new()?;
            let client = if let Some(ref model) = args.ai_model {
                client.with_model(model)
            } else {
                client
            };
            client.send_message(&prompt, Some(system_prompt))?
        }
        AiProvider::Google => {
            let client = GeminiClient::new()?;
            let client = if let Some(ref model) = args.ai_model {
                client.with_model(model)
            } else {
                client
            };
            client.send_message(&prompt, Some(system_prompt))?
        }
    };

    // Format the output
    let mut output = String::new();
    output.push_str("================================================================================\n");
    output.push_str("                         AI LOG ANALYSIS\n");
    output.push_str("================================================================================\n\n");

    output.push_str(&format!("Log File: {}\n", log_path));
    output.push_str(&format!("Model: {}\n", response.model));

    if let (Some(prompt_tokens), Some(completion_tokens)) =
        (response.prompt_tokens, response.completion_tokens)
    {
        output.push_str(&format!(
            "Tokens: {} prompt + {} completion = {} total\n",
            prompt_tokens,
            completion_tokens,
            prompt_tokens + completion_tokens
        ));
    }

    output.push_str("\n--------------------------------------------------------------------------------\n");
    output.push_str("ANALYSIS\n");
    output.push_str("--------------------------------------------------------------------------------\n\n");
    output.push_str(&response.content);
    output.push_str("\n\n================================================================================\n");

    Ok(output)
}

fn read_log_file(path: &str) -> Result<String, TpuDocError> {
    // Check file exists
    let metadata = fs::metadata(path).map_err(|e| TpuDocError::IoError {
        context: "read_log_file".to_string(),
        message: format!("Cannot access log file '{}': {}", path, e),
    })?;

    // Check file size
    if metadata.len() > MAX_LOG_FILE_SIZE {
        return Err(TpuDocError::IoError {
            context: "read_log_file".to_string(),
            message: format!(
                "Log file is too large ({:.1} MB). Maximum size is {} MB.",
                metadata.len() as f64 / (1024.0 * 1024.0),
                MAX_LOG_FILE_SIZE / (1024 * 1024)
            ),
        });
    }

    // Read the file
    fs::read_to_string(path).map_err(|e| TpuDocError::IoError {
        context: "read_log_file".to_string(),
        message: format!("Failed to read log file '{}': {}", path, e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_log_file_not_found() {
        let result = read_log_file("/nonexistent/path/to/file.log");
        assert!(result.is_err());
    }
}
