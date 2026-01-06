//! Anthropic (Claude) API client.
//!
//! Provides integration with the Anthropic Messages API for AI-powered
//! log analysis.
//!
//! # Usage
//!
//! ```no_run
//! use tpu_doc::ai::anthropic::AnthropicClient;
//!
//! let client = AnthropicClient::new()?;
//! let response = client.send_message("Analyze this log...", None)?;
//! println!("{}", response.content);
//! ```
//!
//! # Environment Variables
//!
//! - `ANTHROPIC_API_KEY`: Required. Your Anthropic API key.

use super::client::HttpClient;
use super::{AiError, AiProvider, AnalysisResponse};

const API_HOST: &str = "api.anthropic.com";
const API_PATH: &str = "/v1/messages";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Anthropic API client
pub struct AnthropicClient {
    api_key: String,
    model: String,
    max_tokens: u32,
    http_client: HttpClient,
}

impl AnthropicClient {
    /// Create a new Anthropic client using API key from environment
    pub fn new() -> Result<Self, AiError> {
        let api_key = super::get_api_key(&AiProvider::Anthropic)?;
        Ok(Self::with_key(api_key))
    }

    /// Create a new Anthropic client with a specific API key
    pub fn with_key(api_key: String) -> Self {
        AnthropicClient {
            api_key,
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
            http_client: HttpClient::new(),
        }
    }

    /// Set the model to use
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Set the maximum tokens for response
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Send a message to the API and get a response
    pub fn send_message(
        &self,
        user_message: &str,
        system_prompt: Option<&str>,
    ) -> Result<AnalysisResponse, AiError> {
        let request_body = self.build_request_body(user_message, system_prompt);

        let headers = [
            ("Content-Type", "application/json"),
            ("x-api-key", &self.api_key),
            ("anthropic-version", API_VERSION),
        ];

        let response = self
            .http_client
            .post_https(API_HOST, API_PATH, &headers, &request_body)?;

        if !response.is_success() {
            return Err(self.parse_error_response(&response.body, response.status));
        }

        self.parse_success_response(&response.body)
    }

    fn build_request_body(&self, user_message: &str, system_prompt: Option<&str>) -> String {
        let escaped_user = escape_json_string(user_message);
        let escaped_system = system_prompt.map(escape_json_string);

        let mut body = String::new();
        body.push_str("{\n");
        body.push_str(&format!("  \"model\": \"{}\",\n", self.model));
        body.push_str(&format!("  \"max_tokens\": {},\n", self.max_tokens));

        if let Some(system) = escaped_system {
            body.push_str(&format!("  \"system\": \"{}\",\n", system));
        }

        body.push_str("  \"messages\": [\n");
        body.push_str("    {\n");
        body.push_str("      \"role\": \"user\",\n");
        body.push_str(&format!("      \"content\": \"{}\"\n", escaped_user));
        body.push_str("    }\n");
        body.push_str("  ]\n");
        body.push_str("}\n");

        body
    }

    fn parse_success_response(&self, body: &str) -> Result<AnalysisResponse, AiError> {
        // Parse the JSON response manually
        // Expected format:
        // {
        //   "content": [{"type": "text", "text": "..."}],
        //   "model": "...",
        //   "usage": {"input_tokens": N, "output_tokens": N}
        // }

        let content = extract_json_string(body, "text").ok_or_else(|| AiError::ParseError {
            message: "Could not extract 'text' from response".to_string(),
        })?;

        let model = extract_json_string(body, "model").unwrap_or_else(|| self.model.clone());

        let prompt_tokens = extract_json_number(body, "input_tokens");
        let completion_tokens = extract_json_number(body, "output_tokens");

        Ok(AnalysisResponse {
            content,
            model,
            prompt_tokens,
            completion_tokens,
        })
    }

    fn parse_error_response(&self, body: &str, status: u16) -> AiError {
        // Try to extract error message from response
        let message = extract_json_string(body, "message")
            .or_else(|| extract_json_string(body, "error"))
            .unwrap_or_else(|| format!("HTTP {}", status));

        AiError::ApiError { status, message }
    }
}

/// Escape special characters for JSON string
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

/// Extract a string value from JSON by key (simple implementation)
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    // Look for "key": "value" pattern
    let search = format!("\"{}\":", key);
    let start = json.find(&search)?;
    let after_key = &json[start + search.len()..];

    // Skip whitespace
    let trimmed = after_key.trim_start();

    if !trimmed.starts_with('"') {
        return None;
    }

    // Find the closing quote, handling escapes
    let content = &trimmed[1..];
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped {
            match c {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                'u' => {
                    // Unicode escape: \uXXXX
                    let mut hex = String::new();
                    for _ in 0..4 {
                        if let Some(h) = chars.next() {
                            hex.push(h);
                        }
                    }
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(code) {
                            result.push(ch);
                        }
                    }
                }
                _ => {
                    result.push('\\');
                    result.push(c);
                }
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(result);
        } else {
            result.push(c);
        }
    }

    None
}

/// Extract a number value from JSON by key
fn extract_json_number(json: &str, key: &str) -> Option<u32> {
    let search = format!("\"{}\":", key);
    let start = json.find(&search)?;
    let after_key = &json[start + search.len()..];

    // Skip whitespace
    let trimmed = after_key.trim_start();

    // Extract the number
    let mut num_str = String::new();
    for c in trimmed.chars() {
        if c.is_ascii_digit() {
            num_str.push(c);
        } else if !num_str.is_empty() {
            break;
        }
    }

    num_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_json_string() {
        assert_eq!(escape_json_string("hello"), "hello");
        assert_eq!(escape_json_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_json_string("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_json_string("path\\to\\file"), "path\\\\to\\\\file");
        assert_eq!(escape_json_string("tab\there"), "tab\\there");
    }

    #[test]
    fn test_extract_json_string() {
        let json = r#"{"name": "test", "value": "hello world"}"#;
        assert_eq!(extract_json_string(json, "name"), Some("test".to_string()));
        assert_eq!(
            extract_json_string(json, "value"),
            Some("hello world".to_string())
        );
        assert_eq!(extract_json_string(json, "missing"), None);

        // Test with escaped content
        let json_escaped = r#"{"text": "line1\nline2"}"#;
        assert_eq!(
            extract_json_string(json_escaped, "text"),
            Some("line1\nline2".to_string())
        );
    }

    #[test]
    fn test_extract_json_number() {
        let json = r#"{"count": 42, "total": 100}"#;
        assert_eq!(extract_json_number(json, "count"), Some(42));
        assert_eq!(extract_json_number(json, "total"), Some(100));
        assert_eq!(extract_json_number(json, "missing"), None);
    }

    #[test]
    fn test_build_request_body() {
        let client = AnthropicClient::with_key("test-key".to_string());
        let body = client.build_request_body("Hello, Claude!", None);

        assert!(body.contains("\"model\": \"claude-sonnet-4-20250514\""));
        assert!(body.contains("\"max_tokens\": 4096"));
        assert!(body.contains("\"role\": \"user\""));
        assert!(body.contains("\"content\": \"Hello, Claude!\""));
    }

    #[test]
    fn test_build_request_body_with_system() {
        let client = AnthropicClient::with_key("test-key".to_string());
        let body = client.build_request_body("Hello!", Some("You are a helpful assistant."));

        assert!(body.contains("\"system\": \"You are a helpful assistant.\""));
    }
}
