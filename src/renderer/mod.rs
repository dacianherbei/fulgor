//! Renderer module of fulgor
//!
//! Provides multiple backends under a unified namespace and a manager for them.

pub mod prelude;
pub mod async_communication;
pub mod factory;
pub mod capabilities;
pub mod custom;
pub mod world;
mod manager;

use std::any::TypeId;
use std::fmt;
use std::sync::{Arc, Mutex};
pub use crate::renderer::async_communication::sender::BufferedAsyncSender;
pub use factory::{RendererInfo, RendererFactory, MockRenderer, MockRendererFactory};
use std::fmt::Debug;

/// Events emitted by renderer components in the fulgor engine.
///
/// These events represent significant state changes in the rendering system
/// and can be used for monitoring, debugging, and coordinating between
/// different parts of the application.
#[derive(Debug, Clone)]
pub enum RendererEvent {
    /// A renderer backend has been started.
    ///
    /// Emitted when a specific renderer backend successfully initializes
    /// and begins its rendering loop.
    Started(RendererKind),

    /// A renderer backend has been stopped.
    ///
    /// Emitted when a specific renderer backend has been shut down,
    /// either gracefully or due to an error condition.
    Stopped(RendererKind),

    ShutdownRequested,

    /// The active renderer has been switched.
    ///
    /// Emitted when the rendering system switches from one backend to another.
    /// The `Option<RendererKind>` represents the new active renderer:
    /// - `Some(RendererKind)` indicates a switch to the specified backend
    /// - `None` indicates no renderer is currently active
    Switched(Option<RendererKind>),

    /// The data precision for computations has been changed.
    ///
    /// Emitted when a renderer changes its internal data precision,
    /// which affects memory usage and computational accuracy.
    DataPrecisionChanged {
        old_precision: DataPrecision,
        new_precision: DataPrecision
    },

    ViewportResized { width: u32, height: u32 },
    SplatDataUpdated { splat_count: usize },
    FrameRendered {
        renderer_kind: RendererKind,
        frame_number: u64,
        frame_time_microseconds: u64,
        render_time_ns: u64
    },
    Error {
        renderer_kind: Option<RendererKind>,
        message: String,
    },
}

/// Base trait for all capabilities in the fulgor rendering system.
///
/// This trait provides a foundation for the capability system, allowing
/// different components to expose their features and characteristics
/// in a consistent manner.
pub trait Capability {
    /// Get the unique identifier for this capability.
    ///
    /// This should be a human-readable string that clearly identifies
    /// the capability, such as "rendering", "gpu_compute", or "precision_f64".
    fn capability_name(&self) -> &'static str;

    /// Provides an optional description of what this capability does.
    ///
    /// Returns `None` by default, but implementations can override this
    /// to provide detailed information about the capability's purpose
    /// and functionality.
    fn description(&self) -> Option<&'static str> {
        None
    }
}

/// Trait for components that perform processing operations with configurable precision.
///
/// This trait extends the base `Capability` trait to provide precision-specific
/// functionality for processing units like renderers, compute kernels, or
/// mathematical operations that can work with different data precisions.
pub trait ProcessingUnitCapability: Capability {
    /// Check if this processing unit supports a specific data precision.
    ///
    /// # Arguments
    /// * `precision` - The data precision to check support for
    ///
    /// # Returns
    /// `true` if the precision is supported, `false` otherwise
    fn supports_precision(&self, precision: DataPrecision) -> bool;

    /// Get a list of all data precisions supported by this processing unit.
    ///
    /// # Returns
    /// A vector containing all supported data precisions, typically ordered
    /// from lowest to highest precision or by preference.
    fn supported_precisions(&self) -> Vec<DataPrecision>;

    /// Get the preferred data precision for this processing unit.
    ///
    /// This represents the precision that provides the best balance of
    /// performance and accuracy for this particular processing unit.
    ///
    /// # Returns
    /// `Some(DataPrecision)` if there's a preferred precision,
    /// `None` if no preference is specified
    fn preferred_precision(&self) -> Option<DataPrecision>;
}

