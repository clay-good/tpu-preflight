//! Resource snapshot command
//!
//! Captures a point-in-time view of resource utilization.

use crate::cli::args::{Args, OutputFormat};
use crate::platform::tpu;
use crate::TpuDocError;
use std::fs;
use std::thread;
use std::time::Duration;

/// Resource snapshot
#[derive(Debug)]
pub struct ResourceSnapshot {
    pub timestamp: String,
    pub tpu: TpuResources,
    pub system: SystemResources,
    pub processes: Vec<ProcessInfo>,
    pub io: IoStats,
}

#[derive(Debug)]
pub struct TpuResources {
    pub hbm_utilization_percent: Option<f64>,
    pub duty_cycle_percent: Option<f64>,
    pub temperature_c: Option<f64>,
}

#[derive(Debug)]
pub struct SystemResources {
    pub cpu_utilization_percent: f64,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub memory_percent: f64,
    pub swap_used_gb: f64,
    pub swap_total_gb: f64,
}

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_mb: f64,
    pub cpu_percent: f64,
}

#[derive(Debug)]
pub struct IoStats {
    pub disk_read_mb_s: f64,
    pub disk_write_mb_s: f64,
    pub net_rx_mb_s: f64,
    pub net_tx_mb_s: f64,
}

/// Run the snapshot command
pub fn run(args: &Args) -> Result<String, TpuDocError> {
    if args.continuous > 0 {
        run_continuous(args)
    } else {
        let snapshot = capture_snapshot();
        match args.format {
            OutputFormat::Json => Ok(format_json(&snapshot)),
            _ => Ok(format_text(&snapshot)),
        }
    }
}

fn run_continuous(args: &Args) -> Result<String, TpuDocError> {
    let interval = Duration::from_secs(args.continuous as u64);
    let mut iteration = 0;

    loop {
        // Clear screen (simple approach)
        print!("\x1B[2J\x1B[1;1H");

        let snapshot = capture_snapshot();
        let output = format_text(&snapshot);
        println!("{}", output);
        println!("\nRefreshing every {} seconds... (Ctrl+C to stop)", args.continuous);

        iteration += 1;
        if iteration >= 1000 {
            // Safety limit
            break;
        }

        thread::sleep(interval);
    }

    Ok("Continuous monitoring stopped".to_string())
}

fn capture_snapshot() -> ResourceSnapshot {
    // Get timestamp
    let timestamp = get_timestamp();

    // Capture TPU resources using function-based API
    let thermal_info = tpu::get_thermal_info().ok();
    let avg_temp = thermal_info.as_ref().map(|t| {
        if t.chip_temperatures.is_empty() {
            0.0
        } else {
            t.chip_temperatures.iter().sum::<f64>() / t.chip_temperatures.len() as f64
        }
    });

    // HBM utilization is not directly available without libtpu, use None
    let tpu_resources = TpuResources {
        hbm_utilization_percent: None, // Would need libtpu for actual values
        duty_cycle_percent: None,      // Would need libtpu for actual values
        temperature_c: avg_temp,
    };

    // Capture system resources
    let (mem_used, mem_total) = get_memory_usage();
    let (swap_used, swap_total) = get_swap_usage();
    let cpu_util = get_cpu_utilization();

    let system_resources = SystemResources {
        cpu_utilization_percent: cpu_util,
        memory_used_gb: mem_used,
        memory_total_gb: mem_total,
        memory_percent: if mem_total > 0.0 { (mem_used / mem_total) * 100.0 } else { 0.0 },
        swap_used_gb: swap_used,
        swap_total_gb: swap_total,
    };

    // Capture process information
    let processes = get_top_processes();

    // Capture I/O stats
    let io_stats = get_io_stats();

    ResourceSnapshot {
        timestamp,
        tpu: tpu_resources,
        system: system_resources,
        processes,
        io: io_stats,
    }
}

fn get_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    format!("{:02}:{:02}:{:02} UTC", hours, minutes, seconds)
}

fn get_memory_usage() -> (f64, f64) {
    if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
        let mut mem_total: f64 = 0.0;
        let mut mem_available: f64 = 0.0;

        for line in contents.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(kb) = parse_meminfo_value(line) {
                    mem_total = kb / (1024.0 * 1024.0);
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(kb) = parse_meminfo_value(line) {
                    mem_available = kb / (1024.0 * 1024.0);
                }
            }
        }

        let mem_used = mem_total - mem_available;
        return (mem_used, mem_total);
    }

    (0.0, 0.0)
}

