//! Environment info command
//!
//! Displays complete environment fingerprint without making pass/fail judgments.

use crate::cli::args::{Args, OutputFormat};
use crate::platform::{gcp, linux, tpu};
use crate::TpuDocError;
use std::env;

/// Environment information structure
#[derive(Debug)]
pub struct EnvironmentInfo {
    pub timestamp: String,
    pub tpu: TpuInfo,
    pub software: SoftwareInfo,
    pub system: SystemInfo,
    pub gcp: GcpInfo,
    pub network: NetworkInfo,
}

#[derive(Debug)]
pub struct TpuInfo {
    pub tpu_type: String,
    pub chip_count: Option<u32>,
    pub topology: Option<String>,
    pub hbm_capacity_gb: Option<u32>,
    pub machine_type: Option<String>,
}

#[derive(Debug)]
pub struct SoftwareInfo {
    pub python_version: Option<String>,
    pub jax_version: Option<String>,
    pub jaxlib_version: Option<String>,
    pub libtpu_version: Option<String>,
    pub numpy_version: Option<String>,
    pub env_vars: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct SystemInfo {
    pub hostname: String,
    pub kernel_version: String,
    pub total_memory_gb: f64,
    pub cpu_count: u32,
}

#[derive(Debug)]
pub struct GcpInfo {
    pub project_id: Option<String>,
    pub zone: Option<String>,
    pub instance_name: Option<String>,
    pub service_account: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug)]
pub struct NetworkInfo {
    pub internal_ip: Option<String>,
    pub external_ip: Option<String>,
}

/// Run the info command
pub fn run(args: &Args) -> Result<String, TpuDocError> {
    let info = gather_environment_info();

    match args.format {
        OutputFormat::Json => Ok(format_json(&info)),
        _ => Ok(format_text(&info, args.verbose)),
    }
}

/// Internal function to gather environment info (for use by other modules)
pub fn gather_environment_info_internal() -> EnvironmentInfo {
    gather_environment_info()
}

fn gather_environment_info() -> EnvironmentInfo {
    // Get current timestamp
    let timestamp = get_iso_timestamp();

    // Gather TPU information
    let tpu_type_result = tpu::get_tpu_type();
    let tpu_info = TpuInfo {
        tpu_type: tpu_type_result.as_ref().map(|t| t.to_string()).unwrap_or_else(|_| "Unknown".to_string()),
        chip_count: tpu::get_tpu_chip_count().ok(),
        topology: tpu::get_tpu_topology().ok().map(|t| t.shape),
        hbm_capacity_gb: tpu::get_hbm_info().ok().map(|h| (h.total_bytes / (1024 * 1024 * 1024)) as u32),
        machine_type: gcp::get_machine_type().ok(),
    };

    // Gather software information
    let software_info = SoftwareInfo {
        python_version: detect_python_version(),
        jax_version: detect_jax_version(),
        jaxlib_version: detect_jaxlib_version(),
        libtpu_version: detect_libtpu_version(),
        numpy_version: detect_numpy_version(),
        env_vars: get_relevant_env_vars(),
    };

    // Gather system information
    let mem_info = linux::get_memory_info().ok();
    let cpu_info = linux::get_cpu_info().ok();

    let system_info = SystemInfo {
        hostname: linux::get_hostname().unwrap_or_else(|_| "Unknown".to_string()),
        kernel_version: linux::get_kernel_version().unwrap_or_else(|_| "Unknown".to_string()),
        total_memory_gb: mem_info.map(|m| m.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)).unwrap_or(0.0),
        cpu_count: cpu_info.map(|c| c.cores).unwrap_or(0),
    };

    // Gather GCP information
    let gcp_info = GcpInfo {
        project_id: gcp::get_project_id().ok(),
        zone: gcp::get_zone().ok(),
        instance_name: gcp::get_instance_name().ok(),
        service_account: gcp::get_service_account().ok(),
        scopes: gcp::get_access_scopes().unwrap_or_default(),
    };

    // Gather network information
    let network_info = NetworkInfo {
        internal_ip: gcp::get_instance_attribute("internal-ip").ok().flatten(),
        external_ip: gcp::get_instance_attribute("external-ip").ok().flatten(),
    };

    EnvironmentInfo {
        timestamp,
        tpu: tpu_info,
        software: software_info,
        system: system_info,
        gcp: gcp_info,
        network: network_info,
    }
}

fn get_iso_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Simple ISO 8601 format (UTC)
    let days_since_1970 = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year, month, day (simplified)
    let mut year = 1970;
    let mut remaining_days = days_since_1970;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days in days_in_months.iter() {
        if remaining_days < *days {
            break;
        }
        remaining_days -= days;
        month += 1;
    }
    let day = remaining_days + 1;

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, minutes, seconds)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn detect_python_version() -> Option<String> {
    use std::process::Command;
    let output = Command::new("python3")
        .args(["--version"])
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        let version = version.trim();
        if version.starts_with("Python ") {
            return Some(version[7..].to_string());
        }
        Some(version.to_string())
    } else {
        None
    }
}

