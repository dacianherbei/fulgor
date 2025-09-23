//! CPU reference renderer for the fulgor library
//!
//! This module provides a reference implementation of a CPU-based renderer
//! for 3D Gaussian Splatting primitives. It serves as a baseline implementation
//! for testing and development purposes.

use crate::renderer::{Renderer, DataPrecision, RendererError, RendererInfo};
use crate::renderer::factory::{parse_parameters, RendererFactory};
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Add, AddAssign};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

/// Configuration for CpuReferenceRenderer
#[derive(Debug, Clone, PartialEq)]
pub struct CpuReferenceConfig {
    pub threads: usize,
    pub quality: String,
    pub debug: bool,
    pub precision: DataPrecision,
}

impl Default for CpuReferenceConfig {
    fn default() -> Self {
        Self {
            threads: num_cpus::get(),
            quality: "medium".to_string(),
            debug: false,
            precision: DataPrecision::F32,
        }
    }
}

impl CpuReferenceConfig {
    /// Parse configuration from parameter string
    pub fn from_parameters(precision: DataPrecision, parameters: &str) -> Result<Self, RendererError> {
        let params = parse_parameters(parameters);
        let mut config = Self {
            precision,
            ..Default::default()
        };

        // Parse threads parameter
        if let Some(threads_str) = params.get("threads") {
            config.threads = threads_str.parse::<usize>()
                .map_err(|_| RendererError::InvalidParameters(
                    format!("Invalid threads value: {}", threads_str)
                ))?;

            if config.threads == 0 {
                return Err(RendererError::InvalidParameters(
                    "threads must be greater than 0".to_string()
                ));
            }
        }

        // Parse quality parameter
        if let Some(quality) = params.get("quality") {
            match quality.as_str() {
                "low" | "medium" | "high" | "ultra" => {
                    config.quality = quality.clone();
                }
                _ => return Err(RendererError::InvalidParameters(
                    format!("Invalid quality value: {}. Must be one of: low, medium, high, ultra", quality)
                )),
            }
        }

        // Parse debug parameter
        if let Some(debug_str) = params.get("debug") {
            config.debug = debug_str.parse::<bool>()
                .map_err(|_| RendererError::InvalidParameters(
                    format!("Invalid debug value: {}", debug_str)
                ))?;
        }

        // Validate unsupported parameters
        let supported_params = ["threads", "quality", "debug"];
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

/// A CPU-based reference renderer for 3D Gaussian Splatting.
///
/// This renderer provides a template-able implementation that can work with
/// different precision levels. The template parameter `T` represents the
/// numeric type used for internal calculations.
///
/// # Type Parameters
///
/// * `T` - The numeric type for calculations (typically `f32` or `f64`)
///
/// # Examples
///
/// ```rust
/// use fulgor::renderer::cpu_reference::{CpuReferenceConfig, CpuReferenceRenderer};
/// use fulgor::renderer::DataPrecision;
///
/// // Create a single-precision renderer
/// let mut renderer_f32: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
///
/// // Create a double-precision renderer with configuration
/// let config = CpuReferenceConfig::from_parameters(DataPrecision::F64, "threads=4,debug=true")?;
/// let mut renderer_f64: CpuReferenceRenderer<f64> = CpuReferenceRenderer::with_config(config);
/// ```
#[derive(Debug)]
pub struct CpuReferenceRenderer<NumberType = f32>
where
    NumberType: Copy + Default + PartialEq + Add<Output = NumberType> + AddAssign + Send + Sync + std::fmt::Debug + 'static,
{
    /// Configuration for this renderer instance
    config: CpuReferenceConfig,

    /// Indicates whether the renderer is currently running
    is_running: AtomicBool,

    /// Total number of frames rendered since creation or last reset
    frame_count: AtomicU64,

    /// Phantom data to maintain the template parameter
    _phantom: std::marker::PhantomData<NumberType>,
}

impl<NumberType> CpuReferenceRenderer<NumberType>
where
    NumberType: Copy + Default + PartialEq + Add<Output = NumberType> + AddAssign + Send + Sync + std::fmt::Debug + 'static,
{
    /// Creates a new `CpuReferenceRenderer` instance with default configuration.
    ///
    /// The renderer starts in a stopped state with zero frames rendered.
    ///
    /// # Returns
    ///
    /// A new `CpuReferenceRenderer` with:
    /// - `is_running`: `false`
    /// - `frame_count`: `0`
    /// - Default configuration
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fulgor::renderer::cpu_reference::CpuReferenceRenderer;
    ///
    /// let renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
    /// assert_eq!(renderer.get_frame_count(), 0);
    /// ```
    pub fn new() -> Self {
        Self::with_config(CpuReferenceConfig::default())
    }

    /// Creates a new `CpuReferenceRenderer` instance with the specified configuration.
    ///
    /// # Arguments
    /// * `config` - Configuration parameters for the renderer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fulgor::renderer::cpu_reference::{CpuReferenceRenderer, CpuReferenceConfig};
    /// use fulgor::renderer::DataPrecision;
    ///
    /// let config = CpuReferenceConfig::from_parameters(
    ///     DataPrecision::F32,
    ///     "threads=4,quality=high,debug=true"
    /// ).unwrap();
    /// let renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::with_config(config);
    /// ```
    pub fn with_config(config: CpuReferenceConfig) -> Self {
        Self {
            config,
            is_running: AtomicBool::new(false),
            frame_count: AtomicU64::new(0),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns the current frame count.
    ///
    /// This method provides access to the total number of frames
    /// that have been rendered since the renderer was created
    /// or since the last reset.
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count.load(Ordering::Relaxed)
    }

    /// Resets the frame counter to zero.
    ///
    /// This method can be useful for benchmarking or when
    /// starting a new rendering session.
    pub fn reset_frame_count(&self) {
        self.frame_count.store(0, Ordering::Relaxed);
    }

    /// Returns a reference to the renderer configuration.
    pub fn get_config(&self) -> &CpuReferenceConfig {
        &self.config
    }

    /// Returns whether the renderer is currently running.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Simulate rendering a frame (for testing/demo purposes).
    ///
    /// In a real implementation, this would perform the actual 3D Gaussian splatting.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the frame was rendered successfully, or an error message if
    /// the renderer is not running.
    pub fn render_frame(&mut self) -> Result<(), String> {
        if !self.is_running() {
            return Err("CpuReferenceRenderer is not running".to_string());
        }

        if self.config.debug {
            println!("Rendering frame {} with {} threads at {} quality",
                     self.frame_count.load(Ordering::Relaxed) + 1,
                     self.config.threads,
                     self.config.quality);
        }

        // Simulate frame rendering work
        self.frame_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

impl<NumberType> Clone for CpuReferenceRenderer<NumberType>
where
    NumberType: Copy + Default + PartialEq + Add<Output = NumberType> + AddAssign + Send + Sync + std::fmt::Debug + 'static,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            is_running: AtomicBool::new(self.is_running.load(Ordering::Relaxed)),
            frame_count: AtomicU64::new(self.frame_count.load(Ordering::Relaxed)),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<NumberType> Renderer for CpuReferenceRenderer<NumberType>
where
    NumberType: Copy + Default + PartialEq + Add<Output = NumberType> + AddAssign + Send + Sync + std::fmt::Debug + 'static,
{
    /// Start the renderer.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the renderer was started successfully, or an error message if
    /// the renderer is already running.
    fn start(&mut self) -> Result<(), String> {
        if self.is_running() {
            return Err("CpuReferenceRenderer is already running".to_string());
        }

        if self.config.debug {
            println!("Starting CpuReferenceRenderer with config: {:?}", self.config);
        }

        self.is_running.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Stop the renderer.
    ///
    /// This method always succeeds and can be called multiple times safely.
    fn stop(&mut self) {
        if self.config.debug {
            println!("Stopping CpuReferenceRenderer. Frames rendered: {}",
                     self.frame_count.load(Ordering::Relaxed));
        }

        self.is_running.store(false, Ordering::Relaxed);
    }

    fn render_frame(&mut self) -> Result<(), String> {
        if !self.is_running() {
            return Err("CpuReferenceRenderer is not running".to_string());
        }

        if self.config.debug {
            println!("Rendering frame {} with {} threads at {} quality",
                     self.frame_count.load(Ordering::Relaxed) + 1,
                     self.config.threads,
                     self.config.quality);
        }

        // Simulate frame rendering work
        self.frame_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Get the renderer's name.
    fn name(&self) -> &'static str {
        "CpuReference"
    }
}

impl<NumberType> Default for CpuReferenceRenderer<NumberType>
where
    NumberType: Copy + Default + PartialEq + Add<Output = NumberType> + AddAssign + Send + Sync + std::fmt::Debug + 'static,{
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for single-precision CPU reference renderer.
pub type CpuReferenceRendererFloat32 = CpuReferenceRenderer<f32>;

/// Type alias for double-precision CPU reference renderer.
pub type CpuReferenceRendererFloat64 = CpuReferenceRenderer<f64>;

/// Factory implementation for CPU reference renderer
#[derive(Debug)]
pub struct CpuReferenceRendererFactory {
    name: String,
}

impl CpuReferenceRendererFactory {
    pub fn new() -> Self {
        Self {
            name: "CpuReferenceRendererFactory".to_string(),
        }
    }
}

impl RendererFactory for CpuReferenceRendererFactory {
    fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError> {
        // Validate precision support
        match precision {
            DataPrecision::F32 | DataPrecision::F64 => {},
            _ => return Err(RendererError::UnsupportedPrecision(precision)),
        }

        // Parse and validate configuration
        let config = CpuReferenceConfig::from_parameters(precision, parameters)?;

        // Create renderer with appropriate precision
        let renderer: Box<dyn Renderer> = match precision {
            DataPrecision::F32 => Box::new(CpuReferenceRenderer::<f32>::with_config(config)),
            DataPrecision::F64 => Box::new(CpuReferenceRenderer::<f64>::with_config(config)),
            _ => unreachable!(), // Already validated above
        };

        Ok(renderer)
    }

    fn get_info(&self) -> RendererInfo {
        let mut parameters = HashMap::new();
        parameters.insert("threads".to_string(),
                          "Number of CPU threads to use (default: number of CPU cores)".to_string());
        parameters.insert("quality".to_string(),
                          "Rendering quality: low, medium, high, ultra (default: medium)".to_string());
        parameters.insert("debug".to_string(),
                          "Enable debug output: true/false (default: false)".to_string());

        RendererInfo::new(
            self.name.clone(),
            "software,reference,debugging,cpu".to_string(),
            parameters,
            1000, // 1000 microseconds timeout
        )
    }

    fn validate_parameters(&self, precision: DataPrecision, parameters: &str) -> Result<(), RendererError> {
        // Check precision support
        match precision {
            DataPrecision::F32 | DataPrecision::F64 => {},
            _ => return Err(RendererError::UnsupportedPrecision(precision)),
        }

        // Validate parameters by attempting to parse them
        CpuReferenceConfig::from_parameters(precision, parameters)?;
        Ok(())
    }
}

impl Default for CpuReferenceRendererFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_reference_renderer_new() {
        let renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
        assert_eq!(renderer.is_running(), false);
        assert_eq!(renderer.get_frame_count(), 0);
    }

    #[test]
    fn test_cpu_reference_renderer_f64_new() {
        let renderer: CpuReferenceRenderer<f64> = CpuReferenceRenderer::new();
        assert_eq!(renderer.is_running(), false);
        assert_eq!(renderer.get_frame_count(), 0);
    }

    #[test]
    fn test_cpu_reference_config_default() {
        let config = CpuReferenceConfig::default();
        assert_eq!(config.quality, "medium");
        assert_eq!(config.debug, false);
        assert_eq!(config.precision, DataPrecision::F32);
        assert!(config.threads > 0);
    }

    #[test]
    fn test_cpu_reference_config_parsing() {
        // Test default configuration
        let config = CpuReferenceConfig::from_parameters(DataPrecision::F32, "").unwrap();
        assert_eq!(config.precision, DataPrecision::F32);
        assert_eq!(config.quality, "medium");
        assert_eq!(config.debug, false);

        // Test custom configuration
        let config = CpuReferenceConfig::from_parameters(
            DataPrecision::F64,
            "threads=8,quality=high,debug=true"
        ).unwrap();
        assert_eq!(config.precision, DataPrecision::F64);
        assert_eq!(config.threads, 8);
        assert_eq!(config.quality, "high");
        assert_eq!(config.debug, true);

        // Test invalid parameters
        assert!(CpuReferenceConfig::from_parameters(
            DataPrecision::F32,
            "threads=0"
        ).is_err());

        assert!(CpuReferenceConfig::from_parameters(
            DataPrecision::F32,
            "quality=invalid"
        ).is_err());

        assert!(CpuReferenceConfig::from_parameters(
            DataPrecision::F32,
            "unsupported=value"
        ).is_err());
    }

    #[test]
    fn test_renderer_with_config() {
        let config = CpuReferenceConfig::from_parameters(
            DataPrecision::F32,
            "threads=4,quality=high,debug=true"
        ).unwrap();

        let renderer = CpuReferenceRenderer::<f32>::with_config(config.clone());
        assert_eq!(renderer.get_config().threads, 4);
        assert_eq!(renderer.get_config().quality, "high");
        assert_eq!(renderer.get_config().debug, true);
        assert_eq!(renderer.get_frame_count(), 0);
        assert!(!renderer.is_running());
    }

    #[test]
    fn test_renderer_lifecycle() {
        let mut renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();

        // Test initial state
        assert!(!renderer.is_running());
        assert_eq!(renderer.get_frame_count(), 0);

        // Test starting the renderer
        assert!(renderer.start().is_ok());
        assert!(renderer.is_running());

        // Test rendering frames
        assert!(renderer.render_frame().is_ok());
        assert_eq!(renderer.get_frame_count(), 1);

        assert!(renderer.render_frame().is_ok());
        assert_eq!(renderer.get_frame_count(), 2);

        // Test stopping the renderer
        renderer.stop();
        assert!(!renderer.is_running());
        assert_eq!(renderer.get_frame_count(), 2); // Frame count should persist
    }

    #[test]
    fn test_cannot_render_when_not_running() {
        let mut renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
        let result = renderer.render_frame();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not running"));
    }

    #[test]
    fn test_cannot_start_when_already_running() {
        let mut renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
        renderer.start().unwrap();

        let result = renderer.start();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already running"));
    }

    #[test]
    fn test_factory_creation() {
        let factory = CpuReferenceRendererFactory::new();
        let info = factory.get_info();

        // Test factory info
        assert_eq!(info.name, "CpuReferenceRendererFactory");
        assert_eq!(info.timeout_microseconds, 1000);
        assert!(info.has_capability("software"));
        assert!(info.has_capability("reference"));
        assert!(info.has_capability("debugging"));
        assert!(info.has_capability("cpu"));

        // Test parameter validation
        assert!(factory.validate_parameters(DataPrecision::F32, "threads=4").is_ok());
        assert!(factory.validate_parameters(DataPrecision::F64, "quality=high").is_ok());
        assert!(factory.validate_parameters(DataPrecision::F16, "").is_err());

        // Test renderer creation
        let renderer = factory.create(DataPrecision::F32, "threads=2,debug=true").unwrap();
        assert_eq!(renderer.name(), "CpuReference");
    }

    #[test]
    fn test_type_aliases() {
        let _renderer_f32: CpuReferenceRendererFloat32 = CpuReferenceRenderer::new();
        let _renderer_f64: CpuReferenceRendererFloat64 = CpuReferenceRenderer::new();
    }

    #[test]
    fn test_frame_count_operations() {
        let mut renderer: CpuReferenceRenderer<f32> = CpuReferenceRenderer::new();
        renderer.start().unwrap();

        // Render some frames
        for _ in 0..5 {
            renderer.render_frame().unwrap();
        }

        assert_eq!(renderer.get_frame_count(), 5);

        // Reset frame count
        renderer.reset_frame_count();
        assert_eq!(renderer.get_frame_count(), 0);
    }
}