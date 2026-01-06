//! Network connectivity interface.
//!
//! Provides DNS resolution, TCP connectivity, and HTTP endpoint checking.
//!
//! # Graceful Degradation
//!
//! This module handles errors gracefully:
//! - DNS failures: Returns TpuDocError::IoError with hostname context
//! - Connection timeout: Returns ConnectResult with success=false
//! - Connection refused: Returns error with connection context
//! - HTTP errors: Returns HttpResult with status_code for caller to handle
//! - HTTPS endpoints: Returns TCP connectivity result (no TLS implementation)
//! - Interface errors: Returns empty list or None for missing data
//!
//! All operations respect timeout parameters. No function will block
//! indefinitely or panic.

use crate::TpuDocError;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// DNS resolution result
#[derive(Debug, Clone)]
pub struct DnsResult {
    pub addresses: Vec<String>,
    pub resolution_time_ms: u64,
}

/// TCP connectivity result
#[derive(Debug, Clone)]
pub struct ConnectResult {
    pub success: bool,
    pub latency_ms: u64,
}

/// HTTP endpoint result
#[derive(Debug, Clone)]
pub struct HttpResult {
    pub status_code: u16,
    pub latency_ms: u64,
    pub body_preview: String,
}

/// Bandwidth measurement result
#[derive(Debug, Clone)]
pub struct BandwidthResult {
    pub bytes_per_second: f64,
    pub latency_ms: u64,
}

/// Network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_address: Option<String>,
    pub is_up: bool,
}

/// Check DNS resolution for a hostname
pub fn check_dns_resolution(hostname: &str) -> Result<DnsResult, TpuDocError> {
    let start = Instant::now();

    // Use ToSocketAddrs for DNS resolution
    let socket_addr = format!("{}:80", hostname);
    let addrs: Vec<_> = socket_addr
        .to_socket_addrs()
        .map_err(|e| TpuDocError::IoError {
            context: format!("DNS resolution for {}", hostname),
            message: e.to_string(),
        })?
        .collect();

    let resolution_time_ms = start.elapsed().as_millis() as u64;

    if addrs.is_empty() {
        return Err(TpuDocError::IoError {
            context: format!("DNS resolution for {}", hostname),
            message: "No addresses returned".to_string(),
        });
    }

    let addresses: Vec<String> = addrs.iter().map(|a| a.ip().to_string()).collect();

    Ok(DnsResult {
        addresses,
        resolution_time_ms,
    })
}

/// Check TCP connectivity to a host:port
pub fn check_tcp_connectivity(
    host: &str,
    port: u16,
    timeout_ms: u64,
) -> Result<ConnectResult, TpuDocError> {
    let start = Instant::now();

    // Resolve hostname first
    let socket_addr = format!("{}:{}", host, port);
    let addr = socket_addr
        .to_socket_addrs()
        .map_err(|e| TpuDocError::IoError {
            context: format!("TCP connect to {}:{}", host, port),
            message: format!("DNS resolution failed: {}", e),
        })?
        .next()
        .ok_or_else(|| TpuDocError::IoError {
            context: format!("TCP connect to {}:{}", host, port),
            message: "No address resolved".to_string(),
        })?;

    // Attempt connection
    match TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)) {
        Ok(_stream) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            Ok(ConnectResult {
                success: true,
                latency_ms,
            })
        }
        Err(e) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            if latency_ms >= timeout_ms {
                Ok(ConnectResult {
                    success: false,
                    latency_ms,
                })
            } else {
                Err(TpuDocError::IoError {
                    context: format!("TCP connect to {}:{}", host, port),
                    message: e.to_string(),
                })
            }
        }
    }
}