fn detect_jax_version() -> Option<String> {
    // Try environment variable first
    if let Ok(version) = env::var("JAX_VERSION") {
        return Some(version);
    }

    // Try Python import
    use std::process::Command;
    let output = Command::new("python3")
        .args(["-c", "import jax; print(jax.__version__)"])
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        Some(version.trim().to_string())
    } else {
        None
    }
}

fn detect_jaxlib_version() -> Option<String> {
    use std::process::Command;
    let output = Command::new("python3")
        .args(["-c", "import jaxlib; print(jaxlib.__version__)"])
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        Some(version.trim().to_string())
    } else {
        None
    }
}

fn detect_libtpu_version() -> Option<String> {
    // Check environment variable
    if let Ok(version) = env::var("LIBTPU_VERSION") {
        return Some(version);
    }

    tpu::get_libtpu_version().ok()
}

fn detect_numpy_version() -> Option<String> {
    use std::process::Command;
    let output = Command::new("python3")
        .args(["-c", "import numpy; print(numpy.__version__)"])
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        Some(version.trim().to_string())
    } else {
        None
    }
}

fn get_relevant_env_vars() -> Vec<(String, String)> {
    let relevant_vars = [
        "TPU_NAME",
        "TPU_CHIPS_PER_HOST_BOUNDS",
        "CLOUD_TPU_TASK_ID",
        "TPU_WORKER_ID",
        "TPU_WORKER_HOSTNAMES",
        "XLA_FLAGS",
        "TF_CPP_MIN_LOG_LEVEL",
        "JAX_PLATFORMS",
        "JAX_DEBUG_NANS",
        "XLA_PYTHON_CLIENT_PREALLOCATE",
        "XLA_PYTHON_CLIENT_MEM_FRACTION",
        "TPU_LIBRARY_PATH",
        "LIBTPU_INIT_ARGS",
    ];

    relevant_vars
        .iter()
        .filter_map(|&name| {
            env::var(name).ok().map(|value| (name.to_string(), value))
        })
        .collect()
}

fn format_text(info: &EnvironmentInfo, verbose: bool) -> String {
    let mut output = String::new();

    output.push_str("================================================================================\n");
    output.push_str("                         TPU ENVIRONMENT INFORMATION\n");
    output.push_str("================================================================================\n\n");
    output.push_str(&format!("Timestamp: {}\n\n", info.timestamp));

    // TPU Information
    output.push_str("TPU INFORMATION\n");
    output.push_str("---------------\n");
    output.push_str(&format!("  TPU Type:        {}\n", info.tpu.tpu_type));
    output.push_str(&format!("  Chip Count:      {}\n",
        info.tpu.chip_count.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())));
    output.push_str(&format!("  Topology:        {}\n",
        info.tpu.topology.as_deref().unwrap_or("N/A")));
    output.push_str(&format!("  HBM Capacity:    {} GB\n",
        info.tpu.hbm_capacity_gb.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())));
    output.push_str(&format!("  Machine Type:    {}\n",
        info.tpu.machine_type.as_deref().unwrap_or("N/A")));
    output.push('\n');

    // Software Stack
    output.push_str("SOFTWARE STACK\n");
    output.push_str("--------------\n");
    output.push_str(&format!("  Python:          {}\n",
        info.software.python_version.as_deref().unwrap_or("N/A")));
    output.push_str(&format!("  JAX:             {}\n",
        info.software.jax_version.as_deref().unwrap_or("N/A")));
    output.push_str(&format!("  jaxlib:          {}\n",
        info.software.jaxlib_version.as_deref().unwrap_or("N/A")));
    output.push_str(&format!("  libtpu:          {}\n",
        info.software.libtpu_version.as_deref().unwrap_or("N/A")));
    output.push_str(&format!("  NumPy:           {}\n",
        info.software.numpy_version.as_deref().unwrap_or("N/A")));
    output.push('\n');

    // Environment Variables
    if !info.software.env_vars.is_empty() {
        output.push_str("ENVIRONMENT VARIABLES\n");
        output.push_str("---------------------\n");
        for (name, value) in &info.software.env_vars {
            let display_value = if value.len() > 60 && !verbose {
                format!("{}...", &value[..60])
            } else {
                value.clone()
            };
            output.push_str(&format!("  {}={}\n", name, display_value));
        }
        output.push('\n');
    }

    // System Information
    output.push_str("SYSTEM INFORMATION\n");
    output.push_str("------------------\n");
    output.push_str(&format!("  Hostname:        {}\n", info.system.hostname));
    output.push_str(&format!("  Kernel:          {}\n", info.system.kernel_version));
    output.push_str(&format!("  Memory:          {:.1} GB\n", info.system.total_memory_gb));
    output.push_str(&format!("  CPU Count:       {}\n", info.system.cpu_count));
    output.push('\n');

    // GCP Information
    output.push_str("GCP INFORMATION\n");
    output.push_str("---------------\n");
    output.push_str(&format!("  Project:         {}\n",
        info.gcp.project_id.as_deref().unwrap_or("N/A")));
    output.push_str(&format!("  Zone:            {}\n",
        info.gcp.zone.as_deref().unwrap_or("N/A")));
    output.push_str(&format!("  Instance:        {}\n",
        info.gcp.instance_name.as_deref().unwrap_or("N/A")));
    output.push_str(&format!("  Service Account: {}\n",
        info.gcp.service_account.as_deref().unwrap_or("N/A")));

    if verbose && !info.gcp.scopes.is_empty() {
        output.push_str("  Scopes:\n");
        for scope in &info.gcp.scopes {
            output.push_str(&format!("    - {}\n", scope));
        }
    }
    output.push('\n');

    // Network Information
    output.push_str("NETWORK INFORMATION\n");
    output.push_str("-------------------\n");
    output.push_str(&format!("  Internal IP:     {}\n",
        info.network.internal_ip.as_deref().unwrap_or("N/A")));
    output.push_str(&format!("  External IP:     {}\n",
        info.network.external_ip.as_deref().unwrap_or("N/A")));

    output.push_str("\n================================================================================\n");

    output
}

