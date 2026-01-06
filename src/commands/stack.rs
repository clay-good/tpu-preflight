//! Stack compatibility command
//!
//! Provides detailed software stack analysis and compatibility checking.

use crate::cli::args::{Args, OutputFormat};
use crate::data::compatibility::{CompatibilityMatrix, CompatibilityStatus};
use crate::TpuDocError;
use std::env;
use std::process::Command;

/// Software version information
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub package: String,
    pub version: Option<String>,
    pub detection_method: String,
}

/// Stack analysis result
#[derive(Debug)]
pub struct StackAnalysis {
    pub versions: Vec<VersionInfo>,
    pub compatibility_status: CompatibilityStatus,
    pub issues: Vec<StackIssue>,
    pub recommendations: Vec<String>,
}

/// Issue found in the stack
#[derive(Debug)]
pub struct StackIssue {
    pub severity: IssueSeverity,
    pub description: String,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

/// Run the stack command
pub fn run(args: &Args) -> Result<String, TpuDocError> {
    let analysis = analyze_stack();

    if args.show_matrix {
        let matrix = CompatibilityMatrix::load();
        match args.format {
            OutputFormat::Json => Ok(format_matrix_json(&matrix)),
            _ => Ok(format_matrix_text(&matrix)),
        }
    } else {
        match args.format {
            OutputFormat::Json => Ok(format_json(&analysis)),
            _ => Ok(format_text(&analysis, args.verbose)),
        }
    }
}

fn analyze_stack() -> StackAnalysis {
    let mut versions = Vec::new();
    let mut issues = Vec::new();
    let mut recommendations = Vec::new();

    // Detect Python version
    let python_version = detect_version("Python", || {
        let output = Command::new("python3")
            .args(["--version"])
            .output()
            .ok()?;
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim();
            if version.starts_with("Python ") {
                return Some((version[7..].to_string(), "python3 --version"));
            }
        }
        None
    });
    versions.push(python_version.clone());

    // Check Python version compatibility
    if let Some(ref ver) = python_version.version {
        if let Some((major, minor)) = parse_version(ver) {
            if major < 3 || (major == 3 && minor < 9) {
                issues.push(StackIssue {
                    severity: IssueSeverity::Error,
                    description: format!("Python {} is below minimum required version 3.9", ver),
                    resolution: Some("Upgrade to Python 3.9 or later".to_string()),
                });
            } else if major == 3 && minor >= 13 {
                issues.push(StackIssue {
                    severity: IssueSeverity::Warning,
                    description: format!("Python {} may not be fully tested with JAX", ver),
                    resolution: Some("Consider using Python 3.10-3.12 for best compatibility".to_string()),
                });
            }
        }
    }

