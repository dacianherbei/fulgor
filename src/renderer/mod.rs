//! Renderer module of fulgor
//!
//! Provides multiple backends under a unified namespace and a manager for them.

pub mod cpu_reference;
pub mod gpu_optional;
pub mod prelude;
pub mod async_communication;
pub mod factory;
mod manager;

use std::any::TypeId;
use crate::renderer::cpu_reference::CpuReferenceRenderer;
use std::fmt;
use std::sync::{Arc, Mutex};
pub use crate::renderer::async_communication::sender::BufferedAsyncSender;
pub use factory::{RendererInfo, RendererFactory, MockRenderer, MockRendererFactory};
use crate::renderer::factory::Renderer;

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

/// Kinds of renderer backends available in fulgor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RendererKind {
    CpuReference,
    #[cfg(feature = "gpu")]
    GpuOptional,
}

impl RendererKind {
    pub fn all() -> Vec<Self> {
        let kinds = vec![RendererKind::CpuReference];
        #[cfg(feature = "gpu")]
        kinds.push(RendererKind::GpuOptional);
        kinds
    }

    pub fn create(self) -> Box<dyn Renderer + Send + Sync> {
        match self {
            // Fix: Explicitly specify f32 as the NumberType parameter
            RendererKind::CpuReference => Box::new(CpuReferenceRenderer::<f32>::new()),
            #[cfg(feature = "gpu")]
            RendererKind::GpuOptional => Box::new(GpuOptionalRenderer::new()),
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

    /// No renderer factory was found with the specified name.
    /// Contains the name that was searched for.
    RendererNotFoundByName(String),

    /// Attempted to register a factory for a renderer type that's already registered.
    /// Contains the TypeId of the conflicting renderer type.
    FactoryAlreadyRegistered(TypeId),

    /// No factories were found that support the specified capability.
    /// Contains the capability that was searched for.
    NoFactoriesWithCapability(String),

    /// No factories were found that support the specified data precision.
    /// Contains the precision that was searched for.
    NoFactoriesWithPrecision(DataPrecision),

    /// The factory registry is empty (no factories have been registered).
    EmptyFactoryRegistry,
}

impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RendererError::UnsupportedPrecision(precision) => {
                write!(f, "Unsupported data precision: {}", precision)
            },
            RendererError::InvalidParameters(msg) => {
                write!(f, "Invalid parameters: {}", msg)
            },
            RendererError::CreationFailed(msg) => {
                write!(f, "Renderer creation failed: {}", msg)
            },
            RendererError::RendererNotFound(type_id) => {
                write!(f, "Renderer not found for type: {:?}", type_id)
            },
            RendererError::RendererNotFoundByName(name) => {
                write!(f, "Renderer factory not found with name: '{}'", name)
            },
            RendererError::FactoryAlreadyRegistered(type_id) => {
                write!(f, "Factory already registered for type: {:?}", type_id)
            },
            RendererError::NoFactoriesWithCapability(capability) => {
                write!(f, "No factories found with capability: '{}'", capability)
            },
            RendererError::NoFactoriesWithPrecision(precision) => {
                write!(f, "No factories found supporting precision: {}", precision)
            },
            RendererError::EmptyFactoryRegistry => {
                write!(f, "No renderer factories have been registered")
            },
        }
    }
}

impl std::error::Error for RendererError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::error::Error;

    #[test]
    fn test_data_precision_creation() {
        let precisions = [
            DataPrecision::F16,
            DataPrecision::F32,
            DataPrecision::F64,
            DataPrecision::BFloat16,
        ];

        // Ensure all variants can be created and are distinct
        assert_eq!(precisions.len(), 4);
        for (i, &precision1) in precisions.iter().enumerate() {
            for (j, &precision2) in precisions.iter().enumerate() {
                if i == j {
                    assert_eq!(precision1, precision2);
                } else {
                    assert_ne!(precision1, precision2);
                }
            }
        }
    }

    #[test]
    fn test_data_precision_display() {
        assert_eq!(format!("{}", DataPrecision::F16), "f16");
        assert_eq!(format!("{}", DataPrecision::F32), "f32");
        assert_eq!(format!("{}", DataPrecision::F64), "f64");
        assert_eq!(format!("{}", DataPrecision::BFloat16), "bfloat16");
    }

    #[test]
    fn test_data_precision_hash_and_eq() {
        use std::collections::HashMap;

        let mut precision_map = HashMap::new();
        precision_map.insert(DataPrecision::F32, "single");
        precision_map.insert(DataPrecision::F64, "double");

        assert_eq!(precision_map.get(&DataPrecision::F32), Some(&"single"));
        assert_eq!(precision_map.get(&DataPrecision::F64), Some(&"double"));
        assert_eq!(precision_map.get(&DataPrecision::F16), None);
    }

    #[test]
    fn test_renderer_error_creation() {
        let type_id = TypeId::of::<String>();

        let errors = [
            RendererError::UnsupportedPrecision(DataPrecision::F16),
            RendererError::InvalidParameters("test error".to_string()),
            RendererError::CreationFailed("init failed".to_string()),
            RendererError::RendererNotFound(type_id),
            RendererError::FactoryAlreadyRegistered(type_id),
        ];

        // Ensure all error variants can be created
        assert_eq!(errors.len(), 5);
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

        let error5 = RendererError::FactoryAlreadyRegistered(type_id);
        assert!(format!("{}", error5).contains("Factory already registered for type:"));
    }

    #[test]
    fn test_renderer_error_as_error_trait() {
        let error = RendererError::CreationFailed("test".to_string());

        // Test that it implements the Error trait
        let error_trait: &dyn Error = &error;
        assert!(error_trait.source().is_none());

        // Test that we can get a string representation
        let error_string = format!("{}", error_trait);
        assert!(error_string.contains("Renderer creation failed: test"));
    }

    #[test]
    fn test_data_precision_clone_and_copy() {
        let original = DataPrecision::F32;
        let cloned = original.clone();
        let copied = original;

        assert_eq!(original, cloned);
        assert_eq!(original, copied);
        assert_eq!(cloned, copied);
    }

    #[test]
    fn test_renderer_error_clone() {
        let original = RendererError::InvalidParameters("test".to_string());
        let cloned = original.clone();

        match (&original, &cloned) {
            (
                RendererError::InvalidParameters(msg1),
                RendererError::InvalidParameters(msg2)
            ) => assert_eq!(msg1, msg2),
            _ => panic!("Clone did not preserve error variant"),
        }
    }

    #[test]
    fn test_data_precision_debug() {
        let precision = DataPrecision::BFloat16;
        let debug_string = format!("{:?}", precision);
        assert_eq!(debug_string, "BFloat16");
    }

    #[test]
    fn test_renderer_error_debug() {
        let error = RendererError::UnsupportedPrecision(DataPrecision::F64);
        let debug_string = format!("{:?}", error);
        assert!(debug_string.contains("UnsupportedPrecision"));
        assert!(debug_string.contains("F64"));
    }
}