fn format_json(info: &EnvironmentInfo) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"timestamp\": \"{}\",\n", info.timestamp));

    // TPU
    json.push_str("  \"tpu\": {\n");
    json.push_str(&format!("    \"type\": \"{}\",\n", info.tpu.tpu_type));
    json.push_str(&format!("    \"chip_count\": {},\n",
        info.tpu.chip_count.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"topology\": {},\n",
        info.tpu.topology.as_ref().map(|t| format!("\"{}\"", t)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"hbm_capacity_gb\": {},\n",
        info.tpu.hbm_capacity_gb.map(|c| c.to_string()).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"machine_type\": {}\n",
        info.tpu.machine_type.as_ref().map(|t| format!("\"{}\"", t)).unwrap_or_else(|| "null".to_string())));
    json.push_str("  },\n");

    // Software
    json.push_str("  \"software\": {\n");
    json.push_str(&format!("    \"python_version\": {},\n",
        info.software.python_version.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"jax_version\": {},\n",
        info.software.jax_version.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"jaxlib_version\": {},\n",
        info.software.jaxlib_version.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"libtpu_version\": {},\n",
        info.software.libtpu_version.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"numpy_version\": {},\n",
        info.software.numpy_version.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str("    \"env_vars\": {\n");
    for (i, (name, value)) in info.software.env_vars.iter().enumerate() {
        let escaped_value = value.replace('\\', "\\\\").replace('"', "\\\"");
        let comma = if i < info.software.env_vars.len() - 1 { "," } else { "" };
        json.push_str(&format!("      \"{}\": \"{}\"{}\n", name, escaped_value, comma));
    }
    json.push_str("    }\n");
    json.push_str("  },\n");

    // System
    json.push_str("  \"system\": {\n");
    json.push_str(&format!("    \"hostname\": \"{}\",\n", info.system.hostname));
    json.push_str(&format!("    \"kernel_version\": \"{}\",\n", info.system.kernel_version));
    json.push_str(&format!("    \"total_memory_gb\": {:.1},\n", info.system.total_memory_gb));
    json.push_str(&format!("    \"cpu_count\": {}\n", info.system.cpu_count));
    json.push_str("  },\n");

    // GCP
    json.push_str("  \"gcp\": {\n");
    json.push_str(&format!("    \"project_id\": {},\n",
        info.gcp.project_id.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"zone\": {},\n",
        info.gcp.zone.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"instance_name\": {},\n",
        info.gcp.instance_name.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"service_account\": {},\n",
        info.gcp.service_account.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str("    \"scopes\": [\n");
    for (i, scope) in info.gcp.scopes.iter().enumerate() {
        let comma = if i < info.gcp.scopes.len() - 1 { "," } else { "" };
        json.push_str(&format!("      \"{}\"{}\n", scope, comma));
    }
    json.push_str("    ]\n");
    json.push_str("  },\n");

    // Network
    json.push_str("  \"network\": {\n");
    json.push_str(&format!("    \"internal_ip\": {},\n",
        info.network.internal_ip.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"external_ip\": {}\n",
        info.network.external_ip.as_ref().map(|v| format!("\"{}\"", v)).unwrap_or_else(|| "null".to_string())));
    json.push_str("  }\n");

    json.push_str("}\n");
    json
}
