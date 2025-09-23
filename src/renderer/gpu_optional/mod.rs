pub mod backend;

use crate::renderer::{Renderer, DataPrecision, RendererError, RendererInfo};
use crate::renderer::factory::{parse_parameters};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

/// Configuration for GPU renderer
#[derive(Debug, Clone, PartialEq)]
pub struct GpuRendererConfig {
    pub device: String,
    pub memory_limit: u64, // Memory limit in bytes
    pub precision: DataPrecision,
}

impl Default for GpuRendererConfig {
    fn default() -> Self {
        Self {
            device: "auto".to_string(),
            memory_limit: 1024 * 1024 * 1024, // 1GB default
            precision: DataPrecision::F32,
        }
    }
}

impl GpuRendererConfig {
    /// Parse configuration from parameter string
    pub fn from_parameters(precision: DataPrecision, parameters: &str) -> Result<Self, RendererError> {
        // GPU renderer only supports F32 precision
        if precision != DataPrecision::F32 {
            return Err(RendererError::UnsupportedPrecision(precision));
        }

        let params = parse_parameters(parameters);
        let mut config = Self {
            precision,
            ..Default::default()
        };

        // Parse device parameter
        if let Some(device) = params.get("device") {
            // Validate device string
            match device.as_str() {
                "auto" | "cuda:0" | "cuda:1" | "cuda:2" | "cuda:3" => {
                    config.device = device.clone();
                }
                _ if device.starts_with("cuda:") => {
                    // Allow any CUDA device ID
                    config.device = device.clone();
                }
                _ => return Err(RendererError::InvalidParameters(
                    format!("Invalid device: {}. Must be 'auto' or 'cuda:N'", device)
                )),
            }
        }

        // Parse memory_limit parameter
        if let Some(memory_str) = params.get("memory_limit") {
            // Support both raw bytes and human-readable formats (MB, GB)
            if let Some(stripped) = memory_str.strip_suffix("GB") {
                let gb = stripped.parse::<u64>()
                    .map_err(|_| RendererError::InvalidParameters(
                        format!("Invalid memory_limit value: {}", memory_str)
                    ))?;
                config.memory_limit = gb * 1024 * 1024 * 1024;
            } else if let Some(stripped) = memory_str.strip_suffix("MB") {
                let mb = stripped.parse::<u64>()
                    .map_err(|_| RendererError::InvalidParameters(
                        format!("Invalid memory_limit value: {}", memory_str)
                    ))?;
                config.memory_limit = mb * 1024 * 1024;
            } else {
                // Raw bytes
                config.memory_limit = memory_str.parse::<u64>()
                    .map_err(|_| RendererError::InvalidParameters(
                        format!("Invalid memory_limit value: {}", memory_str)
                    ))?;
            }

            // Validate minimum memory requirement (64MB)
            if config.memory_limit < 64 * 1024 * 1024 {
                return Err(RendererError::InvalidParameters(
                    "memory_limit must be at least 64MB".to_string()
                ));
            }
        }

        // Validate unsupported parameters
        let supported_params = ["device", "memory_limit"];
        for key in params.keys() {
            if !supported_params.contains(&key.as_str()) {
                return Err(RendererError::InvalidParameters(
                    format!("Unsupported parameter: {}", key)
                ));
            }
        }

        Ok(config)
    }
}

#[derive(Debug)]
pub struct GpuOptionalRenderer {
    config: GpuRendererConfig,
    running: AtomicBool,
    frame_count: AtomicU64,
}

impl GpuOptionalRenderer {
    pub fn new() -> Self {
        Self::with_config(GpuRendererConfig::default())
    }

    pub fn with_config(config: GpuRendererConfig) -> Self {
        Self {
            config,
            running: AtomicBool::new(false),
            frame_count: AtomicU64::new(0),
        }
    }

    /// Returns a reference to the renderer configuration.
    pub fn get_config(&self) -> &GpuRendererConfig {
        &self.config
    }

