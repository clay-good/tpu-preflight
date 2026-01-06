//! Minimal HTTP client for AI API requests.
//!
//! This module provides a simple HTTP/1.1 client implementation using only
//! the standard library. For HTTPS support (required by AI APIs), the "ai"
//! feature must be enabled which brings in TLS support.
//!
//! # Design Notes
//!
//! - Uses std::net::TcpStream for raw TCP connections
//! - Implements HTTP/1.1 protocol manually
//! - TLS support requires the "ai" feature flag
//! - Includes timeout handling and retry logic

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use super::AiError;

/// HTTP response from the server
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body
    pub body: String,
}

impl HttpResponse {
    /// Check if the response indicates success (2xx status)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Get a header value by name (case-insensitive)
    pub fn get_header(&self, name: &str) -> Option<&str> {
        let lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v.as_str())
    }
}

/// Configuration for HTTP requests
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Connection timeout in milliseconds
    pub connect_timeout_ms: u64,
    /// Read timeout in milliseconds
    pub read_timeout_ms: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Initial retry delay in milliseconds (doubles with each retry)
    pub retry_delay_ms: u64,
}

impl Default for HttpConfig {
    fn default() -> Self {
        HttpConfig {
            connect_timeout_ms: 30000,
            read_timeout_ms: 120000, // AI APIs can be slow
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Simple HTTP client
pub struct HttpClient {
    config: HttpConfig,
}

impl HttpClient {
    /// Create a new HTTP client with default configuration
    pub fn new() -> Self {
        HttpClient {
            config: HttpConfig::default(),
        }
    }

    /// Create a new HTTP client with custom configuration
    pub fn with_config(config: HttpConfig) -> Self {
        HttpClient { config }
    }

    /// Make an HTTP POST request (non-TLS only, for internal use)
    ///
    /// Note: AI APIs require HTTPS. Use `post_https` for production.
    #[allow(dead_code)]
    pub fn post(
        &self,
        host: &str,
        port: u16,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<HttpResponse, AiError> {
        self.post_with_retries(host, port, path, headers, body, false)
    }

    /// Make an HTTPS POST request
    ///
    /// This is the main entry point for AI API requests.
    #[cfg(feature = "ai")]
    pub fn post_https(
        &self,
        host: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<HttpResponse, AiError> {
        self.post_with_retries(host, 443, path, headers, body, true)
    }

    #[cfg(not(feature = "ai"))]
    pub fn post_https(
        &self,
        _host: &str,
        _path: &str,
        _headers: &[(&str, &str)],
        _body: &str,
    ) -> Result<HttpResponse, AiError> {
        Err(AiError::FeatureNotEnabled)
    }

    fn post_with_retries(
        &self,
        host: &str,
        port: u16,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
        _use_tls: bool,
    ) -> Result<HttpResponse, AiError> {
        let mut last_error = None;
        let mut delay = self.config.retry_delay_ms;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(delay));
                delay *= 2; // Exponential backoff
            }

            match self.do_post(host, port, path, headers, body, _use_tls) {
                Ok(response) => {
                    // Retry on 5xx errors (server errors) and 429 (rate limit)
                    if response.status >= 500 || response.status == 429 {
                        last_error = Some(AiError::ApiError {
                            status: response.status,
                            message: format!("Server error, attempt {}/{}", attempt + 1, self.config.max_retries + 1),
                        });
                        continue;
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    // Continue to retry on network errors
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AiError::RequestFailed {
            message: "Unknown error".to_string(),
        }))
    }

    #[cfg(feature = "ai")]
    fn do_post(
        &self,
        host: &str,
        port: u16,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
        use_tls: bool,
    ) -> Result<HttpResponse, AiError> {
        if use_tls {
            self.do_post_tls(host, port, path, headers, body)
        } else {
            self.do_post_plain(host, port, path, headers, body)
        }
    }

    #[cfg(not(feature = "ai"))]
    fn do_post(
        &self,
        host: &str,
        port: u16,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
        use_tls: bool,
    ) -> Result<HttpResponse, AiError> {
        if use_tls {
            Err(AiError::FeatureNotEnabled)
        } else {
            self.do_post_plain(host, port, path, headers, body)
        }
    }

    fn do_post_plain(
        &self,
        host: &str,
        port: u16,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<HttpResponse, AiError> {
        // Connect to server
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(&addr).map_err(|e| AiError::RequestFailed {
            message: format!("Connection failed: {}", e),
        })?;

        // Set timeouts
        stream
            .set_read_timeout(Some(Duration::from_millis(self.config.read_timeout_ms)))
            .ok();
        stream
            .set_write_timeout(Some(Duration::from_millis(self.config.connect_timeout_ms)))
            .ok();

        // Build request
        let request = self.build_request(host, path, headers, body);

        // Send request
        stream
            .write_all(request.as_bytes())
            .map_err(|e| AiError::RequestFailed {
                message: format!("Write failed: {}", e),
            })?;

        // Read response
        self.read_response(&mut stream)
    }

    #[cfg(feature = "ai")]
    fn do_post_tls(
        &self,
        host: &str,
        port: u16,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<HttpResponse, AiError> {
        use std::sync::Arc;

        // Connect TCP
        let addr = format!("{}:{}", host, port);
        let mut tcp_stream = TcpStream::connect(&addr).map_err(|e| AiError::RequestFailed {
            message: format!("Connection failed: {}", e),
        })?;

        tcp_stream
            .set_read_timeout(Some(Duration::from_millis(self.config.read_timeout_ms)))
            .ok();
        tcp_stream
            .set_write_timeout(Some(Duration::from_millis(self.config.connect_timeout_ms)))
            .ok();

        // Create TLS connection using rustls
        let root_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
            .map_err(|_| AiError::RequestFailed {
                message: format!("Invalid server name: {}", host),
            })?;

        let mut conn = rustls::ClientConnection::new(Arc::new(config), server_name)
            .map_err(|e| AiError::RequestFailed {
                message: format!("TLS setup failed: {}", e),
            })?;

        let mut tls_stream = rustls::Stream::new(&mut conn, &mut tcp_stream);

        // Build request
        let request = self.build_request(host, path, headers, body);

        // Send request
        tls_stream
            .write_all(request.as_bytes())
            .map_err(|e| AiError::RequestFailed {
                message: format!("TLS write failed: {}", e),
            })?;

        // Read response
        self.read_tls_response(&mut tls_stream)
    }

    fn build_request(
        &self,
        host: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> String {
        let mut request = format!(
            "POST {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n",
            path,
            host,
            body.len()
        );

        for (name, value) in headers {
            request.push_str(&format!("{}: {}\r\n", name, value));
        }

        request.push_str("\r\n");
        request.push_str(body);

        request
    }

    fn read_response<R: Read>(&self, reader: &mut R) -> Result<HttpResponse, AiError> {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 8192];

        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Timeout
                    if buffer.is_empty() {
                        return Err(AiError::Timeout {
                            message: "Read timeout".to_string(),
                        });
                    }
                    break;
                }
                Err(e) => {
                    return Err(AiError::RequestFailed {
                        message: format!("Read failed: {}", e),
                    });
                }
            }
        }

        self.parse_response(&buffer)
    }

    #[cfg(feature = "ai")]
    fn read_tls_response<S: Read + Write>(
        &self,
        stream: &mut rustls::Stream<rustls::ClientConnection, S>,
    ) -> Result<HttpResponse, AiError> {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 8192];

        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if buffer.is_empty() {
                        return Err(AiError::Timeout {
                            message: "Read timeout".to_string(),
                        });
                    }
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // TLS connection closed
                    break;
                }
                Err(e) => {
                    return Err(AiError::RequestFailed {
                        message: format!("TLS read failed: {}", e),
                    });
                }
            }
        }

