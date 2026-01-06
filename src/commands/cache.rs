//! XLA cache analysis command
//!
//! Analyzes the XLA compilation cache status and health.

use crate::cli::args::{Args, OutputFormat};
use crate::TpuDocError;
use std::env;
use std::fs;
use std::path::Path;

/// XLA cache analysis result
#[derive(Debug)]
pub struct CacheAnalysis {
    pub cache_configured: bool,
    pub cache_path: Option<String>,
    pub cache_exists: bool,
    pub cache_writable: bool,
    pub entry_count: usize,
    pub total_size_mb: f64,
    pub oldest_entry: Option<String>,
    pub newest_entry: Option<String>,
    pub health_status: CacheHealth,
    pub issues: Vec<CacheIssue>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CacheHealth {
    Healthy,
    Warning,
    Error,
    NotConfigured,
}

#[derive(Debug)]
pub struct CacheIssue {
    pub severity: IssueSeverity,
    pub description: String,
}

#[derive(Debug, Clone, Copy)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

impl CacheAnalysis {
    fn default_not_configured() -> Self {
        CacheAnalysis {
            cache_configured: false,
            cache_path: None,
            cache_exists: false,
            cache_writable: false,
            entry_count: 0,
            total_size_mb: 0.0,
            oldest_entry: None,
            newest_entry: None,
            health_status: CacheHealth::NotConfigured,
            issues: vec![CacheIssue {
                severity: IssueSeverity::Info,
                description: "XLA cache is not configured".to_string(),
            }],
            recommendations: vec![
                "Set XLA_FLAGS='--xla_dump_to=/path/to/cache' to enable caching".to_string(),
                "Or use JAX's built-in cache: export JAX_COMPILATION_CACHE_DIR=/path/to/cache".to_string(),
            ],
        }
    }
}

/// Run the cache command
pub fn run(args: &Args) -> Result<String, TpuDocError> {
    let analysis = analyze_cache();

    match args.format {
        OutputFormat::Json => Ok(format_json(&analysis)),
        _ => Ok(format_text(&analysis, args.verbose)),
    }
}

fn analyze_cache() -> CacheAnalysis {
    let mut issues = Vec::new();
    let mut recommendations = Vec::new();

    // Check for cache configuration
    let cache_path = get_cache_path();
    let cache_configured = cache_path.is_some();

    if !cache_configured {
        issues.push(CacheIssue {
            severity: IssueSeverity::Info,
            description: "XLA cache is not configured".to_string(),
        });
        recommendations.push("Set XLA_FLAGS='--xla_dump_to=/path/to/cache' to enable caching".to_string());
        recommendations.push("Or use JAX's built-in cache: export JAX_COMPILATION_CACHE_DIR=/path/to/cache".to_string());

        return CacheAnalysis {
            cache_configured: false,
            cache_path: None,
            cache_exists: false,
            cache_writable: false,
            entry_count: 0,
            total_size_mb: 0.0,
            oldest_entry: None,
            newest_entry: None,
            health_status: CacheHealth::NotConfigured,
            issues,
            recommendations,
        };
    }

    // Safe to unwrap - we returned early if cache_path is None
    let cache_path_str = match cache_path {
        Some(p) => p,
        None => return CacheAnalysis::default_not_configured(),
    };
    let path = Path::new(&cache_path_str);

    // Check if cache directory exists
    let cache_exists = path.exists();
    if !cache_exists {
        issues.push(CacheIssue {
            severity: IssueSeverity::Warning,
            description: format!("Cache directory does not exist: {}", cache_path_str),
        });
        recommendations.push(format!("Create the cache directory: mkdir -p {}", cache_path_str));

        return CacheAnalysis {
            cache_configured: true,
            cache_path: Some(cache_path_str),
            cache_exists: false,
            cache_writable: false,
            entry_count: 0,
            total_size_mb: 0.0,
            oldest_entry: None,
            newest_entry: None,
            health_status: CacheHealth::Warning,
            issues,
            recommendations,
        };
    }

    // Check if cache is writable
    let cache_writable = check_writable(&cache_path_str);
    if !cache_writable {
        issues.push(CacheIssue {
            severity: IssueSeverity::Error,
            description: "Cache directory is not writable".to_string(),
        });
    }

    // Analyze cache contents
    let (entry_count, total_size_bytes, oldest, newest) = analyze_cache_contents(&cache_path_str);
    let total_size_mb = total_size_bytes as f64 / (1024.0 * 1024.0);

    // Check for potential issues
    if total_size_mb > 10240.0 {
        // > 10 GB
        issues.push(CacheIssue {
            severity: IssueSeverity::Warning,
            description: format!("Cache size is very large: {:.1} GB", total_size_mb / 1024.0),
        });
        recommendations.push("Consider clearing old cache entries to free disk space".to_string());
    }

    if entry_count == 0 && cache_exists {
        issues.push(CacheIssue {
            severity: IssueSeverity::Info,
            description: "Cache directory is empty (no compiled modules yet)".to_string(),
        });
    }

    // Check disk space
    if let Some(available_mb) = get_available_disk_space(&cache_path_str) {
        if available_mb < 1024.0 {
            issues.push(CacheIssue {
                severity: IssueSeverity::Warning,
                description: format!("Low disk space: {:.1} MB available", available_mb),
            });
            recommendations.push("Free up disk space to avoid compilation failures".to_string());
        }
    }

    // Determine health status
    let health_status = if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Error)) {
        CacheHealth::Error
    } else if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Warning)) {
        CacheHealth::Warning
    } else {
        CacheHealth::Healthy
    };

    CacheAnalysis {
        cache_configured: true,
        cache_path: Some(cache_path_str),
        cache_exists,
        cache_writable,
        entry_count,
        total_size_mb,
        oldest_entry: oldest,
        newest_entry: newest,
        health_status,
        issues,
        recommendations,
    }
}