    // Detect JAX version
    let jax_version = detect_version("JAX", || {
        // Try environment variable first
        if let Ok(version) = env::var("JAX_VERSION") {
            return Some((version, "JAX_VERSION env var"));
        }

        // Try Python import
        let output = Command::new("python3")
            .args(["-c", "import jax; print(jax.__version__)"])
            .output()
            .ok()?;
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            return Some((version.trim().to_string(), "python import"));
        }

        // Try pip show
        let output = Command::new("pip3")
            .args(["show", "jax"])
            .output()
            .ok()?;
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.starts_with("Version: ") {
                    return Some((line[9..].to_string(), "pip show"));
                }
            }
        }

        None
    });
    versions.push(jax_version.clone());

    // Detect jaxlib version
    let jaxlib_version = detect_version("jaxlib", || {
        let output = Command::new("python3")
            .args(["-c", "import jaxlib; print(jaxlib.__version__)"])
            .output()
            .ok()?;
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            return Some((version.trim().to_string(), "python import"));
        }
        None
    });
    versions.push(jaxlib_version.clone());

    // Check JAX/jaxlib version match
    if let (Some(jax_ver), Some(jaxlib_ver)) = (&jax_version.version, &jaxlib_version.version) {
        if !versions_compatible(jax_ver, jaxlib_ver) {
            issues.push(StackIssue {
                severity: IssueSeverity::Error,
                description: format!("JAX {} and jaxlib {} version mismatch", jax_ver, jaxlib_ver),
                resolution: Some("Ensure JAX and jaxlib versions match".to_string()),
            });
        }
    }

    // Detect libtpu version
    let libtpu_version = detect_version("libtpu", || {
        if let Ok(version) = env::var("LIBTPU_VERSION") {
            return Some((version, "LIBTPU_VERSION env var"));
        }

        let output = Command::new("python3")
            .args(["-c", "import libtpu; print(libtpu.__version__)"])
            .output()
            .ok()?;
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            return Some((version.trim().to_string(), "python import"));
        }
        None
    });
    versions.push(libtpu_version);

    // Detect NumPy version
    let numpy_version = detect_version("NumPy", || {
        let output = Command::new("python3")
            .args(["-c", "import numpy; print(numpy.__version__)"])
            .output()
            .ok()?;
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            return Some((version.trim().to_string(), "python import"));
        }
        None
    });
    versions.push(numpy_version.clone());

    // Check NumPy 2.x compatibility
    if let Some(ref ver) = numpy_version.version {
        if let Some((major, _)) = parse_version(ver) {
            if major >= 2 {
                if let Some(ref jax_ver) = jax_version.version {
                    if let Some((jax_major, jax_minor)) = parse_version(jax_ver) {
                        if jax_major == 0 && jax_minor < 4 {
                            issues.push(StackIssue {
                                severity: IssueSeverity::Error,
                                description: format!("NumPy {} is incompatible with JAX {}", ver, jax_ver),
                                resolution: Some("Upgrade JAX to 0.4.26+ or downgrade NumPy to 1.x".to_string()),
                            });
                        }
                    }
                }
            }
        }
    }

    // Check PJRT plugin
    let pjrt_status = check_pjrt_plugin();
    if pjrt_status.is_none() {
        issues.push(StackIssue {
            severity: IssueSeverity::Warning,
            description: "PJRT TPU plugin not detected".to_string(),
            resolution: Some("Ensure TPU_LIBRARY_PATH is set correctly".to_string()),
        });
    }

    // Check for TensorFlow conflicts
    let tf_version = detect_version("TensorFlow", || {
        let output = Command::new("python3")
            .args(["-c", "import tensorflow; print(tensorflow.__version__)"])
            .output()
            .ok()?;
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            return Some((version.trim().to_string(), "python import"));
        }
        None
    });

    if let Some(ref tf_ver) = tf_version.version {
        if let Some((tf_major, tf_minor)) = parse_version(tf_ver) {
            if let Some(ref jax_ver) = jax_version.version {
                if let Some((jax_major, jax_minor)) = parse_version(jax_ver) {
                    if jax_major == 0 && jax_minor >= 30 && tf_major == 2 && tf_minor < 15 {
                        issues.push(StackIssue {
                            severity: IssueSeverity::Warning,
                            description: format!("TensorFlow {} may conflict with JAX {}", tf_ver, jax_ver),
                            resolution: Some("Consider using separate environments or upgrade TensorFlow".to_string()),
                        });
                    }
                }
            }
        }
    }

    // Generate recommendations
    let matrix = CompatibilityMatrix::load();
    if let Some(ref jax_ver) = jax_version.version {
        if let Some(recommended) = matrix.get_recommended_for_jax(jax_ver) {
            if jax_version.version.as_deref() != Some(&recommended.jax_version) {
                recommendations.push(format!(
                    "Recommended JAX version for your TPU: {}",
                    recommended.jax_version
                ));
            }
        }
    }

    if issues.iter().any(|i| i.severity == IssueSeverity::Error) {
        recommendations.push("Fix critical issues before running workloads".to_string());
    }

    // Determine overall compatibility status
    let compatibility_status = if issues.iter().any(|i| i.severity == IssueSeverity::Error) {
        CompatibilityStatus::Incompatible
    } else if issues.iter().any(|i| i.severity == IssueSeverity::Warning) {
        CompatibilityStatus::CompatibleWithWarnings
    } else if jax_version.version.is_some() {
        CompatibilityStatus::Compatible
    } else {
        CompatibilityStatus::Unknown
    };

    StackAnalysis {
        versions,
        compatibility_status,
        issues,
        recommendations,
    }
}