fn get_swap_usage() -> (f64, f64) {
    if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
        let mut swap_total: f64 = 0.0;
        let mut swap_free: f64 = 0.0;

        for line in contents.lines() {
            if line.starts_with("SwapTotal:") {
                if let Some(kb) = parse_meminfo_value(line) {
                    swap_total = kb / (1024.0 * 1024.0);
                }
            } else if line.starts_with("SwapFree:") {
                if let Some(kb) = parse_meminfo_value(line) {
                    swap_free = kb / (1024.0 * 1024.0);
                }
            }
        }

        let swap_used = swap_total - swap_free;
        return (swap_used, swap_total);
    }

    (0.0, 0.0)
}

fn parse_meminfo_value(line: &str) -> Option<f64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse().ok()
    } else {
        None
    }
}

fn get_cpu_utilization() -> f64 {
    // Read /proc/stat twice with a small delay to calculate utilization
    let stat1 = read_cpu_stat();
    thread::sleep(Duration::from_millis(100));
    let stat2 = read_cpu_stat();

    if let (Some((user1, nice1, system1, idle1, iowait1)),
            Some((user2, nice2, system2, idle2, iowait2))) = (stat1, stat2) {
        let total1 = user1 + nice1 + system1 + idle1 + iowait1;
        let total2 = user2 + nice2 + system2 + idle2 + iowait2;
        let idle_diff = (idle2 + iowait2) - (idle1 + iowait1);
        let total_diff = total2 - total1;

        if total_diff > 0 {
            return ((total_diff - idle_diff) as f64 / total_diff as f64) * 100.0;
        }
    }

    0.0
}

fn read_cpu_stat() -> Option<(u64, u64, u64, u64, u64)> {
    let contents = fs::read_to_string("/proc/stat").ok()?;
    for line in contents.lines() {
        if line.starts_with("cpu ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                let user: u64 = parts[1].parse().ok()?;
                let nice: u64 = parts[2].parse().ok()?;
                let system: u64 = parts[3].parse().ok()?;
                let idle: u64 = parts[4].parse().ok()?;
                let iowait: u64 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
                return Some((user, nice, system, idle, iowait));
            }
        }
    }
    None
}

fn get_top_processes() -> Vec<ProcessInfo> {
    let mut processes = Vec::new();

    // Read /proc to get process information
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Only look at numeric directories (PIDs)
            if name_str.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(pid) = name_str.parse::<u32>() {
                    if let Some(info) = get_process_info(pid) {
                        processes.push(info);
                    }
                }
            }
        }
    }

    // Sort by memory usage (descending) and take top 10
    processes.sort_by(|a, b| b.memory_mb.partial_cmp(&a.memory_mb).unwrap_or(std::cmp::Ordering::Equal));
    processes.truncate(10);

    processes
}

fn get_process_info(pid: u32) -> Option<ProcessInfo> {
    // Read process name from /proc/[pid]/comm
    let comm_path = format!("/proc/{}/comm", pid);
    let name = fs::read_to_string(&comm_path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Read memory from /proc/[pid]/status
    let status_path = format!("/proc/{}/status", pid);
    let status = fs::read_to_string(&status_path).ok()?;

    let mut rss_kb: f64 = 0.0;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                rss_kb = parts[1].parse().unwrap_or(0.0);
            }
        }
    }

    let memory_mb = rss_kb / 1024.0;

    // Skip processes with negligible memory
    if memory_mb < 1.0 {
        return None;
    }

    Some(ProcessInfo {
        pid,
        name,
        memory_mb,
        cpu_percent: 0.0, // Would need more complex calculation
    })
}

fn get_io_stats() -> IoStats {
    // This is a simplified implementation
    // Real implementation would read /proc/diskstats and /proc/net/dev twice
    IoStats {
        disk_read_mb_s: 0.0,
        disk_write_mb_s: 0.0,
        net_rx_mb_s: 0.0,
        net_tx_mb_s: 0.0,
    }
}

