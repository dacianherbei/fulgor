//! Convenient re-exports for the renderer module.
//!
//! This prelude module provides easy access to commonly used types and traits
//! from the renderer system. Import this module to get access to the core
//! renderer functionality without needing to import individual components.
//!
//! # Example
//! ```
//! use crate::renderer::manager::RendererManager;
//! use crate::renderer::factory::MockRendererFactory;
//!
//! // Now you have access to all core renderer types
//! let manager = RendererManager::new();
//! let factory = MockRendererFactory::new("TestFactory");
//! ```

// Core types and traits
pub use super::{RendererKind, RendererEvent, RendererEventStream,
                DataPrecision, RendererError, RendererInfo,
                Capability, ProcessingUnitCapability, Renderer, ReferenceRenderer};

// Re-export factory types with clear naming to avoid confusion
pub use super::factory::{RendererFactory, MockRenderer, MockRendererFactory};

// Manager for factory registration and renderer creation
pub use super::manager::RendererManager;

// Concrete renderer implementations
#[cfg(feature = "gpu")]
pub use super::gpu_optional::GpuOptionalRenderer;

// Async communication utilities
pub use super::async_communication::sender::{BufferedAsyncSender, ChannelConfiguration};

// Capability system
pub use super::capabilities;

// OpenGL3 renderer system
pub use super::custom::opengl3::{OpenGL3Renderer, OpenGL3RendererConfig, OpenGL3RendererBuilder};

// World and scene management
pub use super::world::{World, Camera, GaussianSplat, Point3D, PrecisionPoint3D};