    /// Returns the current frame count.
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count.load(Ordering::Relaxed)
    }

    /// Resets the frame counter to zero.
    pub fn reset_frame_count(&self) {
        self.frame_count.store(0, Ordering::Relaxed);
    }

    /// Returns whether the renderer is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Simulate rendering a frame.
    pub fn render_frame(&mut self) -> Result<(), String> {
        if !self.is_running() {
            return Err("GpuOptionalRenderer is not running".to_string());
        }

        // Simulate GPU rendering work
        std::thread::sleep(std::time::Duration::from_micros(100));
        self.frame_count.fetch_add(1, Ordering::Relaxed);

        println!("GPU rendered frame {} on device {} with {}MB memory",
                 self.frame_count.load(Ordering::Relaxed),
                 self.config.device,
                 self.config.memory_limit / (1024 * 1024));

        Ok(())
    }
}

impl Clone for GpuOptionalRenderer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            running: AtomicBool::new(self.running.load(Ordering::Relaxed)),
            frame_count: AtomicU64::new(self.frame_count.load(Ordering::Relaxed)),
        }
    }
}

impl Renderer for GpuOptionalRenderer {
    fn start(&mut self) -> Result<(), String> {
        #[cfg(not(feature = "gpu"))]
        {
            return Err("gpu feature not enabled".into());
        }

        #[cfg(feature = "gpu")]
        {
            if self.is_running() {
                return Err("GpuOptionalRenderer is already running".to_string());
            }

            // Initialize GPU backend
            backend::initialize_gpu_backend()?;

            // Simulate GPU initialization with device selection
            match self.config.device.as_str() {
                "auto" => {
                    println!("Initializing GPU renderer with auto device selection");
                }
                device if device.starts_with("cuda:") => {
                    println!("Initializing GPU renderer with device: {}", device);
                }
                _ => {
                    return Err(format!("Unsupported device: {}", self.config.device));
                }
            }

            println!("GPU renderer initialized with {}MB memory limit",
                     self.config.memory_limit / (1024 * 1024));

            self.running.store(true, Ordering::Relaxed);
            println!("GpuOptionalRenderer started");
            Ok(())
        }
    }

    fn stop(&mut self) {
        println!("Stopping GPU renderer. Frames rendered: {}",
                 self.frame_count.load(Ordering::Relaxed));
        self.running.store(false, Ordering::Relaxed);
        println!("GpuOptionalRenderer stopped");
    }

    fn name(&self) -> &'static str {
        "GpuOptional"
    }

    fn render_frame(&mut self) -> Result<(), String> {
        if !self.is_running() {
            return Err("GpuOptionalRenderer is not running".to_string());
        }

        // Simulate GPU rendering work
        std::thread::sleep(std::time::Duration::from_micros(100));
        self.frame_count.fetch_add(1, Ordering::Relaxed);

        println!("GPU rendered frame {} on device {} with {}MB memory",
                 self.frame_count.load(Ordering::Relaxed),
                 self.config.device,
                 self.config.memory_limit / (1024 * 1024));

        Ok(())
    }
}

impl Default for GpuOptionalRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory implementation for GPU renderer
#[derive(Debug)]
pub struct GpuRendererFactory {
    name: String,
}

impl GpuRendererFactory {
    pub fn new() -> Self {
        Self {
            name: "GpuRendererFactory".to_string(),
        }
    }
}

impl crate::renderer::factory::RendererFactory for GpuRendererFactory {
    fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError> {
        // Validate precision support (only F32)
        if precision != DataPrecision::F32 {
            return Err(RendererError::UnsupportedPrecision(precision));
        }

        // Parse and validate configuration
        let config = GpuRendererConfig::from_parameters(precision, parameters)?;

        // Create renderer with configuration
        let renderer = GpuOptionalRenderer::with_config(config);
        Ok(Box::new(renderer))
    }

    fn get_info(&self) -> RendererInfo {
        let mut parameters = HashMap::new();
        parameters.insert("device".to_string(),
                          "GPU device to use: 'auto' or 'cuda:N' (default: auto)".to_string());
        parameters.insert("memory_limit".to_string(),
                          "GPU memory limit in bytes, MB, or GB (default: 1GB)".to_string());

        RendererInfo::new(
            self.name.clone(),
            "gpu,cuda,fast,realtime".to_string(),
            parameters,
            50000, // 50000 microseconds timeout
        )
    }