fn format_text(snapshot: &ResourceSnapshot) -> String {
    let mut output = String::new();

    output.push_str("================================================================================\n");
    output.push_str("                         RESOURCE SNAPSHOT\n");
    output.push_str("================================================================================\n\n");
    output.push_str(&format!("Timestamp: {}\n\n", snapshot.timestamp));

    // TPU Resources
    output.push_str("TPU RESOURCES\n");
    output.push_str("-------------\n");
    output.push_str(&format!("  HBM Utilization:  {}\n",
        snapshot.tpu.hbm_utilization_percent
            .map(|v| format!("{:.1}%", v))
            .unwrap_or_else(|| "N/A".to_string())));
    output.push_str(&format!("  Duty Cycle:       {}\n",
        snapshot.tpu.duty_cycle_percent
            .map(|v| format!("{:.1}%", v))
            .unwrap_or_else(|| "N/A".to_string())));
    output.push_str(&format!("  Temperature:      {}\n",
        snapshot.tpu.temperature_c
            .map(|v| format!("{:.1}C", v))
            .unwrap_or_else(|| "N/A".to_string())));
    output.push('\n');

    // System Resources
    output.push_str("SYSTEM RESOURCES\n");
    output.push_str("----------------\n");
    output.push_str(&format!("  CPU Utilization:  {:.1}%\n", snapshot.system.cpu_utilization_percent));
    output.push_str(&format!("  Memory:           {:.1} / {:.1} GB ({:.1}%)\n",
        snapshot.system.memory_used_gb,
        snapshot.system.memory_total_gb,
        snapshot.system.memory_percent));
    if snapshot.system.swap_total_gb > 0.0 {
        output.push_str(&format!("  Swap:             {:.1} / {:.1} GB\n",
            snapshot.system.swap_used_gb,
            snapshot.system.swap_total_gb));
    }
    output.push('\n');

    // Top Processes
    if !snapshot.processes.is_empty() {
        output.push_str("TOP PROCESSES (by memory)\n");
        output.push_str("-------------------------\n");
        output.push_str(&format!("  {:>6}  {:20}  {:>10}\n", "PID", "NAME", "MEMORY"));
        for proc in &snapshot.processes {
            output.push_str(&format!("  {:>6}  {:20}  {:>8.1} MB\n",
                proc.pid, proc.name, proc.memory_mb));
        }
        output.push('\n');
    }

    // I/O Stats (if available)
    if snapshot.io.disk_read_mb_s > 0.0 || snapshot.io.disk_write_mb_s > 0.0 {
        output.push_str("I/O STATISTICS\n");
        output.push_str("--------------\n");
        output.push_str(&format!("  Disk Read:        {:.1} MB/s\n", snapshot.io.disk_read_mb_s));
        output.push_str(&format!("  Disk Write:       {:.1} MB/s\n", snapshot.io.disk_write_mb_s));
        output.push_str(&format!("  Network RX:       {:.1} MB/s\n", snapshot.io.net_rx_mb_s));
        output.push_str(&format!("  Network TX:       {:.1} MB/s\n", snapshot.io.net_tx_mb_s));
        output.push('\n');
    }

    output.push_str("================================================================================\n");

    output
}

fn format_json(snapshot: &ResourceSnapshot) -> String {
    let mut json = String::new();
    json.push_str("{\n");

    json.push_str(&format!("  \"timestamp\": \"{}\",\n", snapshot.timestamp));

    // TPU
    json.push_str("  \"tpu\": {\n");
    json.push_str(&format!("    \"hbm_utilization_percent\": {},\n",
        snapshot.tpu.hbm_utilization_percent.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"duty_cycle_percent\": {},\n",
        snapshot.tpu.duty_cycle_percent.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string())));
    json.push_str(&format!("    \"temperature_c\": {}\n",
        snapshot.tpu.temperature_c.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string())));
    json.push_str("  },\n");

    // System
    json.push_str("  \"system\": {\n");
    json.push_str(&format!("    \"cpu_utilization_percent\": {:.1},\n", snapshot.system.cpu_utilization_percent));
    json.push_str(&format!("    \"memory_used_gb\": {:.1},\n", snapshot.system.memory_used_gb));
    json.push_str(&format!("    \"memory_total_gb\": {:.1},\n", snapshot.system.memory_total_gb));
    json.push_str(&format!("    \"memory_percent\": {:.1},\n", snapshot.system.memory_percent));
    json.push_str(&format!("    \"swap_used_gb\": {:.1},\n", snapshot.system.swap_used_gb));
    json.push_str(&format!("    \"swap_total_gb\": {:.1}\n", snapshot.system.swap_total_gb));
    json.push_str("  },\n");

    // Processes
    json.push_str("  \"processes\": [\n");
    for (i, proc) in snapshot.processes.iter().enumerate() {
        json.push_str("    {\n");
        json.push_str(&format!("      \"pid\": {},\n", proc.pid));
        json.push_str(&format!("      \"name\": \"{}\",\n", proc.name));
        json.push_str(&format!("      \"memory_mb\": {:.1}\n", proc.memory_mb));
        json.push_str("    }");
        if i < snapshot.processes.len() - 1 {
            json.push(',');
        }
        json.push('\n');
    }
    json.push_str("  ],\n");

    // I/O
    json.push_str("  \"io\": {\n");
    json.push_str(&format!("    \"disk_read_mb_s\": {:.1},\n", snapshot.io.disk_read_mb_s));
    json.push_str(&format!("    \"disk_write_mb_s\": {:.1},\n", snapshot.io.disk_write_mb_s));
    json.push_str(&format!("    \"net_rx_mb_s\": {:.1},\n", snapshot.io.net_rx_mb_s));
    json.push_str(&format!("    \"net_tx_mb_s\": {:.1}\n", snapshot.io.net_tx_mb_s));
    json.push_str("  }\n");

    json.push_str("}\n");
    json
}
