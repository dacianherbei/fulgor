// src/renderer/prelude.rs

//! Convenient re-exports for the renderer module.
//!
//! This prelude module provides easy access to commonly used types and traits
//! from the renderer system. Import this module to get access to the core
//! renderer functionality without needing to import individual components.
//!
//! # Example
//! ```
//! use crate::renderer::prelude::*;
//!
//! // Now you have access to all core renderer types
//! let manager = RendererManager::new();
//! let factory = MockRendererFactory::new("TestFactory");
//! ```

// Core types and traits
pub use super::{RendererKind, RendererEvent, RendererEventStream,
                DataPrecision, RendererError, RendererInfo};

pub use super::factory::Renderer;

// Manager for factory registration and renderer creation
pub use super::manager::RendererManager;

// Concrete renderer implementations
pub use super::cpu_reference::CpuReferenceRenderer;
pub use super::gpu_optional::GpuOptionalRenderer;

// Async communication utilities
pub use super::async_communication::sender::{BufferedAsyncSender, ChannelConfiguration};

// Factory system
pub use super::factory::{RendererFactory, MockRenderer, MockRendererFactory};