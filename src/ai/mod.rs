//! AI integration layer for log analysis.
//!
//! This module provides optional AI-powered analysis capabilities.
//! It is feature-gated behind the "ai" feature flag.
//!
//! # Usage
//!
//! Enable the feature in Cargo.toml:
//! ```toml
//! [dependencies]
//! tpu-doc = { version = "0.1", features = ["ai"] }
//! ```
//!
//! Or build with:
//! ```sh
//! cargo build --features ai
//! ```
//!
//! # Supported Providers
//!
//! - Anthropic (Claude): Set ANTHROPIC_API_KEY environment variable
//! - Google (Gemini): Set GOOGLE_API_KEY environment variable
//!
//! # Design Principles
//!
//! - No runtime dependencies for the core binary
//! - AI features are strictly opt-in
//! - User must provide their own API keys
//! - Graceful error handling for API failures

pub mod client;
pub mod prompt;

#[cfg(feature = "ai")]
pub mod anthropic;

#[cfg(feature = "ai")]
pub mod google;

use crate::TpuDocError;

/// AI provider selection
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AiProvider {
    #[default]
    Anthropic,
    Google,
}

impl AiProvider {
    /// Parse provider from string
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

    /// Get the environment variable name for the API key
    pub fn api_key_env_var(&self) -> &'static str {
        match self {
            AiProvider::Anthropic => "ANTHROPIC_API_KEY",
            AiProvider::Google => "GOOGLE_API_KEY",
        }
    }

    /// Get the default model for this provider
    pub fn default_model(&self) -> &'static str {
        match self {
            AiProvider::Anthropic => "claude-sonnet-4-20250514",
            AiProvider::Google => "gemini-1.5-flash",
        }
    }
}

/// AI analysis request
#[derive(Debug, Clone)]
pub struct AnalysisRequest {
    /// The log content to analyze
    pub log_content: String,
    /// Optional user question
    pub question: Option<String>,
    /// AI provider to use
    pub provider: AiProvider,
    /// Model to use (provider-specific)
    pub model: Option<String>,
    /// Maximum tokens in response
    pub max_tokens: u32,
}

impl Default for AnalysisRequest {
    fn default() -> Self {
        AnalysisRequest {
            log_content: String::new(),
            question: None,
            provider: AiProvider::default(),
            model: None,
            max_tokens: 4096,
        }
    }
}

/// AI analysis response
#[derive(Debug, Clone)]
pub struct AnalysisResponse {
    /// The analysis result text
    pub content: String,
    /// Model used for analysis
    pub model: String,
    /// Tokens used in prompt
    pub prompt_tokens: Option<u32>,
    /// Tokens used in response
    pub completion_tokens: Option<u32>,
}

/// Error types specific to AI operations
#[derive(Debug, Clone)]
pub enum AiError {
    /// API key not found in environment
    ApiKeyNotFound { provider: String, env_var: String },
    /// API request failed
    RequestFailed { message: String },
    /// API returned an error response
    ApiError { status: u16, message: String },
    /// Response parsing failed
    ParseError { message: String },
    /// Connection timeout
    Timeout { message: String },
    /// Feature not enabled
    FeatureNotEnabled,
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::ApiKeyNotFound { provider, env_var } => {
                write!(
                    f,
                    "{} API key not found. Set {} environment variable.",
                    provider, env_var
                )
            }
            AiError::RequestFailed { message } => {
                write!(f, "API request failed: {}", message)
            }
            AiError::ApiError { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
            AiError::ParseError { message } => {
                write!(f, "Failed to parse API response: {}", message)
            }
            AiError::Timeout { message } => {
                write!(f, "Request timeout: {}", message)
            }
            AiError::FeatureNotEnabled => {
                write!(
                    f,
                    "AI feature not enabled. Build with: cargo build --features ai"
                )
            }
        }
    }
}

impl From<AiError> for TpuDocError {
    fn from(e: AiError) -> Self {
        TpuDocError::CommandError {
            command: "analyze".to_string(),
            message: e.to_string(),
        }
    }
}

/// Check if AI features are available
pub fn is_ai_available() -> bool {
    cfg!(feature = "ai")
}

/// Get API key for a provider from environment
pub fn get_api_key(provider: &AiProvider) -> Result<String, AiError> {
    let env_var = provider.api_key_env_var();
    std::env::var(env_var).map_err(|_| AiError::ApiKeyNotFound {
        provider: format!("{:?}", provider),
        env_var: env_var.to_string(),
    })
}
