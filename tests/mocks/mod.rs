//! Mock implementations for testing without TPU hardware.
//!
//! This module provides configurable mock implementations of the platform layer
//! that can simulate various TPU configurations, health states, and error conditions.

pub mod platform;

pub use platform::*;
