//! Mock platform implementations for testing.
//!
//! Provides configurable mock implementations that simulate:
//! - TPU VM environments (healthy, degraded, failing)
//! - GCP metadata server responses
//! - Linux system information
//! - Network connectivity

use std::collections::HashMap;

/// TPU type for mock configuration
#[derive(Debug, Clone, PartialEq)]
pub enum MockTpuType {
    V4,
    V5e,
    V5p,
    V6e,
    V7,
    None,
}

/// TPU health state for mock configuration
#[derive(Debug, Clone, PartialEq)]
pub enum MockTpuHealth {
    Healthy,
    Degraded,
    ThermalWarning,
    ThermalCritical,
    HbmErrors,
    IciErrors,
    DriverMissing,
}

/// Mock TPU platform configuration
#[derive(Debug, Clone)]
pub struct MockTpuConfig {
    pub is_tpu_vm: bool,
    pub tpu_type: MockTpuType,
    pub chip_count: u32,
    pub health: MockTpuHealth,
    pub hbm_total_gb: f64,
    pub hbm_available_pct: f64,
    pub temperatures: Vec<f64>,
    pub correctable_errors: u64,
    pub uncorrectable_errors: u64,
    pub driver_loaded: bool,
    pub driver_version: Option<String>,
    pub libtpu_version: Option<String>,
    pub ici_bandwidth_gbps: f64,
}

impl Default for MockTpuConfig {
    fn default() -> Self {
        MockTpuConfig {
            is_tpu_vm: false,
            tpu_type: MockTpuType::None,
            chip_count: 0,
            health: MockTpuHealth::Healthy,
            hbm_total_gb: 0.0,
            hbm_available_pct: 95.0,
            temperatures: vec![],
            correctable_errors: 0,
            uncorrectable_errors: 0,
            driver_loaded: false,
            driver_version: None,
            libtpu_version: None,
            ici_bandwidth_gbps: 0.0,
        }
    }
}

impl MockTpuConfig {
    /// Create a healthy v5e-8 TPU configuration
    pub fn healthy_v5e_8() -> Self {
        MockTpuConfig {
            is_tpu_vm: true,
            tpu_type: MockTpuType::V5e,
            chip_count: 8,
            health: MockTpuHealth::Healthy,
            hbm_total_gb: 128.0, // 16GB per chip * 8
            hbm_available_pct: 95.0,
            temperatures: vec![65.0, 66.0, 64.0, 67.0, 65.0, 66.0, 64.0, 65.0],
            correctable_errors: 0,
            uncorrectable_errors: 0,
            driver_loaded: true,
            driver_version: Some("1.0.0".to_string()),
            libtpu_version: Some("0.1.dev20251201".to_string()),
            ici_bandwidth_gbps: 200.0,
        }
    }

    /// Create a healthy v6e-4 TPU configuration
    pub fn healthy_v6e_4() -> Self {
        MockTpuConfig {
            is_tpu_vm: true,
            tpu_type: MockTpuType::V6e,
            chip_count: 4,
            health: MockTpuHealth::Healthy,
            hbm_total_gb: 128.0, // 32GB per chip * 4
            hbm_available_pct: 95.0,
            temperatures: vec![62.0, 63.0, 61.0, 62.0],
            correctable_errors: 0,
            uncorrectable_errors: 0,
            driver_loaded: true,
            driver_version: Some("2.0.0".to_string()),
            libtpu_version: Some("0.2.dev20251201".to_string()),
            ici_bandwidth_gbps: 500.0,
        }
    }

    /// Create a TPU with thermal warning
    pub fn thermal_warning() -> Self {
        let mut config = Self::healthy_v5e_8();
        config.health = MockTpuHealth::ThermalWarning;
        config.temperatures = vec![65.0, 78.0, 64.0, 76.0, 65.0, 77.0, 64.0, 65.0];
        config
    }

    /// Create a TPU with thermal critical
    pub fn thermal_critical() -> Self {
        let mut config = Self::healthy_v5e_8();
        config.health = MockTpuHealth::ThermalCritical;
        config.temperatures = vec![65.0, 88.0, 64.0, 86.0, 65.0, 87.0, 64.0, 65.0];
        config
    }

