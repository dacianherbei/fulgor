//! Renderer module of fulgor
//!
//! Provides multiple backends under a unified namespace and a manager for them.

pub mod cpu_reference;
pub mod gpu_optional;
pub mod prelude;

use crate::renderer::cpu_reference::CpuReferenceRenderer;
use crate::renderer::gpu_optional::GpuOptionalRenderer;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};

/// Trait that all renderers implement.
///
/// The `Send + Sync` bounds allow them to be passed between threads or
/// shared across concurrency contexts safely.
pub trait Renderer: Send + Sync {
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self);
    fn render_frame(&mut self) -> Result<(), String>;
    fn name(&self) -> &'static str;
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

/// Events emitted by RendererManager.
#[derive(Debug, Clone)]
pub enum RendererEvent {
    Started(RendererKind),
    Stopped(RendererKind),
    Switched(Option<RendererKind>), // None means no active renderer
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
                    renderer.render_frame()
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