fn detect_version<F>(package: &str, detector: F) -> VersionInfo
where
    F: FnOnce() -> Option<(String, &'static str)>,
{
    match detector() {
        Some((version, method)) => VersionInfo {
            package: package.to_string(),
            version: Some(version),
            detection_method: method.to_string(),
        },
        None => VersionInfo {
            package: package.to_string(),
            version: None,
            detection_method: "not found".to_string(),
        },
    }
}

fn parse_version(version: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()?;
        Some((major, minor))
    } else {
        None
    }
}

fn versions_compatible(jax: &str, jaxlib: &str) -> bool {
    // JAX and jaxlib should have matching major.minor versions
    let jax_parts: Vec<&str> = jax.split('.').collect();
    let jaxlib_parts: Vec<&str> = jaxlib.split('.').collect();

    if jax_parts.len() >= 2 && jaxlib_parts.len() >= 2 {
        jax_parts[0] == jaxlib_parts[0] && jax_parts[1] == jaxlib_parts[1]
    } else {
        false
    }
}

fn check_pjrt_plugin() -> Option<String> {
    env::var("TPU_LIBRARY_PATH").ok()
}

fn format_text(analysis: &StackAnalysis, verbose: bool) -> String {
    let mut output = String::new();

    output.push_str("================================================================================\n");
    output.push_str("                         SOFTWARE STACK ANALYSIS\n");
    output.push_str("================================================================================\n\n");

    // Status summary
    let status_str = match analysis.compatibility_status {
        CompatibilityStatus::Compatible => "COMPATIBLE",
        CompatibilityStatus::CompatibleWithWarnings => "COMPATIBLE (with warnings)",
        CompatibilityStatus::Incompatible => "INCOMPATIBLE",
        CompatibilityStatus::Unknown => "UNKNOWN",
    };
    output.push_str(&format!("Stack Status: {}\n\n", status_str));

    // Version table
    output.push_str("DETECTED VERSIONS\n");
    output.push_str("-----------------\n");
    for v in &analysis.versions {
        let version_str = v.version.as_deref().unwrap_or("Not found");
        output.push_str(&format!("  {:12} {:20}", v.package, version_str));
        if verbose {
            output.push_str(&format!(" ({})", v.detection_method));
        }
        output.push('\n');
    }
    output.push('\n');

    // Issues
    if !analysis.issues.is_empty() {
        output.push_str("ISSUES FOUND\n");
        output.push_str("------------\n");
        for issue in &analysis.issues {
            let icon = match issue.severity {
                IssueSeverity::Error => "[ERROR]  ",
                IssueSeverity::Warning => "[WARN]   ",
                IssueSeverity::Info => "[INFO]   ",
            };
            output.push_str(&format!("  {} {}\n", icon, issue.description));
            if let Some(ref resolution) = issue.resolution {
                output.push_str(&format!("           Resolution: {}\n", resolution));
            }
        }
        output.push('\n');
    }

    // Recommendations
    if !analysis.recommendations.is_empty() {
        output.push_str("RECOMMENDATIONS\n");
        output.push_str("---------------\n");
        for rec in &analysis.recommendations {
            output.push_str(&format!("  * {}\n", rec));
        }
        output.push('\n');
    }

    output.push_str("================================================================================\n");

    output
}

fn format_json(analysis: &StackAnalysis) -> String {
    let mut json = String::new();
    json.push_str("{\n");

    // Status
    json.push_str(&format!("  \"status\": \"{:?}\",\n", analysis.compatibility_status));

    // Versions
    json.push_str("  \"versions\": [\n");
    for (i, v) in analysis.versions.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"package\": \"{}\",\n", v.package));
        json.push_str(&format!("      \"version\": {},\n",
            v.version.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
        json.push_str(&format!("      \"detection_method\": \"{}\"\n", v.detection_method));
        json.push_str("    }");
        if i < analysis.versions.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");

    // Issues
    json.push_str("  \"issues\": [\n");
    for (i, issue) in analysis.issues.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"severity\": \"{:?}\",\n", issue.severity));
        json.push_str(&format!("      \"description\": \"{}\",\n", issue.description));
        json.push_str(&format!("      \"resolution\": {}\n",
            issue.resolution.as_ref().map(|r| format!("\"{}\"", r)).unwrap_or_else(|| "null".to_string())));
        json.push_str("    }");
        if i < analysis.issues.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");

    // Recommendations
    json.push_str("  \"recommendations\": [\n");
    for (i, rec) in analysis.recommendations.iter().enumerate() {
        json.push_str(&format!("    \"{}\"", rec));
        if i < analysis.recommendations.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");

    json.push_str("}\n");
    json
}

