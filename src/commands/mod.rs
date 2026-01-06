//! Command handlers for tpu-doc
//!
//! This module contains implementations for all tpu-doc commands:
//! - `info`: Display complete environment information
//! - `stack`: Analyze software stack compatibility
//! - `cache`: Analyze XLA compilation cache
//! - `snapshot`: Capture resource utilization snapshot
//! - `audit`: Run configuration audit
//! - `analyze`: AI-powered log analysis (requires --ai flag)

pub mod analyze;
pub mod audit;
pub mod cache;
pub mod info;
pub mod snapshot;
pub mod stack;
