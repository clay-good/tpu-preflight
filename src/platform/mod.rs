//! Platform abstraction layer.
//!
//! Provides consistent interfaces for:
//! - TPU device information
//! - Linux system information
//! - GCP metadata
//! - Network connectivity

pub mod gcp;
pub mod linux;
pub mod network;
pub mod tpu;