fn get_cache_path() -> Option<String> {
    // Check JAX compilation cache directory
    if let Ok(path) = env::var("JAX_COMPILATION_CACHE_DIR") {
        return Some(path);
    }

    // Check XLA_FLAGS for --xla_dump_to
    if let Ok(flags) = env::var("XLA_FLAGS") {
        for part in flags.split_whitespace() {
            if part.starts_with("--xla_dump_to=") {
                return Some(part[14..].to_string());
            }
        }
    }

    // Check common default locations
    let home = env::var("HOME").ok()?;
    let default_path = format!("{}/.cache/jax", home);
    if Path::new(&default_path).exists() {
        return Some(default_path);
    }

    None
}

fn check_writable(path: &str) -> bool {
    let test_file = format!("{}/.tpu_doc_write_test", path);
    if fs::write(&test_file, "test").is_ok() {
        let _ = fs::remove_file(&test_file);
        true
    } else {
        false
    }
}

fn analyze_cache_contents(path: &str) -> (usize, u64, Option<String>, Option<String>) {
    let mut entry_count = 0;
    let mut total_size: u64 = 0;
    let mut oldest_time: Option<std::time::SystemTime> = None;
    let mut newest_time: Option<std::time::SystemTime> = None;
    let mut oldest_name: Option<String> = None;
    let mut newest_name: Option<String> = None;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.is_file() {
                entry_count += 1;
                total_size += metadata.len();

                if let Ok(modified) = metadata.modified() {
                    let entry_name = entry.file_name().to_string_lossy().to_string();

                    let is_oldest = match oldest_time {
                        None => true,
                        Some(t) => modified < t,
                    };
                    if is_oldest {
                        oldest_time = Some(modified);
                        oldest_name = Some(entry_name.clone());
                    }

                    let is_newest = match newest_time {
                        None => true,
                        Some(t) => modified > t,
                    };
                    if is_newest {
                        newest_time = Some(modified);
                        newest_name = Some(entry_name);
                    }
                }
            } else if metadata.is_dir() {
                // Recursively count subdirectory contents
                let subpath = entry.path();
                let (sub_count, sub_size, _, _) = analyze_cache_contents(subpath.to_str().unwrap_or(""));
                entry_count += sub_count;
                total_size += sub_size;
            }
        }
    }

    (entry_count, total_size, oldest_name, newest_name)
}

