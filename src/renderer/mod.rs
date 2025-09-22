//! Renderer module of fulgor
//!
//! Provides multiple backends under a unified namespace and a manager for them.

pub mod cpu_reference;
pub mod gpu_optional;
pub mod prelude;
pub mod async_communication;
pub mod factory;

use std::any::TypeId;
use crate::renderer::cpu_reference::CpuReferenceRenderer;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::AtomicU64;
pub use crate::renderer::async_communication::sender::BufferedAsyncSender;
pub use factory::{RendererInfo, RendererFactory, MockRenderer, MockRendererFactory};

/// Core trait for all rendering implementations in the fulgor library.
///
/// This trait provides a common interface for different rendering backends
/// such as software renderers, GPU-based renderers, or specialized
/// Gaussian splatting implementations.
///
/// The trait requires `Send + Sync` to ensure thread safety across
/// different rendering contexts and multi-threaded applications.
pub trait Renderer: Send + Sync {
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self);
    fn render_frame(&mut self) -> Result<(), String>;
    fn name(&self) -> &'static str;
}

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
        let mut kinds = vec![RendererKind::CpuReference];
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

/// Internal state of the manager.
struct RendererManagerInner {
    renderers: HashMap<RendererKind, Box<dyn Renderer + Send + Sync>>,
    active: Option<RendererKind>,
    sender: Option<mpsc::Sender<RendererEvent>>,
    async_sinks: Vec<Arc<Mutex<Vec<RendererEvent>>>>,
    // New field for async buffered sender
    buffered_async_sender: Option<BufferedAsyncSender<RendererEvent>>,
}

/// Thread-safe manager handle.
#[derive(Clone)]
pub struct RendererManager {
    inner: Arc<Mutex<RendererManagerInner>>,
}

