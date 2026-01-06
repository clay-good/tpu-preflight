//! TPU hardware specifications
//!
//! Provides TPU hardware specifications for different TPU types.

/// TPU type specification
#[derive(Debug, Clone)]
pub struct TpuTypeSpec {
    pub name: String,
    pub hbm_per_chip_gb: u32,
    pub chips_per_host: Vec<u32>,
    pub mxu_count: u32,
    pub bf16_tflops: u32,
    pub ici_bandwidth_gbps: u32,
}

/// TPU specifications database
#[derive(Debug)]
pub struct TpuSpecs {
    pub version: String,
    pub specs: Vec<TpuTypeSpec>,
}

impl TpuSpecs {
    /// Load the embedded TPU specifications
    pub fn load() -> Self {
        TpuSpecs {
            version: "1.0".to_string(),
            specs: vec![
                TpuTypeSpec {
                    name: "v4".to_string(),
                    hbm_per_chip_gb: 32,
                    chips_per_host: vec![4],
                    mxu_count: 2,
                    bf16_tflops: 275,
                    ici_bandwidth_gbps: 4800,
                },
                TpuTypeSpec {
                    name: "v5e".to_string(),
                    hbm_per_chip_gb: 16,
                    chips_per_host: vec![1, 4, 8],
                    mxu_count: 1,
                    bf16_tflops: 197,
                    ici_bandwidth_gbps: 1600,
                },
                TpuTypeSpec {
                    name: "v5p".to_string(),
                    hbm_per_chip_gb: 95,
                    chips_per_host: vec![4],
                    mxu_count: 2,
                    bf16_tflops: 459,
                    ici_bandwidth_gbps: 4800,
                },
                TpuTypeSpec {
                    name: "v6e".to_string(),
                    hbm_per_chip_gb: 32,
                    chips_per_host: vec![1, 4, 8],
                    mxu_count: 1,
                    bf16_tflops: 918,
                    ici_bandwidth_gbps: 3584,
                },
            ],
        }
    }

    /// Get specification for a TPU type
    pub fn get_spec(&self, tpu_type: &str) -> Option<&TpuTypeSpec> {
        self.specs.iter().find(|s| s.name.eq_ignore_ascii_case(tpu_type))
    }

    /// Get expected HBM capacity for a TPU type
    pub fn get_expected_hbm_gb(&self, tpu_type: &str) -> Option<u32> {
        self.get_spec(tpu_type).map(|s| s.hbm_per_chip_gb)
    }

    /// Get expected chip count options for a TPU type
    pub fn get_chip_count_options(&self, tpu_type: &str) -> Option<&[u32]> {
        self.get_spec(tpu_type).map(|s| s.chips_per_host.as_slice())
    }

    /// Check if a chip count is valid for a TPU type
    pub fn is_valid_chip_count(&self, tpu_type: &str, count: u32) -> bool {
        self.get_spec(tpu_type)
            .map(|s| s.chips_per_host.contains(&count))
            .unwrap_or(false)
    }

    /// Get theoretical peak TFLOPS for a TPU type
    pub fn get_peak_tflops(&self, tpu_type: &str) -> Option<u32> {
        self.get_spec(tpu_type).map(|s| s.bf16_tflops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_specs() {
        let specs = TpuSpecs::load();
        assert!(!specs.specs.is_empty());
    }

    #[test]
    fn test_get_spec() {
        let specs = TpuSpecs::load();

        let v4 = specs.get_spec("v4");
        assert!(v4.is_some());
        assert_eq!(v4.unwrap().hbm_per_chip_gb, 32);

        let v5e = specs.get_spec("v5e");
        assert!(v5e.is_some());
        assert_eq!(v5e.unwrap().hbm_per_chip_gb, 16);
    }

    #[test]
    fn test_valid_chip_count() {
        let specs = TpuSpecs::load();

        assert!(specs.is_valid_chip_count("v4", 4));
        assert!(!specs.is_valid_chip_count("v4", 8));

        assert!(specs.is_valid_chip_count("v5e", 1));
        assert!(specs.is_valid_chip_count("v5e", 4));
        assert!(specs.is_valid_chip_count("v5e", 8));
    }
}