fn get_available_disk_space(path: &str) -> Option<f64> {
    // Try to get disk space using df command
    use std::process::Command;
    let output = Command::new("df")
        .args(["-m", path])
        .output()
        .ok()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let Ok(available) = parts[3].parse::<f64>() {
                    return Some(available);
                }
            }
        }
    }

    None
}

fn format_text(analysis: &CacheAnalysis, verbose: bool) -> String {
    let mut output = String::new();

    output.push_str("================================================================================\n");
    output.push_str("                         XLA CACHE ANALYSIS\n");
    output.push_str("================================================================================\n\n");

    // Status
    let status_str = match analysis.health_status {
        CacheHealth::Healthy => "HEALTHY",
        CacheHealth::Warning => "WARNING",
        CacheHealth::Error => "ERROR",
        CacheHealth::NotConfigured => "NOT CONFIGURED",
    };
    output.push_str(&format!("Cache Status: {}\n\n", status_str));

    // Configuration
    output.push_str("CONFIGURATION\n");
    output.push_str("-------------\n");
    output.push_str(&format!("  Configured:      {}\n", if analysis.cache_configured { "Yes" } else { "No" }));
    if let Some(ref path) = analysis.cache_path {
        output.push_str(&format!("  Cache Path:      {}\n", path));
    }
    output.push_str(&format!("  Directory Exists: {}\n", if analysis.cache_exists { "Yes" } else { "No" }));
    output.push_str(&format!("  Writable:        {}\n", if analysis.cache_writable { "Yes" } else { "No" }));
    output.push('\n');

    // Contents
    if analysis.cache_exists {
        output.push_str("CACHE CONTENTS\n");
        output.push_str("--------------\n");
        output.push_str(&format!("  Entry Count:     {}\n", analysis.entry_count));
        output.push_str(&format!("  Total Size:      {:.2} MB\n", analysis.total_size_mb));
        if verbose {
            if let Some(ref oldest) = analysis.oldest_entry {
                output.push_str(&format!("  Oldest Entry:    {}\n", oldest));
            }
            if let Some(ref newest) = analysis.newest_entry {
                output.push_str(&format!("  Newest Entry:    {}\n", newest));
            }
        }
        output.push('\n');
    }

    // Issues
    if !analysis.issues.is_empty() {
        output.push_str("ISSUES\n");
        output.push_str("------\n");
        for issue in &analysis.issues {
            let icon = match issue.severity {
                IssueSeverity::Error => "[ERROR]",
                IssueSeverity::Warning => "[WARN] ",
                IssueSeverity::Info => "[INFO] ",
            };
            output.push_str(&format!("  {} {}\n", icon, issue.description));
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

fn format_json(analysis: &CacheAnalysis) -> String {
    let mut json = String::new();
    json.push_str("{\n");

    json.push_str(&format!("  \"health_status\": \"{:?}\",\n", analysis.health_status));
    json.push_str(&format!("  \"cache_configured\": {},\n", analysis.cache_configured));
    json.push_str(&format!("  \"cache_path\": {},\n",
        analysis.cache_path.as_ref().map(|p| format!("\"{}\"", p)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("  \"cache_exists\": {},\n", analysis.cache_exists));
    json.push_str(&format!("  \"cache_writable\": {},\n", analysis.cache_writable));
    json.push_str(&format!("  \"entry_count\": {},\n", analysis.entry_count));
    json.push_str(&format!("  \"total_size_mb\": {:.2},\n", analysis.total_size_mb));
    json.push_str(&format!("  \"oldest_entry\": {},\n",
        analysis.oldest_entry.as_ref().map(|e| format!("\"{}\"", e)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("  \"newest_entry\": {},\n",
        analysis.newest_entry.as_ref().map(|e| format!("\"{}\"", e)).unwrap_or_else(|| "null".to_string())));

    json.push_str("  \"issues\": [\n");
    for (i, issue) in analysis.issues.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"severity\": \"{:?}\",\n", issue.severity));
        json.push_str(&format!("      \"description\": \"{}\"\n", issue.description));
        json.push_str("    }");
        if i < analysis.issues.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");

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