    /// Create a TPU with HBM errors
    pub fn hbm_errors() -> Self {
        let mut config = Self::healthy_v5e_8();
        config.health = MockTpuHealth::HbmErrors;
        config.uncorrectable_errors = 5;
        config
    }

    /// Create a TPU with correctable errors (warning)
    pub fn correctable_errors() -> Self {
        let mut config = Self::healthy_v5e_8();
        config.health = MockTpuHealth::Degraded;
        config.correctable_errors = 10;
        config
    }

    /// Create a TPU with ICI errors
    pub fn ici_errors() -> Self {
        let mut config = Self::healthy_v5e_8();
        config.health = MockTpuHealth::IciErrors;
        config.ici_bandwidth_gbps = 50.0; // Degraded bandwidth
        config
    }

    /// Create a TPU with missing driver
    pub fn missing_driver() -> Self {
        let mut config = Self::healthy_v5e_8();
        config.health = MockTpuHealth::DriverMissing;
        config.driver_loaded = false;
        config.driver_version = None;
        config
    }

    /// Create a non-TPU VM configuration
    pub fn non_tpu_vm() -> Self {
        MockTpuConfig::default()
    }

    /// Create a single-chip TPU (no ICI)
    pub fn single_chip_v5e() -> Self {
        MockTpuConfig {
            is_tpu_vm: true,
            tpu_type: MockTpuType::V5e,
            chip_count: 1,
            health: MockTpuHealth::Healthy,
            hbm_total_gb: 16.0,
            hbm_available_pct: 95.0,
            temperatures: vec![65.0],
            correctable_errors: 0,
            uncorrectable_errors: 0,
            driver_loaded: true,
            driver_version: Some("1.0.0".to_string()),
            libtpu_version: Some("0.1.dev20251201".to_string()),
            ici_bandwidth_gbps: 0.0, // No ICI for single chip
        }
    }

    /// Create a TPU with low HBM availability
    pub fn low_hbm_availability() -> Self {
        let mut config = Self::healthy_v5e_8();
        config.hbm_available_pct = 60.0;
        config
    }
}

/// Mock GCP platform configuration
#[derive(Debug, Clone)]
pub struct MockGcpConfig {
    pub is_on_gcp: bool,
    pub project_id: Option<String>,
    pub zone: Option<String>,
    pub instance_name: Option<String>,
    pub machine_type: Option<String>,
    pub service_account: Option<String>,
    pub access_scopes: Vec<String>,
    pub attributes: HashMap<String, String>,
}

impl Default for MockGcpConfig {
    fn default() -> Self {
        MockGcpConfig {
            is_on_gcp: false,
            project_id: None,
            zone: None,
            instance_name: None,
            machine_type: None,
            service_account: None,
            access_scopes: vec![],
            attributes: HashMap::new(),
        }
    }
}

impl MockGcpConfig {
    /// Create a standard GCP TPU VM configuration
    pub fn standard_tpu_vm() -> Self {
        let mut attributes = HashMap::new();
        attributes.insert("enable-oslogin".to_string(), "true".to_string());

        MockGcpConfig {
            is_on_gcp: true,
            project_id: Some("my-project".to_string()),
            zone: Some("us-central1-a".to_string()),
            instance_name: Some("tpu-vm-001".to_string()),
            machine_type: Some("ct5lp-hightpu-8t".to_string()),
            service_account: Some("my-sa@my-project.iam.gserviceaccount.com".to_string()),
            access_scopes: vec![
                "https://www.googleapis.com/auth/cloud-platform".to_string(),
            ],
            attributes,
        }
    }

    /// Create a GCP VM with overly permissive service account
    pub fn overly_permissive() -> Self {
        let mut config = Self::standard_tpu_vm();
        config.service_account = Some("123456789-compute@developer.gserviceaccount.com".to_string());
        config.access_scopes = vec![
            "https://www.googleapis.com/auth/cloud-platform".to_string(),
            "https://www.googleapis.com/auth/devstorage.full_control".to_string(),
        ];
        config
    }

