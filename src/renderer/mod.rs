//! Renderer module of fulgor
//!
//! Provides multiple backends under a unified namespace and a manager for them.

pub mod cpu_reference;
pub mod gpu_optional;
pub mod prelude;
pub mod async_communication;

use crate::renderer::cpu_reference::CpuReferenceRenderer;
use crate::renderer::gpu_optional::GpuOptionalRenderer;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};

/// Core trait for all rendering implementations in the fulgor library.
///
/// This trait provides a common interface for different rendering backends
/// such as software renderers, GPU-based renderers, or specialized
/// Gaussian splatting implementations.
///
/// The trait requires `Send + Sync` to ensure thread safety across
/// different rendering contexts and multi-threaded applications.
pub trait Renderer: Send + Sync {
    /// Initialize the renderer and prepare it for rendering operations.
    ///
    /// This method should set up any necessary resources, contexts,
    /// or state required for the rendering pipeline.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful initialization, or `Err(String)`
    /// with an error description if initialization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut renderer = SomeRenderer::new();
    /// renderer.start()?;
    /// ```
    fn start(&mut self) -> Result<(), String>;

    /// Stop the renderer and clean up any allocated resources.
    ///
    /// This method should gracefully shut down the renderer,
    /// release any held resources, and prepare for destruction.
    /// Unlike `start()`, this method does not return an error
    /// as cleanup should be best-effort.
    fn stop(&mut self);

    /// Render a single frame using the current renderer state.
    ///
    /// This method performs the actual rendering work for one frame.
    /// The specific rendering algorithm and output depend on the
    /// concrete implementation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful frame rendering, or `Err(String)`
    /// with an error description if rendering fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// loop {
    ///     renderer.render_frame()?;
    /// }
    /// ```
    fn render_frame(&mut self) -> Result<(), String>;

    /// Get the human-readable name of this renderer implementation.
    ///
    /// This method returns a static string that identifies the
    /// specific renderer type or backend being used.
    ///
    /// # Returns
    ///
    /// A static string slice containing the renderer name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// println!("Using renderer: {}", renderer.name());
    /// ```
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
            RendererKind::CpuReference => Box::new(CpuReferenceRenderer::new()),
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

    fn notify(&self, event: RendererEvent) {
        let mut inner = self.inner.lock().unwrap();
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
                    let start_time = std::time::Instant::now();
                    let result = renderer.render_frame();
                    let elapsed = start_time.elapsed();

                    drop(inner);
                    self.notify(RendererEvent::FrameRendered {
                        renderer_kind: kind,
                        frame_number: 0, // TODO: add retrieve and set of frame number provided by renderer
                        frame_time_microseconds: 0, // TODO: set time of event emition
                        render_time_ns: elapsed.as_nanos() as u64,
                    });

                    result
                } else {
                    Err("Active renderer not found".into())
                }
            }
            None => Err("No active renderer".into()),
        }
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