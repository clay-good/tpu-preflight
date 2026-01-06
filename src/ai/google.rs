//! Google Gemini API client.
//!
//! Provides integration with the Google Gemini API for AI-powered
//! log analysis.
//!
//! # Usage
//!
//! ```no_run
//! use tpu_doc::ai::google::GeminiClient;
//!
//! let client = GeminiClient::new()?;
//! let response = client.send_message("Analyze this log...", None)?;
//! println!("{}", response.content);
//! ```
//!
//! # Environment Variables
//!
//! - `GOOGLE_API_KEY`: Required. Your Google API key with Gemini access.

use super::client::HttpClient;
use super::{AiError, AiProvider, AnalysisResponse};

const API_HOST: &str = "generativelanguage.googleapis.com";
const DEFAULT_MODEL: &str = "gemini-1.5-flash";
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Google Gemini API client
pub struct GeminiClient {
    api_key: String,
    model: String,
    max_tokens: u32,
    http_client: HttpClient,
}

impl GeminiClient {
    /// Create a new Gemini client using API key from environment
    pub fn new() -> Result<Self, AiError> {
        let api_key = super::get_api_key(&AiProvider::Google)?;
        Ok(Self::with_key(api_key))
    }

    /// Create a new Gemini client with a specific API key
    pub fn with_key(api_key: String) -> Self {
        GeminiClient {
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
        let api_path = format!(
            "/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let request_body = self.build_request_body(user_message, system_prompt);

        let headers = [("Content-Type", "application/json")];

        let response = self
            .http_client
            .post_https(API_HOST, &api_path, &headers, &request_body)?;

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

        // System instruction (if provided)
        if let Some(system) = escaped_system {
            body.push_str("  \"system_instruction\": {\n");
            body.push_str("    \"parts\": [\n");
            body.push_str("      {\n");
            body.push_str(&format!("        \"text\": \"{}\"\n", system));
            body.push_str("      }\n");
            body.push_str("    ]\n");
            body.push_str("  },\n");
        }

        // User content
        body.push_str("  \"contents\": [\n");
        body.push_str("    {\n");
        body.push_str("      \"role\": \"user\",\n");
        body.push_str("      \"parts\": [\n");
        body.push_str("        {\n");
        body.push_str(&format!("          \"text\": \"{}\"\n", escaped_user));
        body.push_str("        }\n");
        body.push_str("      ]\n");
        body.push_str("    }\n");
        body.push_str("  ],\n");

        // Generation config
        body.push_str("  \"generationConfig\": {\n");
        body.push_str(&format!("    \"maxOutputTokens\": {}\n", self.max_tokens));
        body.push_str("  }\n");

        body.push_str("}\n");

        body
    }

    fn parse_success_response(&self, body: &str) -> Result<AnalysisResponse, AiError> {
        // Parse the JSON response manually
        // Expected format:
        // {
        //   "candidates": [{
        //     "content": {
        //       "parts": [{"text": "..."}],
        //       "role": "model"
        //     }
        //   }],
        //   "usageMetadata": {
        //     "promptTokenCount": N,
        //     "candidatesTokenCount": N
        //   }
        // }

        let content = extract_json_string(body, "text").ok_or_else(|| AiError::ParseError {
            message: "Could not extract 'text' from response".to_string(),
        })?;

        let prompt_tokens = extract_json_number(body, "promptTokenCount");
        let completion_tokens = extract_json_number(body, "candidatesTokenCount");

        Ok(AnalysisResponse {
            content,
            model: self.model.clone(),
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
    let search = format!("\"{}\":", key);
    let start = json.find(&search)?;
    let after_key = &json[start + search.len()..];

    let trimmed = after_key.trim_start();

    if !trimmed.starts_with('"') {
        return None;
    }

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

    let trimmed = after_key.trim_start();

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
    }

    #[test]
    fn test_build_request_body() {
        let client = GeminiClient::with_key("test-key".to_string());
        let body = client.build_request_body("Hello, Gemini!", None);

        assert!(body.contains("\"role\": \"user\""));
        assert!(body.contains("\"text\": \"Hello, Gemini!\""));
        assert!(body.contains("\"maxOutputTokens\": 4096"));
    }

    #[test]
    fn test_build_request_body_with_system() {
        let client = GeminiClient::with_key("test-key".to_string());
        let body = client.build_request_body("Hello!", Some("You are a helpful assistant."));

        assert!(body.contains("\"system_instruction\""));
        assert!(body.contains("\"text\": \"You are a helpful assistant.\""));
    }
}