    /// Create a GCP VM without OS Login
    pub fn without_os_login() -> Self {
        let mut config = Self::standard_tpu_vm();
        config.attributes.remove("enable-oslogin");
        config
    }

    /// Create a non-GCP configuration
    pub fn non_gcp() -> Self {
        MockGcpConfig::default()
    }
}

/// Mock Linux platform configuration
#[derive(Debug, Clone)]
pub struct MockLinuxConfig {
    pub hostname: String,
    pub kernel_version: String,
    pub memory_total_gb: f64,
    pub memory_available_gb: f64,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub disk_total_gb: f64,
    pub disk_available_gb: f64,
    pub environment: HashMap<String, String>,
}

impl Default for MockLinuxConfig {
    fn default() -> Self {
        MockLinuxConfig {
            hostname: "test-vm".to_string(),
            kernel_version: "5.15.0-generic".to_string(),
            memory_total_gb: 128.0,
            memory_available_gb: 100.0,
            cpu_model: "AMD EPYC".to_string(),
            cpu_cores: 64,
            disk_total_gb: 500.0,
            disk_available_gb: 400.0,
            environment: HashMap::new(),
        }
    }
}

impl MockLinuxConfig {
    /// Create a standard TPU VM Linux configuration
    pub fn standard_tpu_vm() -> Self {
        let mut env = HashMap::new();
        env.insert("TPU_NAME".to_string(), "local".to_string());
        env.insert("TPU_CHIPS_PER_HOST".to_string(), "8".to_string());
        env.insert("PYTHONPATH".to_string(), "/usr/local/lib/python3.11".to_string());

        MockLinuxConfig {
            hostname: "tpu-vm-001".to_string(),
            kernel_version: "5.15.0-1027-gcp".to_string(),
            memory_total_gb: 128.0,
            memory_available_gb: 100.0,
            cpu_model: "AMD EPYC 7B12".to_string(),
            cpu_cores: 64,
            disk_total_gb: 500.0,
            disk_available_gb: 400.0,
            environment: env,
        }
    }

    /// Create configuration with missing TPU environment variables
    pub fn missing_tpu_env() -> Self {
        let mut config = Self::standard_tpu_vm();
        config.environment.remove("TPU_NAME");
        config
    }

    /// Create configuration with low disk space
    pub fn low_disk_space() -> Self {
        let mut config = Self::standard_tpu_vm();
        config.disk_available_gb = 50.0;
        config
    }

    /// Create configuration with Python environment
    pub fn with_python() -> Self {
        let mut config = Self::standard_tpu_vm();
        config.environment.insert("PYTHON_VERSION".to_string(), "3.11.5".to_string());
        config.environment.insert("JAX_VERSION".to_string(), "0.4.35".to_string());
        config
    }

    /// Create configuration with old Python
    pub fn with_old_python() -> Self {
        let mut config = Self::standard_tpu_vm();
        config.environment.insert("PYTHON_VERSION".to_string(), "3.8.0".to_string());
        config
    }
}

/// Mock network configuration
#[derive(Debug, Clone)]
pub struct MockNetworkConfig {
    pub dns_working: bool,
    pub dns_latency_ms: u64,
    pub gcs_reachable: bool,
    pub gcs_latency_ms: u64,
    pub metadata_reachable: bool,
    pub metadata_latency_ms: u64,
    pub compute_reachable: bool,
    pub compute_latency_ms: u64,
    pub exposed_ports: Vec<u16>,
}

impl Default for MockNetworkConfig {
    fn default() -> Self {
        MockNetworkConfig {
            dns_working: true,
            dns_latency_ms: 5,
            gcs_reachable: true,
            gcs_latency_ms: 10,
            metadata_reachable: true,
            metadata_latency_ms: 1,
            compute_reachable: true,
            compute_latency_ms: 15,
            exposed_ports: vec![],
        }
    }
}

impl MockNetworkConfig {
    /// Create a healthy network configuration
    pub fn healthy() -> Self {
        MockNetworkConfig::default()
    }