/// Enhanced renderer trait that supports precision management.
///
/// This trait extends both the factory `Renderer` trait and `ProcessingUnitCapability`
/// to provide renderer-specific functionality including precision switching and
/// configuration management while maintaining compatibility with the existing factory system.
pub trait Renderer {
    /// Set the data precision for this renderer.
    ///
    /// Attempts to change the internal data precision used for computations.
    /// The returned precision may differ from the requested precision if
    /// the exact precision is not supported.
    ///
    /// # Arguments
    /// * `precision` - The desired data precision
    ///
    /// # Returns
    /// * `Ok(DataPrecision)` - The actual precision that was set
    /// * `Err(String)` - Error message if the precision change failed
    fn set_data_precision(&mut self, precision: DataPrecision) -> Result<DataPrecision, String>;

    /// Get the current data precision for this renderer.
    ///
    /// # Returns
    /// The currently active data precision
    fn get_data_precision(&self) -> DataPrecision;

    /// Check if the renderer is currently running.
    fn is_running(&self) -> bool;

    /// Get the total number of frames rendered.
    fn get_frame_count(&self) -> u64;

    /// Start the renderer
    fn start(&mut self) -> Result<(), String>;

    /// Stop the renderer
    fn stop(&mut self);

    /// Get the renderer's name
    fn name(&self) -> &'static str;

    fn render_frame(&mut self) -> Result<(), String>;
}

/// A reference implementation of a renderer that provides basic functionality.
///
/// This renderer serves as a baseline implementation that can be used for
/// testing, validation, and as a fallback when specialized renderers are
/// not available. It supports all standard data precisions and provides
/// CPU/GPU unified rendering capabilities.
#[derive(Debug)]
pub struct ReferenceRenderer {
    /// Current data precision for computations
    precision: DataPrecision,

    /// Whether the renderer is currently running
    is_running: bool,

    /// Total number of frames rendered
    frame_count: u64,
}

impl ReferenceRenderer {
    /// Create a new reference renderer with default settings.
    ///
    /// The renderer starts with F32 precision and in a stopped state.
    pub fn new() -> Self {
        Self {
            precision: DataPrecision::F32,
            is_running: false,
            frame_count: 0,
        }
    }

    /// Create a new reference renderer with specified precision.
    pub fn with_precision(precision: DataPrecision) -> Self {
        Self {
            precision,
            is_running: false,
            frame_count: 0,
        }
    }
}

impl Capability for ReferenceRenderer {
    fn capability_name(&self) -> &'static str {
        "reference_renderer"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Basic reference renderer implementation supporting all standard data precisions")
    }
}

impl ProcessingUnitCapability for ReferenceRenderer {
    fn supports_precision(&self, precision: DataPrecision) -> bool {
        match precision {
            DataPrecision::F16 | DataPrecision::F32 | DataPrecision::F64 | DataPrecision::BFloat16 => true,
        }
    }

    fn supported_precisions(&self) -> Vec<DataPrecision> {
        vec![DataPrecision::F16, DataPrecision::BFloat16, DataPrecision::F32, DataPrecision::F64]
    }

    fn preferred_precision(&self) -> Option<DataPrecision> {
        Some(DataPrecision::F32)
    }
}

impl factory::Renderer for ReferenceRenderer {
    fn start(&mut self) -> Result<(), String> {
        if self.is_running {
            Err("Renderer is already running".to_string())
        } else {
            self.is_running = true;
            Ok(())
        }
    }

    fn stop(&mut self) {
        self.is_running = false;
    }

    fn name(&self) -> &'static str {
        "ReferenceRenderer"
    }

    fn render_frame(&mut self) -> Result<(), String> {
        if !self.is_running {
            return Err("Renderer is not running".to_string());
        }

        self.frame_count += 1;
        // In a real implementation, this would perform actual rendering
        Ok(())
    }

    fn set_data_precision(&mut self, precision: DataPrecision) -> Result<DataPrecision, String> {
        if !self.supports_precision(precision) {
            return Err(format!("Unsupported precision: {}", precision));
        }

        let old_precision = self.precision;
        self.precision = precision;

        // In a real implementation, this would trigger a DataPrecisionChanged event
        // through some event system

        Ok(precision)
    }

    fn get_data_precision(&self) -> DataPrecision {
        self.precision
    }

    fn is_running(&self) -> bool {
        self.is_running
    }

    fn get_frame_count(&self) -> u64 {
        self.frame_count
    }
}

