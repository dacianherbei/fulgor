//! Renderer module of fulgor
//!
//! Provides multiple backends under a unified namespace and a manager for them.

pub mod cpu_reference;
pub mod gpu_optional;
pub mod prelude;
pub mod async_communication;

use crate::renderer::cpu_reference::CpuReferenceRenderer;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::atomic::AtomicU64;
pub use crate::renderer::async_communication::sender::BufferedAsyncSender;

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