    /// Create configuration with DNS failure
    pub fn dns_failure() -> Self {
        let mut config = Self::healthy();
        config.dns_working = false;
        config
    }

    /// Create configuration with high latency
    pub fn high_latency() -> Self {
        MockNetworkConfig {
            dns_working: true,
            dns_latency_ms: 50,
            gcs_reachable: true,
            gcs_latency_ms: 100,
            metadata_reachable: true,
            metadata_latency_ms: 20,
            compute_reachable: true,
            compute_latency_ms: 100,
            exposed_ports: vec![],
        }
    }

    /// Create configuration with GCS unreachable
    pub fn gcs_unreachable() -> Self {
        let mut config = Self::healthy();
        config.gcs_reachable = false;
        config
    }

    /// Create configuration with exposed ports
    pub fn with_exposed_ports() -> Self {
        let mut config = Self::healthy();
        config.exposed_ports = vec![22, 8080, 8888];
        config
    }

    /// Create configuration with metadata unreachable
    pub fn metadata_unreachable() -> Self {
        let mut config = Self::healthy();
        config.metadata_reachable = false;
        config
    }
}

/// Complete mock platform configuration
#[derive(Debug, Clone, Default)]
pub struct MockPlatformConfig {
    pub tpu: MockTpuConfig,
    pub gcp: MockGcpConfig,
    pub linux: MockLinuxConfig,
    pub network: MockNetworkConfig,
}

impl MockPlatformConfig {
    /// Create a fully healthy TPU VM configuration
    pub fn healthy_tpu_vm() -> Self {
        MockPlatformConfig {
            tpu: MockTpuConfig::healthy_v5e_8(),
            gcp: MockGcpConfig::standard_tpu_vm(),
            linux: MockLinuxConfig::standard_tpu_vm(),
            network: MockNetworkConfig::healthy(),
        }
    }

    /// Create a healthy v6e TPU VM configuration
    pub fn healthy_v6e_tpu_vm() -> Self {
        MockPlatformConfig {
            tpu: MockTpuConfig::healthy_v6e_4(),
            gcp: MockGcpConfig::standard_tpu_vm(),
            linux: MockLinuxConfig::standard_tpu_vm(),
            network: MockNetworkConfig::healthy(),
        }
    }

    /// Create a non-TPU VM configuration
    pub fn non_tpu_vm() -> Self {
        MockPlatformConfig {
            tpu: MockTpuConfig::non_tpu_vm(),
            gcp: MockGcpConfig::non_gcp(),
            linux: MockLinuxConfig::default(),
            network: MockNetworkConfig::healthy(),
        }
    }

    /// Create a TPU VM with thermal warning
    pub fn thermal_warning() -> Self {
        let mut config = Self::healthy_tpu_vm();
        config.tpu = MockTpuConfig::thermal_warning();
        config
    }

    /// Create a TPU VM with HBM errors
    pub fn hbm_errors() -> Self {
        let mut config = Self::healthy_tpu_vm();
        config.tpu = MockTpuConfig::hbm_errors();
        config
    }

    /// Create a TPU VM with degraded network
    pub fn degraded_network() -> Self {
        let mut config = Self::healthy_tpu_vm();
        config.network = MockNetworkConfig::high_latency();
        config
    }

    /// Create a TPU VM with missing software dependencies
    pub fn missing_dependencies() -> Self {
        let mut config = Self::healthy_tpu_vm();
        config.linux = MockLinuxConfig::missing_tpu_env();
        config.tpu.libtpu_version = None;
        config
    }

    /// Create a TPU VM with security issues
    pub fn security_issues() -> Self {
        let mut config = Self::healthy_tpu_vm();
        config.gcp = MockGcpConfig::overly_permissive();
        config.network = MockNetworkConfig::with_exposed_ports();
        config
    }
}

/// Trait for mock platform operations
pub trait MockPlatform {
    fn get_config(&self) -> &MockPlatformConfig;

    // TPU operations
    fn is_tpu_vm(&self) -> bool {
        self.get_config().tpu.is_tpu_vm
    }

