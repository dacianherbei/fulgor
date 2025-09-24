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
use std::fmt::{self, Debug};
use std::sync::{Arc, Mutex};
pub use crate::renderer::async_communication::sender::BufferedAsyncSender;
pub use factory::{RendererInfo, RendererFactory, MockRenderer, MockRendererFactory};
use std::any::Any;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
pub use crate::renderer::manager::RendererId;

/// Events emitted by renderer components in the fulgor engine.
///
/// These events represent significant state changes in the rendering system
/// and can be used for monitoring, debugging, and coordinating between
/// different parts of the application.
#[derive(Debug, Clone)]
pub enum RendererEvent {
    /// A renderer has been created.
    RendererCreated {
        renderer_name: String,
        precision: DataPrecision
    },

    /// A renderer has been destroyed.
    Destroyed (RendererId),

    /// A renderer has been started.
    Started (RendererId),

    /// A renderer has been stopped.
    Stopped (RendererId),

    /// The data precision for computations has been changed.
    DataPrecisionChanged {
        id:RendererId,
        old_precision: DataPrecision,
        new_precision: DataPrecision
    },

    /// The active renderer has been switched.
    ///
    /// Emitted when the rendering system switches from one backend to another.
    /// The `Option<RendererKind>` represents the new active renderer:
    /// - `Some(RendererKind)` indicates a switch to the specified backend
    /// - `None` indicates no renderer is currently active
    Switched(Option<RendererId>),
    /// Viewport has been resized.
    ViewportResized {
        id:RendererId,
        width: u32,
        height: u32
    },

    /// Splat data has been updated.
    SplatDataUpdated {
        id:RendererId,
        splat_count: usize
    },

    /// A frame has been rendered.
    FrameRendered {
        id:RendererId,
        frame_number: u64,
        frame_time_microseconds: u64,
        render_time_ns: u64
    },

