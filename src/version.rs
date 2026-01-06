//! Version and build information.
//!
//! Provides version, git commit, and build metadata.

use std::fmt;

/// Build information
#[derive(Debug, Clone)]
pub struct BuildInfo {
    pub version: &'static str,
    pub commit: Option<&'static str>,
    pub build_date: Option<&'static str>,
    pub target: &'static str,
    pub rustc_version: Option<&'static str>,
}

impl fmt::Display for BuildInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "tpu-doc {}", self.version)?;

        if let Some(commit) = self.commit {
            writeln!(f, "Commit: {}", commit)?;
        }

        if let Some(date) = self.build_date {
            writeln!(f, "Built: {}", date)?;
        }

        writeln!(f, "Target: {}", self.target)?;

        if let Some(rustc) = self.rustc_version {
            write!(f, "Rustc: {}", rustc)?;
        }

        Ok(())
    }
}

/// Get build information
pub fn get_build_info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        commit: option_env!("TPU_DOC_GIT_HASH"),
        build_date: option_env!("TPU_DOC_BUILD_DATE"),
        target: std::env::consts::ARCH,
        rustc_version: option_env!("TPU_DOC_RUSTC_VERSION"),
    }
}