    fn get_tpu_chip_count(&self) -> Option<u32> {
        if self.is_tpu_vm() {
            Some(self.get_config().tpu.chip_count)
        } else {
            None
        }
    }

    fn get_tpu_type_str(&self) -> Option<&str> {
        if self.is_tpu_vm() {
            Some(match self.get_config().tpu.tpu_type {
                MockTpuType::V4 => "v4",
                MockTpuType::V5e => "v5e",
                MockTpuType::V5p => "v5p",
                MockTpuType::V6e => "v6e",
                MockTpuType::V7 => "v7",
                MockTpuType::None => return None,
            })
        } else {
            None
        }
    }

    fn get_hbm_total_bytes(&self) -> Option<u64> {
        if self.is_tpu_vm() {
            Some((self.get_config().tpu.hbm_total_gb * 1024.0 * 1024.0 * 1024.0) as u64)
        } else {
            None
        }
    }

    fn get_hbm_available_bytes(&self) -> Option<u64> {
        if let Some(total) = self.get_hbm_total_bytes() {
            Some((total as f64 * self.get_config().tpu.hbm_available_pct / 100.0) as u64)
        } else {
            None
        }
    }

    fn get_temperatures(&self) -> Option<&[f64]> {
        if self.is_tpu_vm() {
            Some(&self.get_config().tpu.temperatures)
        } else {
            None
        }
    }

    fn get_correctable_errors(&self) -> u64 {
        self.get_config().tpu.correctable_errors
    }

    fn get_uncorrectable_errors(&self) -> u64 {
        self.get_config().tpu.uncorrectable_errors
    }

    fn is_driver_loaded(&self) -> bool {
        self.get_config().tpu.driver_loaded
    }

    fn get_driver_version(&self) -> Option<&str> {
        self.get_config().tpu.driver_version.as_deref()
    }

    fn get_libtpu_version(&self) -> Option<&str> {
        self.get_config().tpu.libtpu_version.as_deref()
    }

    fn get_ici_bandwidth_gbps(&self) -> f64 {
        self.get_config().tpu.ici_bandwidth_gbps
    }

    // GCP operations
    fn is_on_gcp(&self) -> bool {
        self.get_config().gcp.is_on_gcp
    }

    fn get_project_id(&self) -> Option<&str> {
        self.get_config().gcp.project_id.as_deref()
    }

    fn get_zone(&self) -> Option<&str> {
        self.get_config().gcp.zone.as_deref()
    }

    fn get_instance_name(&self) -> Option<&str> {
        self.get_config().gcp.instance_name.as_deref()
    }

    fn get_service_account(&self) -> Option<&str> {
        self.get_config().gcp.service_account.as_deref()
    }

    fn get_access_scopes(&self) -> &[String] {
        &self.get_config().gcp.access_scopes
    }

    fn get_instance_attribute(&self, key: &str) -> Option<&str> {
        self.get_config().gcp.attributes.get(key).map(|s| s.as_str())
    }

    // Linux operations
    fn get_hostname(&self) -> &str {
        &self.get_config().linux.hostname
    }

    fn get_kernel_version(&self) -> &str {
        &self.get_config().linux.kernel_version
    }

    fn get_memory_total_bytes(&self) -> u64 {
        (self.get_config().linux.memory_total_gb * 1024.0 * 1024.0 * 1024.0) as u64
    }

    fn get_memory_available_bytes(&self) -> u64 {
        (self.get_config().linux.memory_available_gb * 1024.0 * 1024.0 * 1024.0) as u64
    }

    fn get_disk_total_bytes(&self) -> u64 {
        (self.get_config().linux.disk_total_gb * 1024.0 * 1024.0 * 1024.0) as u64
    }

    fn get_disk_available_bytes(&self) -> u64 {
        (self.get_config().linux.disk_available_gb * 1024.0 * 1024.0 * 1024.0) as u64
    }

    fn get_env(&self, key: &str) -> Option<&str> {
        self.get_config().linux.environment.get(key).map(|s| s.as_str())
    }

    // Network operations
    fn is_dns_working(&self) -> bool {
        self.get_config().network.dns_working
    }

