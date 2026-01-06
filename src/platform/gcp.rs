//! GCP metadata server interface.
//!
//! Provides access to GCP instance metadata via the metadata server.
//!
//! # Graceful Degradation
//!
//! This module handles errors gracefully:
//! - Not on GCP: is_on_gcp() returns false, other functions return errors
//! - Connection timeout: Returns TpuDocError::IoError after timeout
//! - HTTP errors: Returns TpuDocError::IoError with status code
//! - Missing attributes: get_instance_attribute() returns Ok(None) for 404
//! - Parse errors: Returns TpuDocError::ParseError with context
//!
//! Default timeout is 5 seconds for all metadata operations.
//! No function in this module will panic.

use crate::TpuDocError;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const METADATA_HOST: &str = "metadata.google.internal";
const METADATA_IP: &str = "169.254.169.254";
const METADATA_PORT: u16 = 80;
const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Check if running on GCP by probing the metadata server
pub fn is_on_gcp() -> bool {
    // Try to connect to metadata server
    let addr = format!("{}:{}", METADATA_IP, METADATA_PORT);
    let socket_addr = match addr.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&socket_addr, Duration::from_millis(1000)).is_ok()
}

/// Get the GCP project ID
pub fn get_project_id() -> Result<String, TpuDocError> {
    metadata_get("/computeMetadata/v1/project/project-id")
}

/// Get the instance zone
pub fn get_zone() -> Result<String, TpuDocError> {
    let zone_path = metadata_get("/computeMetadata/v1/instance/zone")?;
    // Parse zone from full path: projects/PROJECT_NUM/zones/ZONE
    zone_path
        .rsplit('/')
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| TpuDocError::ParseError {
            context: "get_zone".to_string(),
            message: "Could not parse zone from metadata".to_string(),
        })
}

/// Get the instance name
pub fn get_instance_name() -> Result<String, TpuDocError> {
    metadata_get("/computeMetadata/v1/instance/name")
}

/// Get the machine type
pub fn get_machine_type() -> Result<String, TpuDocError> {
    let machine_path = metadata_get("/computeMetadata/v1/instance/machine-type")?;
    // Parse machine type from full path
    machine_path
        .rsplit('/')
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| TpuDocError::ParseError {
            context: "get_machine_type".to_string(),
            message: "Could not parse machine type from metadata".to_string(),
        })
}

/// Get the default service account email
pub fn get_service_account() -> Result<String, TpuDocError> {
    metadata_get("/computeMetadata/v1/instance/service-accounts/default/email")
}

/// Get the access scopes for the default service account
pub fn get_access_scopes() -> Result<Vec<String>, TpuDocError> {
    let scopes = metadata_get("/computeMetadata/v1/instance/service-accounts/default/scopes")?;
    Ok(scopes.lines().map(|s| s.to_string()).collect())
}

/// Get an instance attribute
pub fn get_instance_attribute(attr: &str) -> Result<Option<String>, TpuDocError> {
    match metadata_get(&format!("/computeMetadata/v1/instance/attributes/{}", attr)) {
        Ok(value) => Ok(Some(value)),
        Err(TpuDocError::IoError { message, .. }) if message.contains("404") => Ok(None),
        Err(e) => Err(e),
    }
}

/// Make a GET request to the metadata server
fn metadata_get(path: &str) -> Result<String, TpuDocError> {
    metadata_get_with_timeout(path, DEFAULT_TIMEOUT_MS)
}

/// Make a GET request to the metadata server with custom timeout
fn metadata_get_with_timeout(path: &str, timeout_ms: u64) -> Result<String, TpuDocError> {
    // Connect to metadata server
    let addr = format!("{}:{}", METADATA_IP, METADATA_PORT);
    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|_| TpuDocError::IoError {
            context: "metadata_get".to_string(),
            message: "Invalid address".to_string(),
        })?,
        Duration::from_millis(timeout_ms),
    )
    .map_err(|e| TpuDocError::IoError {
        context: "metadata_get".to_string(),
        message: format!("Connection failed: {}", e),
    })?;

    stream
        .set_read_timeout(Some(Duration::from_millis(timeout_ms)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_millis(timeout_ms)))
        .ok();

    // Send HTTP request
    let request = format!(
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Metadata-Flavor: Google\r\n\
         Connection: close\r\n\
         \r\n",
        path, METADATA_HOST
    );

    stream
        .write_all(request.as_bytes())
        .map_err(|e| TpuDocError::IoError {
            context: "metadata_get".to_string(),
            message: format!("Write failed: {}", e),
        })?;

    // Read response
    let mut reader = BufReader::new(stream);
    let mut response = String::new();

    // Read status line
    let mut status_line = String::new();
    reader
        .read_line(&mut status_line)
        .map_err(|e| TpuDocError::IoError {
            context: "metadata_get".to_string(),
            message: format!("Read failed: {}", e),
        })?;

    // Parse status code
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);

    if status_code != 200 {
        return Err(TpuDocError::IoError {
            context: "metadata_get".to_string(),
            message: format!("HTTP {} for {}", status_code, path),
        });
    }

    // Skip headers until empty line
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| TpuDocError::IoError {
            context: "metadata_get".to_string(),
            message: format!("Read failed: {}", e),
        })?;
        if line.trim().is_empty() {
            break;
        }
    }

    // Read body
    reader
        .read_to_string(&mut response)
        .map_err(|e| TpuDocError::IoError {
            context: "metadata_get".to_string(),
            message: format!("Read body failed: {}", e),
        })?;

    Ok(response.trim().to_string())
}