/// Check an HTTP endpoint
pub fn check_http_endpoint(url: &str, timeout_ms: u64) -> Result<HttpResult, TpuDocError> {
    let start = Instant::now();

    // Parse URL (simple implementation)
    let (host, port, path, use_https) = parse_url(url)?;

    if use_https {
        // For HTTPS, we can't easily check without TLS support
        // Just do a TCP connect to verify connectivity
        let result = check_tcp_connectivity(&host, port, timeout_ms)?;
        return Ok(HttpResult {
            status_code: if result.success { 200 } else { 0 },
            latency_ms: result.latency_ms,
            body_preview: "HTTPS endpoint (TLS not implemented)".to_string(),
        });
    }

    // Connect
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect_timeout(
        &addr.to_socket_addrs()
            .map_err(|e| TpuDocError::IoError {
                context: format!("HTTP request to {}", url),
                message: e.to_string(),
            })?
            .next()
            .ok_or_else(|| TpuDocError::IoError {
                context: format!("HTTP request to {}", url),
                message: "No address resolved".to_string(),
            })?,
        Duration::from_millis(timeout_ms),
    )
    .map_err(|e| TpuDocError::IoError {
        context: format!("HTTP request to {}", url),
        message: e.to_string(),
    })?;

    stream.set_read_timeout(Some(Duration::from_millis(timeout_ms))).ok();
    stream.set_write_timeout(Some(Duration::from_millis(timeout_ms))).ok();

    // Send request
    let request = format!(
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Connection: close\r\n\
         User-Agent: tpu-doc/0.1.0\r\n\
         \r\n",
        path, host
    );

    stream.write_all(request.as_bytes()).map_err(|e| TpuDocError::IoError {
        context: format!("HTTP request to {}", url),
        message: format!("Write failed: {}", e),
    })?;

    // Read response
    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).map_err(|e| TpuDocError::IoError {
        context: format!("HTTP request to {}", url),
        message: format!("Read failed: {}", e),
    })?;

    // Parse status code
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    // Skip headers
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        if line.trim().is_empty() {
            break;
        }
    }

    // Read body preview (first 256 bytes)
    let mut body = vec![0u8; 256];
    let bytes_read = reader.read(&mut body).unwrap_or(0);
    body.truncate(bytes_read);
    let body_preview = String::from_utf8_lossy(&body).to_string();

    let latency_ms = start.elapsed().as_millis() as u64;

    Ok(HttpResult {
        status_code,
        latency_ms,
        body_preview,
    })
}

/// Get network interfaces
pub fn get_network_interfaces() -> Result<Vec<NetworkInterface>, TpuDocError> {
    let mut interfaces = Vec::new();

    // Read from /sys/class/net
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            // Check if interface is up
            let operstate_path = entry.path().join("operstate");
            let is_up = std::fs::read_to_string(&operstate_path)
                .map(|s| s.trim() == "up")
                .unwrap_or(false);

            // Get IP address (simplified - would need to read from /proc/net/fib_trie or use netlink)
            let ip_address = get_interface_ip(&name);

            interfaces.push(NetworkInterface {
                name,
                ip_address,
                is_up,
            });
        }
    }

    Ok(interfaces)
}

// Helper functions

fn parse_url(url: &str) -> Result<(String, u16, String, bool), TpuDocError> {
    let (use_https, url_rest) = if let Some(rest) = url.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (false, rest)
    } else {
        return Err(TpuDocError::ParseError {
            context: "parse_url".to_string(),
            message: format!("Invalid URL scheme: {}", url),
        });
    };

    let (host_port, path) = match url_rest.find('/') {
        Some(idx) => (&url_rest[..idx], &url_rest[idx..]),
        None => (url_rest, "/"),
    };

    let (host, port) = match host_port.find(':') {
        Some(idx) => {
            let port: u16 = host_port[idx + 1..].parse().map_err(|_| TpuDocError::ParseError {
                context: "parse_url".to_string(),
                message: format!("Invalid port in URL: {}", url),
            })?;
            (&host_port[..idx], port)
        }
        None => (host_port, if use_https { 443 } else { 80 }),
    };

    Ok((host.to_string(), port, path.to_string(), use_https))
}

fn get_interface_ip(interface: &str) -> Option<String> {
    // This is a simplified implementation
    // A proper implementation would use netlink or parse /proc/net/fib_trie

    // Try to get from ip command output
    let output = std::process::Command::new("ip")
        .args(["addr", "show", interface])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for "inet X.X.X.X" pattern
    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("inet ") {
            if let Some(addr) = line.split_whitespace().nth(1) {
                // Remove CIDR suffix
                if let Some(ip) = addr.split('/').next() {
                    return Some(ip.to_string());
                }
            }
        }
    }

    None
}
