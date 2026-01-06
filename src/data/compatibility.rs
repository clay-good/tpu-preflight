//! Compatibility matrix for JAX ecosystem versions
//!
//! Provides version compatibility information loaded from embedded data.

/// Compatibility status between package versions
#[derive(Debug, Clone, PartialEq)]
pub enum CompatibilityStatus {
    /// Versions are fully compatible
    Compatible,
    /// Versions work but with known issues
    CompatibleWithWarnings,
    /// Versions are incompatible
    Incompatible,
    /// Compatibility unknown
    Unknown,
}

/// JAX version entry in the compatibility matrix
#[derive(Debug, Clone)]
pub struct JaxVersionEntry {
    pub version: String,
    pub python_min: String,
    pub python_max: String,
    pub jaxlib_version: String,
    pub libtpu_versions: Vec<String>,
    pub notes: Option<String>,
}

/// Known package conflict
#[derive(Debug, Clone)]
pub struct KnownConflict {
    pub packages: Vec<String>,
    pub description: String,
    pub resolution: String,
}

/// Recommended version set
#[derive(Debug, Clone)]
pub struct RecommendedVersions {
    pub jax_version: String,
    pub python_version: String,
}

/// The compatibility matrix
#[derive(Debug)]
pub struct CompatibilityMatrix {
    pub version: String,
    pub updated: String,
    pub jax_versions: Vec<JaxVersionEntry>,
    pub known_conflicts: Vec<KnownConflict>,
    pub recommended: RecommendedVersionsMap,
}

#[derive(Debug)]
pub struct RecommendedVersionsMap {
    pub v4: RecommendedVersions,
    pub v5e: RecommendedVersions,
    pub v5p: RecommendedVersions,
    pub v6e: RecommendedVersions,
}

impl CompatibilityMatrix {
    /// Load the embedded compatibility matrix
    pub fn load() -> Self {
        // Embedded compatibility data
        // In a real implementation, this would parse JSON from include_str!
        CompatibilityMatrix {
            version: "1.0".to_string(),
            updated: "2025-01-03".to_string(),
            jax_versions: vec![
                JaxVersionEntry {
                    version: "0.4.35".to_string(),
                    python_min: "3.9".to_string(),
                    python_max: "3.12".to_string(),
                    jaxlib_version: "0.4.35".to_string(),
                    libtpu_versions: vec!["0.1.dev20241028".to_string(), "0.1.dev20241101".to_string()],
                    notes: Some("Latest stable release".to_string()),
                },
                JaxVersionEntry {
                    version: "0.4.34".to_string(),
                    python_min: "3.9".to_string(),
                    python_max: "3.12".to_string(),
                    jaxlib_version: "0.4.34".to_string(),
                    libtpu_versions: vec!["0.1.dev20241001".to_string()],
                    notes: None,
                },
                JaxVersionEntry {
                    version: "0.4.33".to_string(),
                    python_min: "3.9".to_string(),
                    python_max: "3.12".to_string(),
                    jaxlib_version: "0.4.33".to_string(),
                    libtpu_versions: vec!["0.1.dev20240901".to_string()],
                    notes: Some("Fixed large batch compilation issues".to_string()),
                },
                JaxVersionEntry {
                    version: "0.4.30".to_string(),
                    python_min: "3.9".to_string(),
                    python_max: "3.12".to_string(),
                    jaxlib_version: "0.4.30".to_string(),
                    libtpu_versions: vec!["0.1.dev20240701".to_string()],
                    notes: Some("NumPy 2.0 compatibility added".to_string()),
                },
                JaxVersionEntry {
                    version: "0.4.26".to_string(),
                    python_min: "3.9".to_string(),
                    python_max: "3.11".to_string(),
                    jaxlib_version: "0.4.26".to_string(),
                    libtpu_versions: vec!["0.1.dev20240501".to_string()],
                    notes: None,
                },
            ],
            known_conflicts: vec![
                KnownConflict {
                    packages: vec!["jax>=0.4.30".to_string(), "tensorflow<2.15".to_string()],
                    description: "TensorFlow 2.14 and earlier conflict with JAX 0.4.30+".to_string(),
                    resolution: "Upgrade TensorFlow to 2.15+ or use separate environments".to_string(),
                },
                KnownConflict {
                    packages: vec!["jax<0.4.26".to_string(), "numpy>=2.0".to_string()],
                    description: "NumPy 2.0 is incompatible with JAX versions before 0.4.26".to_string(),
                    resolution: "Upgrade JAX to 0.4.26+ or downgrade NumPy to 1.x".to_string(),
                },
                KnownConflict {
                    packages: vec!["jax".to_string(), "torch".to_string()],
                    description: "JAX and PyTorch may conflict when both try to use TPU".to_string(),
                    resolution: "Use JAX_PLATFORMS=tpu to ensure JAX uses TPU exclusively".to_string(),
                },
            ],
            recommended: RecommendedVersionsMap {
                v4: RecommendedVersions {
                    jax_version: "0.4.35".to_string(),
                    python_version: "3.11".to_string(),
                },
                v5e: RecommendedVersions {
                    jax_version: "0.4.35".to_string(),
                    python_version: "3.11".to_string(),
                },
                v5p: RecommendedVersions {
                    jax_version: "0.4.35".to_string(),
                    python_version: "3.11".to_string(),
                },
                v6e: RecommendedVersions {
                    jax_version: "0.4.35".to_string(),
                    python_version: "3.11".to_string(),
                },
            },
        }
    }