/// Kinds of renderer backends available in fulgor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RendererKind {
    CpuReference,
    #[cfg(feature = "gpu")]
    GpuOptional,
    Reference,
}

impl RendererKind {
    pub fn all() -> Vec<Self> {
        let kinds = vec![RendererKind::CpuReference, RendererKind::Reference];
        #[cfg(feature = "gpu")]
        kinds.push(RendererKind::GpuOptional);
        kinds
    }

    pub fn create(self) -> Box<dyn Renderer + Send + Sync> {
        match self {
            // Updated to use non-generic renderers
            RendererKind::CpuReference => {
                // Create a CPU reference renderer with default F32 precision
                // This will need to be updated in the cpu_reference module
                // to remove NumberType generics
                Box::new(ReferenceRenderer::with_precision(DataPrecision::F32))
            },
            RendererKind::Reference => Box::new(ReferenceRenderer::new()),
            #[cfg(feature = "gpu")]
            RendererKind::GpuOptional => {
                // This will need to be updated in the gpu_optional module
                // to remove NumberType generics and implement the new traits
                Box::new(ReferenceRenderer::with_precision(DataPrecision::F32))
            },
        }
    }
}

/// Async stream of renderer events.
pub struct RendererEventStream {
    buffer: Arc<Mutex<Vec<RendererEvent>>>,
}

impl futures::Stream for RendererEventStream {
    type Item = RendererEvent;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut buf = self.buffer.lock().unwrap();
        if !buf.is_empty() {
            std::task::Poll::Ready(Some(buf.remove(0)))
        } else {
            std::task::Poll::Pending
        }
    }
}

/// Precision types supported by the renderer factory system.
///
/// Defines the floating-point precision for rendering operations,
/// allowing for performance vs quality trade-offs across different
/// hardware capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum DataPrecision {
    /// Half precision floating point (16-bit)
    F16,
    /// Single precision floating point (32-bit)
    F32,
    /// Double precision floating point (64-bit)
    F64,
    /// Brain floating point format (16-bit, Google's bfloat16)
    BFloat16,
}

impl fmt::Display for DataPrecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataPrecision::F16 => write!(f, "f16"),
            DataPrecision::F32 => write!(f, "f32"),
            DataPrecision::F64 => write!(f, "f64"),
            DataPrecision::BFloat16 => write!(f, "bfloat16"),
        }
    }
}

/// Errors that can occur during renderer factory operations.
///
/// This enum captures all possible failure modes when creating,
/// registering, or managing renderers through the factory system.
#[derive(Debug, Clone)]
pub enum RendererError {
    /// The specified data precision is not supported by this renderer.
    UnsupportedPrecision(DataPrecision),

    /// Invalid parameters were provided during renderer creation.
    /// Contains a descriptive message about what was invalid.
    InvalidParameters(String),

    /// Renderer creation failed for an implementation-specific reason.
    /// Contains a descriptive message about the failure.
    CreationFailed(String),

    /// No renderer was found for the specified type identifier.
    /// This typically occurs when trying to retrieve an unregistered renderer.
    RendererNotFound(TypeId),

    /// Factory already registered for the specified type.
    /// Contains the type identifier that was already registered.
    FactoryAlreadyRegistered(TypeId),
}

impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RendererError::UnsupportedPrecision(precision) => {
                write!(f, "Unsupported data precision: {}", precision)
            }
            RendererError::InvalidParameters(msg) => {
                write!(f, "Invalid parameters: {}", msg)
            }
            RendererError::CreationFailed(msg) => {
                write!(f, "Renderer creation failed: {}", msg)
            }
            RendererError::RendererNotFound(type_id) => {
                write!(f, "Renderer not found for type: {:?}", type_id)
            }
            RendererError::FactoryAlreadyRegistered(type_id) => {
                write!(f, "Factory already registered for type: {:?}", type_id)
            }
        }
    }
}

