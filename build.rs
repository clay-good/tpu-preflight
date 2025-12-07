//! Build script for tpu-preflight.
//!
//! This script:
//! 1. Detects if libtpu is available for linking
//! 2. Generates version information from git and environment
//! 3. Sets up static linking configuration

use std::env;
use std::process::Command;

fn main() {
    // Tell cargo to re-run this script if it changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/HEAD");

    // Set the target triple for version info
    println!(
        "cargo:rustc-env=TARGET={}",
        env::var("TARGET").unwrap_or_else(|_| "unknown".to_string())
    );

    // Get git commit hash
    if let Some(hash) = get_git_hash() {
        println!("cargo:rustc-env=TPU_PREFLIGHT_GIT_HASH={}", hash);
    }

    // Get build date
    if let Some(date) = get_build_date() {
        println!("cargo:rustc-env=TPU_PREFLIGHT_BUILD_DATE={}", date);
    }

    // Get rustc version
    if let Some(version) = get_rustc_version() {
        println!("cargo:rustc-env=TPU_PREFLIGHT_RUSTC_VERSION={}", version);
    }

    // Check for libtpu availability
    check_libtpu();

    // Configure static linking for release builds
    configure_static_linking();
}

/// Get the current git commit hash (short form)
fn get_git_hash() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Get the current build date in ISO 8601 format
fn get_build_date() -> Option<String> {
    Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Get the rustc version
fn get_rustc_version() -> Option<String> {
    Command::new("rustc")
        .args(["--version"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().and_then(|s| {
                    // Parse "rustc 1.75.0 (..." -> "1.75.0"
                    s.split_whitespace().nth(1).map(|v| v.to_string())
                })
            } else {
                None
            }
        })
}

/// Check if libtpu is available for linking
fn check_libtpu() {
    // Check standard library paths for libtpu
    let lib_paths = [
        "/usr/local/lib/libtpu.so",
        "/usr/lib/libtpu.so",
        "/opt/libtpu/lib/libtpu.so",
    ];

    let mut found = false;

    for path in lib_paths.iter() {
        if std::path::Path::new(path).exists() {
            println!("cargo:rustc-cfg=libtpu");
            println!("cargo:warning=Found libtpu at {}", path);

            // Get the directory containing libtpu
            if let Some(dir) = std::path::Path::new(path).parent() {
                println!("cargo:rustc-link-search=native={}", dir.display());
            }

            found = true;
            break;
        }
    }

    // Also check TPU_LIBRARY_PATH environment variable
    if !found {
        if let Ok(tpu_lib_path) = env::var("TPU_LIBRARY_PATH") {
            if std::path::Path::new(&tpu_lib_path).exists() {
                println!("cargo:rustc-cfg=libtpu");
                println!(
                    "cargo:warning=Found libtpu via TPU_LIBRARY_PATH at {}",
                    tpu_lib_path
                );

                if let Some(dir) = std::path::Path::new(&tpu_lib_path).parent() {
                    println!("cargo:rustc-link-search=native={}", dir.display());
                }
            }
        }
    }

    if !found {
        println!("cargo:warning=libtpu not found - some hardware checks will be limited");
    }
}

/// Configure static linking for portable binaries
fn configure_static_linking() {
    // For Linux targets, prefer static linking of system libraries
    let target = env::var("TARGET").unwrap_or_default();

    if target.contains("linux") {
        // These settings help create a more portable binary
        // Note: Full static linking requires musl libc or careful configuration

        // Uncomment the following for fully static builds (requires musl):
        // if target.contains("musl") {
        //     println!("cargo:rustc-link-arg=-static");
        // }
    }

    // For macOS, we rely on the standard dynamic linking
    // since static linking of system libraries is not supported
}