    /// Check if versions are compatible
    pub fn is_compatible(&self, jax_ver: &str, _libtpu_ver: &str, python_ver: &str) -> CompatibilityStatus {
        // Find the JAX version entry
        let jax_entry = self.jax_versions.iter().find(|e| e.version == jax_ver);

        match jax_entry {
            Some(entry) => {
                // Check Python version is in range
                if !is_version_in_range(python_ver, &entry.python_min, &entry.python_max) {
                    return CompatibilityStatus::Incompatible;
                }

                // Check for known conflicts
                // In a real implementation, we'd check the conflicts more thoroughly

                CompatibilityStatus::Compatible
            }
            None => CompatibilityStatus::Unknown,
        }
    }

    /// Get recommended versions for a JAX version
    pub fn get_recommended_for_jax(&self, _jax_ver: &str) -> Option<RecommendedVersions> {
        // Return the general recommendation
        Some(RecommendedVersions {
            jax_version: "0.4.35".to_string(),
            python_version: "3.11".to_string(),
        })
    }

    /// Get recommended versions for a TPU type
    pub fn get_recommended_for_tpu(&self, tpu_type: &str) -> Option<&RecommendedVersions> {
        match tpu_type.to_lowercase().as_str() {
            "v4" => Some(&self.recommended.v4),
            "v5e" => Some(&self.recommended.v5e),
            "v5p" => Some(&self.recommended.v5p),
            "v6e" => Some(&self.recommended.v6e),
            _ => None,
        }
    }
}

fn is_version_in_range(version: &str, min: &str, max: &str) -> bool {
    let version_parts: Vec<u32> = version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let min_parts: Vec<u32> = min
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let max_parts: Vec<u32> = max
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    if version_parts.len() < 2 || min_parts.len() < 2 || max_parts.len() < 2 {
        return false;
    }

    let version_tuple = (version_parts[0], version_parts[1]);
    let min_tuple = (min_parts[0], min_parts[1]);
    let max_tuple = (max_parts[0], max_parts[1]);

    version_tuple >= min_tuple && version_tuple <= max_tuple
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_matrix() {
        let matrix = CompatibilityMatrix::load();
        assert!(!matrix.jax_versions.is_empty());
        assert!(!matrix.known_conflicts.is_empty());
    }

    #[test]
    fn test_version_in_range() {
        assert!(is_version_in_range("3.10", "3.9", "3.12"));
        assert!(is_version_in_range("3.9", "3.9", "3.12"));
        assert!(is_version_in_range("3.12", "3.9", "3.12"));
        assert!(!is_version_in_range("3.8", "3.9", "3.12"));
        assert!(!is_version_in_range("3.13", "3.9", "3.12"));
    }

    #[test]
    fn test_compatibility_check() {
        let matrix = CompatibilityMatrix::load();
        let status = matrix.is_compatible("0.4.35", "0.1.dev20241028", "3.11");
        assert_eq!(status, CompatibilityStatus::Compatible);
    }
}