    fn validate_parameters(&self, precision: DataPrecision, parameters: &str) -> Result<(), RendererError> {
        // Check precision support (only F32)
        if precision != DataPrecision::F32 {
            return Err(RendererError::UnsupportedPrecision(precision));
        }

        // Validate parameters by attempting to parse them
        GpuRendererConfig::from_parameters(precision, parameters)?;
        Ok(())
    }
}

impl Default for GpuRendererFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::factory::RendererFactory;

    #[test]
    fn test_gpu_config_default() {
        let config = GpuRendererConfig::default();
        assert_eq!(config.device, "auto");
        assert_eq!(config.memory_limit, 1024 * 1024 * 1024); // 1GB
        assert_eq!(config.precision, DataPrecision::F32);
    }

    #[test]
    fn test_gpu_config_parsing() {
        // Test default configuration
        let config = GpuRendererConfig::from_parameters(DataPrecision::F32, "").unwrap();
        assert_eq!(config.precision, DataPrecision::F32);
        assert_eq!(config.device, "auto");
        assert_eq!(config.memory_limit, 1024 * 1024 * 1024); // 1GB

        // Test custom configuration
        let config = GpuRendererConfig::from_parameters(
            DataPrecision::F32,
            "device=cuda:0,memory_limit=2GB"
        ).unwrap();
        assert_eq!(config.device, "cuda:0");
        assert_eq!(config.memory_limit, 2 * 1024 * 1024 * 1024); // 2GB

        // Test memory limit parsing
        let config = GpuRendererConfig::from_parameters(
            DataPrecision::F32,
            "memory_limit=512MB"
        ).unwrap();
        assert_eq!(config.memory_limit, 512 * 1024 * 1024);

        // Test invalid precision (only F32 supported)
        assert!(GpuRendererConfig::from_parameters(
            DataPrecision::F64,
            ""
        ).is_err());

        // Test invalid device
        assert!(GpuRendererConfig::from_parameters(
            DataPrecision::F32,
            "device=invalid"
        ).is_err());

        // Test memory limit too small
        assert!(GpuRendererConfig::from_parameters(
            DataPrecision::F32,
            "memory_limit=32MB"
        ).is_err());
    }

    #[test]
    fn test_renderer_with_config() {
        let config = GpuRendererConfig::from_parameters(
            DataPrecision::F32,
            "device=cuda:1,memory_limit=4GB"
        ).unwrap();

        let renderer = GpuOptionalRenderer::with_config(config.clone());
        assert_eq!(renderer.get_config().device, "cuda:1");
        assert_eq!(renderer.get_config().memory_limit, 4 * 1024 * 1024 * 1024);
        assert_eq!(renderer.get_frame_count(), 0);
        assert!(!renderer.is_running());
    }

    #[test]
    fn test_factory_creation() {
        let factory = GpuRendererFactory::new();
        let info = factory.get_info();

        // Test factory info
        assert_eq!(info.name, "GpuRendererFactory");
        assert_eq!(info.timeout_microseconds, 50000);
        assert!(info.has_capability("gpu"));
        assert!(info.has_capability("cuda"));
        assert!(info.has_capability("fast"));
        assert!(info.has_capability("realtime"));

        // Test parameter validation
        assert!(factory.validate_parameters(DataPrecision::F32, "device=cuda:0").is_ok());
        assert!(factory.validate_parameters(DataPrecision::F64, "").is_err());

        // Test renderer creation
        let renderer = factory.create(DataPrecision::F32, "device=auto,memory_limit=512MB").unwrap();
        assert_eq!(renderer.name(), "GpuOptional");
    }

    #[test]
    fn test_memory_limit_formats() {
        // Test GB format
        let config = GpuRendererConfig::from_parameters(DataPrecision::F32, "memory_limit=2GB").unwrap();
        assert_eq!(config.memory_limit, 2 * 1024 * 1024 * 1024);

        // Test MB format
        let config = GpuRendererConfig::from_parameters(DataPrecision::F32, "memory_limit=512MB").unwrap();
        assert_eq!(config.memory_limit, 512 * 1024 * 1024);

        // Test raw bytes
        let config = GpuRendererConfig::from_parameters(DataPrecision::F32, "memory_limit=1073741824").unwrap();
        assert_eq!(config.memory_limit, 1073741824);
    }
}