fn format_matrix_text(matrix: &CompatibilityMatrix) -> String {
    let mut output = String::new();

    output.push_str("================================================================================\n");
    output.push_str("                       VERSION COMPATIBILITY MATRIX\n");
    output.push_str("================================================================================\n\n");

    output.push_str("JAX VERSIONS\n");
    output.push_str("------------\n");
    for entry in &matrix.jax_versions {
        output.push_str(&format!("\nJAX {}\n", entry.version));
        output.push_str(&format!("  Python:    {}-{}\n", entry.python_min, entry.python_max));
        output.push_str(&format!("  jaxlib:    {}\n", entry.jaxlib_version));
        if !entry.libtpu_versions.is_empty() {
            output.push_str(&format!("  libtpu:    {}\n", entry.libtpu_versions.join(", ")));
        }
        if let Some(ref notes) = entry.notes {
            output.push_str(&format!("  Notes:     {}\n", notes));
        }
    }

    if !matrix.known_conflicts.is_empty() {
        output.push_str("\n\nKNOWN CONFLICTS\n");
        output.push_str("---------------\n");
        for conflict in &matrix.known_conflicts {
            output.push_str(&format!("\n  Packages: {}\n", conflict.packages.join(" + ")));
            output.push_str(&format!("  Issue:    {}\n", conflict.description));
            output.push_str(&format!("  Fix:      {}\n", conflict.resolution));
        }
    }

    output.push_str("\n================================================================================\n");

    output
}

fn format_matrix_json(matrix: &CompatibilityMatrix) -> String {
    let mut json = String::new();
    json.push_str("{\n");

    json.push_str(&format!("  \"version\": \"{}\",\n", matrix.version));
    json.push_str(&format!("  \"updated\": \"{}\",\n", matrix.updated));

    json.push_str("  \"jax_versions\": [\n");
    for (i, entry) in matrix.jax_versions.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"version\": \"{}\",\n", entry.version));
        json.push_str(&format!("      \"python_min\": \"{}\",\n", entry.python_min));
        json.push_str(&format!("      \"python_max\": \"{}\",\n", entry.python_max));
        json.push_str(&format!("      \"jaxlib_version\": \"{}\",\n", entry.jaxlib_version));
        json.push_str("      \"libtpu_versions\": [");
        for (j, ltv) in entry.libtpu_versions.iter().enumerate() {
            json.push_str(&format!("\"{}\"", ltv));
            if j < entry.libtpu_versions.len() - 1 {
                json.push_str(", ");
            }
        }
        json.push_str("],\n");
        json.push_str(&format!("      \"notes\": {}\n",
            entry.notes.as_ref().map(|n| format!("\"{}\"", n)).unwrap_or_else(|| "null".to_string())));
        json.push_str("    }");
        if i < matrix.jax_versions.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");

    json.push_str("  \"known_conflicts\": [\n");
    for (i, conflict) in matrix.known_conflicts.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str("      \"packages\": [");
        for (j, pkg) in conflict.packages.iter().enumerate() {
            json.push_str(&format!("\"{}\"", pkg));
            if j < conflict.packages.len() - 1 {
                json.push_str(", ");
            }
        }
        json.push_str("],\n");
        json.push_str(&format!("      \"description\": \"{}\",\n", conflict.description));
        json.push_str(&format!("      \"resolution\": \"{}\"\n", conflict.resolution));
        json.push_str("    }");
        if i < matrix.known_conflicts.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ]\n");

    json.push_str("}\n");
    json
}