        self.parse_response(&buffer)
    }

    fn parse_response(&self, buffer: &[u8]) -> Result<HttpResponse, AiError> {
        let response_str = String::from_utf8_lossy(buffer);

        // Find header/body separator
        let header_end = response_str
            .find("\r\n\r\n")
            .ok_or_else(|| AiError::ParseError {
                message: "Invalid HTTP response: no header/body separator".to_string(),
            })?;

        let header_section = &response_str[..header_end];
        let body_start = header_end + 4;

        // Parse status line
        let mut lines = header_section.lines();
        let status_line = lines.next().ok_or_else(|| AiError::ParseError {
            message: "Empty response".to_string(),
        })?;

        let status = self.parse_status_line(status_line)?;

        // Parse headers
        let mut headers = Vec::new();
        for line in lines {
            if let Some((name, value)) = line.split_once(':') {
                headers.push((name.trim().to_string(), value.trim().to_string()));
            }
        }

        // Get body (handle chunked encoding if needed)
        let body = if let Some(te) = headers.iter().find(|(k, _)| k.to_lowercase() == "transfer-encoding") {
            if te.1.to_lowercase().contains("chunked") {
                self.decode_chunked(&response_str[body_start..])?
            } else {
                response_str[body_start..].to_string()
            }
        } else {
            response_str[body_start..].to_string()
        };

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }

    fn parse_status_line(&self, line: &str) -> Result<u16, AiError> {
        // Format: "HTTP/1.1 200 OK"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(AiError::ParseError {
                message: format!("Invalid status line: {}", line),
            });
        }

        parts[1].parse().map_err(|_| AiError::ParseError {
            message: format!("Invalid status code: {}", parts[1]),
        })
    }

    fn decode_chunked(&self, body: &str) -> Result<String, AiError> {
        let mut result = String::new();
        let mut remaining = body;

        loop {
            // Find chunk size line
            let size_end = remaining.find("\r\n").ok_or_else(|| AiError::ParseError {
                message: "Invalid chunked encoding".to_string(),
            })?;

            let size_str = &remaining[..size_end];
            let chunk_size = usize::from_str_radix(size_str.trim(), 16).map_err(|_| {
                AiError::ParseError {
                    message: format!("Invalid chunk size: {}", size_str),
                }
            })?;

            if chunk_size == 0 {
                break;
            }

            let chunk_start = size_end + 2;
            let chunk_end = chunk_start + chunk_size;

            if chunk_end > remaining.len() {
                // Incomplete chunk, take what we have
                result.push_str(&remaining[chunk_start..]);
                break;
            }

            result.push_str(&remaining[chunk_start..chunk_end]);
            remaining = &remaining[chunk_end + 2..]; // Skip chunk data and trailing \r\n
        }

        Ok(result)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_line() {
        let client = HttpClient::new();
        assert_eq!(client.parse_status_line("HTTP/1.1 200 OK").unwrap(), 200);
        assert_eq!(client.parse_status_line("HTTP/1.1 404 Not Found").unwrap(), 404);
        assert_eq!(client.parse_status_line("HTTP/1.1 500 Internal Server Error").unwrap(), 500);
    }

    #[test]
    fn test_http_response_is_success() {
        let response = HttpResponse {
            status: 200,
            headers: vec![],
            body: String::new(),
        };
        assert!(response.is_success());

        let response = HttpResponse {
            status: 404,
            headers: vec![],
            body: String::new(),
        };
        assert!(!response.is_success());
    }

    #[test]
    fn test_http_response_get_header() {
        let response = HttpResponse {
            status: 200,
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Request-Id".to_string(), "abc123".to_string()),
            ],
            body: String::new(),
        };

        assert_eq!(response.get_header("content-type"), Some("application/json"));
        assert_eq!(response.get_header("Content-Type"), Some("application/json"));
        assert_eq!(response.get_header("x-request-id"), Some("abc123"));
        assert_eq!(response.get_header("missing"), None);
    }

    #[test]
    fn test_build_request() {
        let client = HttpClient::new();
        let request = client.build_request(
            "api.example.com",
            "/v1/test",
            &[("Authorization", "Bearer token"), ("Content-Type", "application/json")],
            r#"{"test": true}"#,
        );

        assert!(request.contains("POST /v1/test HTTP/1.1"));
        assert!(request.contains("Host: api.example.com"));
        assert!(request.contains("Content-Length: 14"));
        assert!(request.contains("Authorization: Bearer token"));
        assert!(request.contains("Content-Type: application/json"));
        assert!(request.contains(r#"{"test": true}"#));
    }
}
