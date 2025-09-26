//! Convenient re-exports for the renderer module.

// Core types and traits (no RendererKind)
pub use super::{RendererEvent, RendererEventStream, RendererError,
                DataPrecision, Capability, ProcessingUnitCapability, Renderer,
                ReferenceRenderer};

// Factory system
pub use super::factory::{RendererFactory, RendererInfo, MockRenderer, MockRendererFactory, ReferenceRendererFactory};
pub use super::manager::RendererManager;

// Custom renderers
pub use super::custom::{OpenGL3Renderer, OpenGL3RendererConfig,
                        OpenGL3RendererBuilder, OpenGL3RendererFactory};

// Async communication
pub use super::async_communication::sender::BufferedAsyncSender;

// Capability system
pub use super::capabilities;

// World and scene management
pub use super::world::{World, Camera, GaussianSplat, Point3D, PrecisionPoint3D};