impl RendererManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RendererManagerInner {
                renderers: HashMap::new(),
                active: None,
                sender: None,
                async_sinks: Vec::new(),
                buffered_async_sender: None,
            })),
        }
    }

    /// Subscribe synchronously (std channel).
    pub fn subscribe(&self) -> mpsc::Receiver<RendererEvent> {
        let (tx, rx) = mpsc::channel();
        let mut inner = self.inner.lock().unwrap();
        inner.sender = Some(tx);
        rx
    }

    /// Subscribe asynchronously (returns a Stream).
    pub fn async_subscribe(&self) -> RendererEventStream {
        let sink = Arc::new(Mutex::new(Vec::new()));
        {
            let mut inner = self.inner.lock().unwrap();
            inner.async_sinks.push(sink.clone());
        }
        RendererEventStream { buffer: sink }
    }

    /// Subscribe using BufferedAsyncSender with bounded channel.
    pub fn subscribe_buffered_bounded(
        &self,
        capacity: usize,
        drop_oldest_on_full: bool
    ) -> tokio::sync::mpsc::Receiver<RendererEvent> {
        let (buffered_sender, receiver) = BufferedAsyncSender::<RendererEvent>::new_bounded(capacity,drop_oldest_on_full,Arc::new(AtomicU64::new(0)));
        let mut inner = self.inner.lock().unwrap();
        inner.buffered_async_sender = Some(buffered_sender);
        receiver
    }

    /// Subscribe using BufferedAsyncSender with unbounded channel.
    pub fn subscribe_buffered_unbounded(&self) -> tokio::sync::mpsc::UnboundedReceiver<RendererEvent> {
        let (buffered_sender, receiver) = BufferedAsyncSender::<RendererEvent>::new_unbounded(Option::<usize>::Some(1));
        let mut inner = self.inner.lock().unwrap();
        inner.buffered_async_sender = Some(buffered_sender);
        receiver
    }

    /// Get the current BufferedAsyncSender if available.
    pub fn get_buffered_sender(&self) -> Option<BufferedAsyncSender<RendererEvent>> {
        let inner = self.inner.lock().unwrap();
        inner.buffered_async_sender.clone()
    }

    /// Notify all subscribers including the buffered async sender.
    async fn notify_async(&self, event: RendererEvent) {
        let inner = self.inner.lock().unwrap();

        // Sync
        if let Some(sender) = &inner.sender {
            let _ = sender.send(event.clone());
        }

        // Async sinks
        for sink in &inner.async_sinks {
            sink.lock().unwrap().push(event.clone());
        }

        // Buffered async sender
        if let Some(buffered_sender) = &inner.buffered_async_sender {
            let sender = buffered_sender.clone();
            drop(inner); // Release lock before awaiting
            let _ = sender.send_event(event).await;
        }
    }

    fn notify(&self, event: RendererEvent) {
        let inner = self.inner.lock().unwrap();
        // Sync
        if let Some(sender) = &inner.sender {
            let _ = sender.send(event.clone());
        }
        // Async sinks
        for sink in &inner.async_sinks {
            sink.lock().unwrap().push(event.clone());
        }
    }

    pub fn add(&self, kind: RendererKind) {
        let mut inner = self.inner.lock().unwrap();
        inner.renderers.entry(kind).or_insert_with(|| kind.create());
    }

    pub async fn start_async(&self, kind: RendererKind) -> Result<(), String> {
        let result = {
            let mut inner = self.inner.lock().unwrap();
            inner.renderers.entry(kind).or_insert_with(|| kind.create());
            if let Some(renderer) = inner.renderers.get_mut(&kind) {
                renderer.start()?;
                inner.active = Some(kind);
                Ok(())
            } else {
                Err(format!("Renderer {:?} not found", kind))
            }
        };

        if result.is_ok() {
            self.notify_async(RendererEvent::Started(kind)).await;
            self.notify_async(RendererEvent::Switched(Some(kind))).await;
        }

        result
    }

    pub fn start(&self, kind: RendererKind) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        inner.renderers.entry(kind).or_insert_with(|| kind.create());
        if let Some(renderer) = inner.renderers.get_mut(&kind) {
            renderer.start()?;
            inner.active = Some(kind);
            drop(inner);
            self.notify(RendererEvent::Started(kind));
            self.notify(RendererEvent::Switched(Some(kind)));
            Ok(())
        } else {
            Err(format!("Renderer {:?} not found", kind))
        }
    }

    pub async fn stop_async(&self, kind: RendererKind) {
        let (was_active, stopped) = {
            let mut inner = self.inner.lock().unwrap();
            let stopped = if let Some(renderer) = inner.renderers.get_mut(&kind) {
                renderer.stop();
                true
            } else {
                false
            };
            let was_active = inner.active == Some(kind);
            if was_active {
                inner.active = None;
            }
            (was_active, stopped)
        };

        if stopped {
            self.notify_async(RendererEvent::Stopped(kind)).await;
            if was_active {
                self.notify_async(RendererEvent::Switched(None)).await;
            }
        }
    }

    pub fn stop(&self, kind: RendererKind) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(renderer) = inner.renderers.get_mut(&kind) {
            renderer.stop();
            let was_active = inner.active == Some(kind);
            if was_active {
                inner.active = None;
            }
            drop(inner);
            self.notify(RendererEvent::Stopped(kind));
            if was_active {
                self.notify(RendererEvent::Switched(None));
            }
        }
    }

    pub fn render_frame(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        match inner.active {
            Some(kind) => {
                if let Some(renderer) = inner.renderers.get_mut(&kind) {
                    renderer.render_frame()
                } else {
                    Err("Active renderer not found".into())
                }
            }
            None => Err("No active renderer".into()),
        }
    }

    pub async fn switch_async(&self, kind: RendererKind) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(active) = inner.active {
                if active == kind {
                    return Ok(()); // already active
                }
                if let Some(renderer) = inner.renderers.get_mut(&active) {
                    renderer.stop();
                }
                inner.active = None;
            }
        }

        self.notify_async(RendererEvent::Stopped(kind)).await;
        self.notify_async(RendererEvent::Switched(None)).await;
        self.start_async(kind).await
    }

    pub fn switch(&self, kind: RendererKind) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().unwrap();
            if let Some(active) = inner.active {
                if active == kind {
                    return Ok(()); // already active
                }
                if let Some(renderer) = inner.renderers.get_mut(&active) {
                    renderer.stop();
                }
                inner.active = None;
                drop(inner);
                self.notify(RendererEvent::Stopped(kind));
                self.notify(RendererEvent::Switched(None));
            }
        }
        self.start(kind)
    }

    pub async fn stop_all_async(&self) {
        let renderer_kinds: Vec<RendererKind> = {
            let mut inner = self.inner.lock().unwrap();
            let kinds: Vec<_> = inner.renderers.keys().cloned().collect();
            for (_, renderer) in inner.renderers.iter_mut() {
                renderer.stop();
            }
            inner.active = None;
            kinds
        };

        for kind in renderer_kinds {
            self.notify_async(RendererEvent::Stopped(kind)).await;
        }
        self.notify_async(RendererEvent::Switched(None)).await;
    }

    pub fn stop_all(&self) {
        let mut inner = self.inner.lock().unwrap();
        for (kind, renderer) in inner.renderers.iter_mut() {
            renderer.stop();
            self.notify(RendererEvent::Stopped(*kind));
        }
        inner.active = None;
        drop(inner);
        self.notify(RendererEvent::Switched(None));
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    /// Attempted to register a factory for a renderer type that's already registered.
    /// Contains the TypeId of the conflicting renderer type.
    FactoryAlreadyRegistered(TypeId),
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
            RendererError::FactoryAlreadyRegistered(type_id) => {
                write!(f, "Factory already registered for type: {:?}", type_id)
            },
        }
    }
}

impl Error for RendererError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // None of our error variants wrap other errors
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;

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