    fn get_dns_latency_ms(&self) -> u64 {
        self.get_config().network.dns_latency_ms
    }

    fn is_gcs_reachable(&self) -> bool {
        self.get_config().network.gcs_reachable
    }

    fn get_gcs_latency_ms(&self) -> u64 {
        self.get_config().network.gcs_latency_ms
    }

    fn is_metadata_reachable(&self) -> bool {
        self.get_config().network.metadata_reachable
    }

    fn get_metadata_latency_ms(&self) -> u64 {
        self.get_config().network.metadata_latency_ms
    }

    fn get_exposed_ports(&self) -> &[u16] {
        &self.get_config().network.exposed_ports
    }
}

/// A mock platform instance
pub struct MockPlatformInstance {
    config: MockPlatformConfig,
}

impl MockPlatformInstance {
    pub fn new(config: MockPlatformConfig) -> Self {
        MockPlatformInstance { config }
    }

    pub fn healthy_tpu_vm() -> Self {
        Self::new(MockPlatformConfig::healthy_tpu_vm())
    }

    pub fn non_tpu_vm() -> Self {
        Self::new(MockPlatformConfig::non_tpu_vm())
    }
}

impl MockPlatform for MockPlatformInstance {
    fn get_config(&self) -> &MockPlatformConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthy_v5e_8_config() {
        let config = MockTpuConfig::healthy_v5e_8();
        assert!(config.is_tpu_vm);
        assert_eq!(config.chip_count, 8);
        assert_eq!(config.tpu_type, MockTpuType::V5e);
        assert!(config.driver_loaded);
        assert_eq!(config.temperatures.len(), 8);
    }

    #[test]
    fn test_healthy_v6e_4_config() {
        let config = MockTpuConfig::healthy_v6e_4();
        assert!(config.is_tpu_vm);
        assert_eq!(config.chip_count, 4);
        assert_eq!(config.tpu_type, MockTpuType::V6e);
    }

    #[test]
    fn test_thermal_warning_config() {
        let config = MockTpuConfig::thermal_warning();
        assert!(config.temperatures.iter().any(|t| *t > 75.0));
    }

    #[test]
    fn test_thermal_critical_config() {
        let config = MockTpuConfig::thermal_critical();
        assert!(config.temperatures.iter().any(|t| *t > 85.0));
    }

    #[test]
    fn test_non_tpu_config() {
        let config = MockTpuConfig::non_tpu_vm();
        assert!(!config.is_tpu_vm);
        assert_eq!(config.chip_count, 0);
    }

    #[test]
    fn test_mock_platform_instance() {
        let platform = MockPlatformInstance::healthy_tpu_vm();
        assert!(platform.is_tpu_vm());
        assert_eq!(platform.get_tpu_chip_count(), Some(8));
        assert_eq!(platform.get_tpu_type_str(), Some("v5e"));
        assert!(platform.is_on_gcp());
        assert!(platform.is_dns_working());
    }

    #[test]
    fn test_non_tpu_platform() {
        let platform = MockPlatformInstance::non_tpu_vm();
        assert!(!platform.is_tpu_vm());
        assert_eq!(platform.get_tpu_chip_count(), None);
        assert!(!platform.is_on_gcp());
    }

    #[test]
    fn test_gcp_standard_config() {
        let config = MockGcpConfig::standard_tpu_vm();
        assert!(config.is_on_gcp);
        assert!(config.project_id.is_some());
        assert!(config.service_account.is_some());
    }

    #[test]
    fn test_network_healthy_config() {
        let config = MockNetworkConfig::healthy();
        assert!(config.dns_working);
        assert!(config.gcs_reachable);
        assert!(config.exposed_ports.is_empty());
    }

    #[test]
    fn test_network_dns_failure() {
        let config = MockNetworkConfig::dns_failure();
        assert!(!config.dns_working);
    }

    #[test]
    fn test_full_platform_config() {
        let config = MockPlatformConfig::healthy_tpu_vm();
        assert!(config.tpu.is_tpu_vm);
        assert!(config.gcp.is_on_gcp);
        assert!(config.network.dns_working);
    }
}