impl std::error::Error for RendererError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_precision_display() {
        assert_eq!(format!("{}", DataPrecision::F16), "f16");
        assert_eq!(format!("{}", DataPrecision::F32), "f32");
        assert_eq!(format!("{}", DataPrecision::F64), "f64");
        assert_eq!(format!("{}", DataPrecision::BFloat16), "bfloat16");
    }

    #[test]
    fn test_reference_renderer_creation() {
        let renderer = ReferenceRenderer::new();
        assert_eq!(renderer.get_data_precision(), DataPrecision::F32);
        assert!(!renderer.is_running());
        assert_eq!(renderer.get_frame_count(), 0);
        assert_eq!(renderer.name(), "ReferenceRenderer");
    }

    #[test]
    fn test_reference_renderer_lifecycle() {
        let mut renderer = ReferenceRenderer::new();
        assert!(!renderer.is_running());

        // Test starting the renderer
        let result = renderer.start();
        assert!(result.is_ok());
        assert!(renderer.is_running());

        // Test starting already running renderer
        let result = renderer.start();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already running"));

        // Test rendering a frame
        let result = renderer.render_frame();
        assert!(result.is_ok());
        assert_eq!(renderer.get_frame_count(), 1);

        // Test stopping the renderer
        renderer.stop();
        assert!(!renderer.is_running());

        // Test rendering when stopped
        let result = renderer.render_frame();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not running"));
    }

    #[test]
    fn test_reference_renderer_precision_support() {
        let renderer = ReferenceRenderer::new();

        // Test precision support
        assert!(renderer.supports_precision(DataPrecision::F16));
        assert!(renderer.supports_precision(DataPrecision::F32));
        assert!(renderer.supports_precision(DataPrecision::F64));
        assert!(renderer.supports_precision(DataPrecision::BFloat16));

        // Test supported precisions
        let supported = renderer.supported_precisions();
        assert_eq!(supported.len(), 4);
        assert!(supported.contains(&DataPrecision::F16));
        assert!(supported.contains(&DataPrecision::F32));
        assert!(supported.contains(&DataPrecision::F64));
        assert!(supported.contains(&DataPrecision::BFloat16));

        // Test preferred precision
        assert_eq!(renderer.preferred_precision(), Some(DataPrecision::F32));
    }

    #[test]
    fn test_reference_renderer_precision_change() {
        let mut renderer = ReferenceRenderer::new();
        assert_eq!(renderer.get_data_precision(), DataPrecision::F32);

        // Test successful precision change
        let result = renderer.set_data_precision(DataPrecision::F64);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), DataPrecision::F64);
        assert_eq!(renderer.get_data_precision(), DataPrecision::F64);

        // Test setting same precision again
        let result = renderer.set_data_precision(DataPrecision::F64);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), DataPrecision::F64);
    }

    #[test]
    fn test_capability_trait() {
        let renderer = ReferenceRenderer::new();
        assert_eq!(renderer.capability_name(), "reference_renderer");
        assert!(renderer.description().is_some());
        assert!(renderer.description().unwrap().contains("Basic reference renderer"));
    }

    #[test]
    fn test_renderer_kind_creation() {
        // Test that renderers can be created and implement both factory and enhanced traits
        let mut cpu_renderer = RendererKind::CpuReference.create();
        assert_eq!(cpu_renderer.name(), "ReferenceRenderer"); // Placeholder until cpu_reference is updated
        assert!(!cpu_renderer.is_running());

        // Test factory trait methods
        let result = cpu_renderer.start();
        assert!(result.is_ok());
        assert!(cpu_renderer.is_running());

        cpu_renderer.stop();
        assert!(!cpu_renderer.is_running());

        let mut ref_renderer = RendererKind::Reference.create();
        assert_eq!(ref_renderer.name(), "ReferenceRenderer");
        assert_eq!(ref_renderer.get_data_precision(), DataPrecision::F32);

        #[cfg(feature = "gpu")]
        {
            let mut gpu_renderer = RendererKind::GpuOptional.create();
            assert_eq!(gpu_renderer.name(), "ReferenceRenderer"); // Placeholder until gpu_optional is updated
            assert!(!gpu_renderer.is_running());
        }
    }

    #[test]
    fn test_data_precision_changed_event() {
        let event = RendererEvent::DataPrecisionChanged {
            old_precision: DataPrecision::F32,
            new_precision: DataPrecision::F64,
        };

        // Test that the event can be created and cloned
        let cloned_event = event.clone();
        match cloned_event {
            RendererEvent::DataPrecisionChanged { old_precision, new_precision } => {
                assert_eq!(old_precision, DataPrecision::F32);
                assert_eq!(new_precision, DataPrecision::F64);
            }
            _ => panic!("Event type mismatch"),
        }
    }
}