    /// An error occurred.
    RendererError {
        id:RendererId,
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
/// This trait provides renderer-specific functionality including precision switching and
/// configuration management while maintaining compatibility with the existing factory system.
///
/// ## Trait Composition
///
/// This trait includes all necessary bounds for renderer implementations:
/// - `Send + Sync` for thread safety
/// - `Debug` for debugging and logging
/// - Methods from both factory operations and enhanced precision management
///
/// This design allows the enhanced renderer to work seamlessly with the existing
/// factory system while providing advanced precision and capability management.
#[async_trait::async_trait]
pub trait Renderer: Send + Sync + Debug {
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

    fn sender(&self) -> async_communication::BufferedAsyncSender<RendererEvent>;
    async fn run(self); // consumes and runs until `Shutdown`
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

    sender: BufferedAsyncSender<RendererEvent>,
    receiver: UnboundedReceiver<RendererEvent>
}

impl ReferenceRenderer {
    /// Create a new reference renderer with default settings.
    ///
    /// The renderer starts with F32 precision and in a stopped state.
    pub fn new() -> Self {
        let (buffered_sender, buffered_receiver) = BufferedAsyncSender::<RendererEvent>::new_unbounded(Option::<usize>::Some(100));
        Self {
            precision: DataPrecision::F32,
            is_running: false,
            frame_count: 0,
            sender: buffered_sender,
            receiver: buffered_receiver
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

    pub fn sender(&self) -> BufferedAsyncSender<RendererEvent> {
        self.sender.clone()
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

impl Renderer for ReferenceRenderer {
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

    pub async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            match event {
                RendererEvent::Destroyed(id) => {
                    self.stop();
                    println!("ReferenceRenderer destroyed {:?}", id);
                    break;
                }
                RendererEvent::Started(id) => {
                    let _ = self.start();
                    println!("ReferenceRenderer started {:?}", id);
                }
                RendererEvent::Stopped(id) => {
                    self.stop();
                    println!("ReferenceRenderer stopped {:?}", id);
                }
                RendererEvent::Switched(active) => {
                    println!("ReferenceRenderer switched, active = {:?}", active);
                }
            }
        }
    }
}

/// Factory for creating ReferenceRenderer instances.
#[derive(Debug)]
pub struct ReferenceRendererFactory {
    factory_name: String,
}

impl ReferenceRendererFactory {
    /// Create a new ReferenceRenderer factory.
    pub fn new() -> Self {
        Self {
            factory_name: "ReferenceRenderer".to_string(),
        }
    }
}

impl RendererFactory for ReferenceRendererFactory {
    fn create(&self, precision: DataPrecision, parameters: &str) -> Result<Box<dyn Renderer>, RendererError> {
        // Parse parameters if any (reference renderer accepts minimal parameters)
        if !parameters.is_empty() {
            let params = factory::parse_parameters(parameters);
            for (key, _) in params {
                match key.as_str() {
                    "precision" => {}, // Handled by precision parameter
                    _ => {
                        return Err(RendererError::InvalidParameters(
                            format!("Unknown parameter for ReferenceRenderer: {}", key)
                        ));
                    }
                }
            }
        }

        Ok(Box::new(ReferenceRenderer::with_precision(precision)))
    }

    fn get_info(&self) -> RendererInfo {
        let mut parameters = std::collections::HashMap::new();
        parameters.insert(
            "precision".to_string(),
            "Data precision for rendering (f16, f32, f64, bfloat16)".to_string()
        );

        RendererInfo::new(
            self.factory_name.clone(),
            "reference,cpu,basic_rendering,all_precisions".to_string(),
            parameters,
            1000, // 1ms timeout
        )
    }

    fn validate_parameters(&self, _precision: DataPrecision, parameters: &str) -> Result<(), RendererError> {
        if parameters.is_empty() {
            return Ok(());
        }

        let params = factory::parse_parameters(parameters);
        for (key, _) in params {
            match key.as_str() {
                "precision" => {},
                _ => {
                    return Err(RendererError::InvalidParameters(
                        format!("Unknown parameter for ReferenceRenderer: {}", key)
                    ));
                }
            }
        }

        Ok(())
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

    /// No renderer factory was found with the specified name.
    /// Contains the name that was searched for.
    RendererNotFoundByName(String),

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
            RendererError::RendererNotFoundByName(name) => {
                write!(f, "Renderer factory not found with name: {}", name)
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
    fn test_reference_renderer_factory() {
        let factory = ReferenceRendererFactory::new();
        let info = factory.get_info();

        assert_eq!(info.name, "ReferenceRenderer");
        assert!(info.has_capability("reference"));
        assert!(info.has_capability("cpu"));
        assert!(info.has_capability("basic_rendering"));
        assert!(info.has_capability("all_precisions"));

        // Test renderer creation
        let renderer = factory.create(DataPrecision::F64, "").unwrap();
        assert_eq!(renderer.get_data_precision(), DataPrecision::F64);
        assert_eq!(renderer.name(), "ReferenceRenderer");

        // Test parameter validation
        assert!(factory.validate_parameters(DataPrecision::F32, "").is_ok());
        assert!(factory.validate_parameters(DataPrecision::F32, "precision=f32").is_ok());
        assert!(factory.validate_parameters(DataPrecision::F32, "invalid_param=value").is_err());
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
        // Test that renderers can be created and implement the enhanced Renderer trait
        let mut cpu_renderer = ReferenceRenderer::new();
        assert_eq!(cpu_renderer.name(), "ReferenceRenderer"); // Placeholder until cpu_reference is updated

        // Test factory trait methods work through the trait object
        let result = cpu_renderer.start();
        assert!(result.is_ok());

        cpu_renderer.stop();

        let mut ref_renderer = ReferenceRenderer::new();
        assert_eq!(ref_renderer.name(), "ReferenceRenderer");

        #[cfg(feature = "gpu")]
        {
            let mut gpu_renderer = RendererKind::GpuOptional.create();
            assert_eq!(gpu_renderer.name(), "ReferenceRenderer"); // Placeholder until gpu_optional is updated
        }
    }

    #[test]
    fn test_enhanced_renderer_trait_composition() {
        // Test that our enhanced Renderer trait properly includes all necessary methods
        let mut renderer = ReferenceRenderer::new();

        // Test enhanced trait methods
        assert_eq!(renderer.get_data_precision(), DataPrecision::F32);
        assert!(!renderer.is_running());
        assert_eq!(renderer.get_frame_count(), 0);

        // Test factory trait methods
        let result = renderer.start();
        assert!(result.is_ok());
        assert!(renderer.is_running());

        let result = renderer.render_frame();
        assert!(result.is_ok());
        assert_eq!(renderer.get_frame_count(), 1);

        renderer.stop();
        assert!(!renderer.is_running());

        // Test precision change
        let result = renderer.set_data_precision(DataPrecision::F64);
        assert!(result.is_ok());
        assert_eq!(renderer.get_data_precision(), DataPrecision::F64);
    }

    #[test]
    fn test_trait_object_debug() {
        // Test that trait objects properly implement Debug
        let renderer = ReferenceRenderer::new();
        let debug_string = format!("{:?}", renderer);
        assert!(debug_string.contains("ReferenceRenderer"));
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

    #[test]
    fn test_renderer_error_creation() {
        let type_id = TypeId::of::<String>();

        let errors = [
            RendererError::UnsupportedPrecision(DataPrecision::F16),
            RendererError::InvalidParameters("test error".to_string()),
            RendererError::CreationFailed("init failed".to_string()),
            RendererError::RendererNotFound(type_id),
            RendererError::RendererNotFoundByName("TestRenderer".to_string()),
            RendererError::FactoryAlreadyRegistered(type_id),
        ];

        // Ensure all error variants can be created
        assert_eq!(errors.len(), 6);
    }

    #[test]
    fn test_renderer_error_display() {
        let type_id = TypeId::of::<String>();

        let error1 = RendererError::UnsupportedPrecision(DataPrecision::F32);
        assert!(format!("{}", error1).contains("Unsupported data precision: f32"));

        let error2 = RendererError::InvalidParameters("missing config".to_string());
        assert!(format!("{}", error2).contains("Invalid parameters: missing config"));

        let error3 = RendererError::CreationFailed("GPU not available".to_string());
        assert!(format!("{}", error3).contains("Renderer creation failed: GPU not available"));

        let error4 = RendererError::RendererNotFound(type_id);
        assert!(format!("{}", error4).contains("Renderer not found for type:"));

        let error5 = RendererError::RendererNotFoundByName("TestRenderer".to_string());
        assert!(format!("{}", error5).contains("Renderer factory not found with name: TestRenderer"));

        let error6 = RendererError::FactoryAlreadyRegistered(type_id);
        assert!(format!("{}", error6).contains("Factory already registered for type:"));
    }
}