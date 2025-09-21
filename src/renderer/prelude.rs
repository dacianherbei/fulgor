//! Prelude for fulgor::renderer
//!
//! Re-exports all renderers, traits, and async communication types for convenient use.

use crate::renderer::RendererEvent;
pub use super::{Renderer, RendererKind, RendererManager, RendererEventStream};
pub use super::cpu_reference::CpuReferenceRenderer;
pub use super::gpu_optional::GpuOptionalRenderer;

// Async communication types
pub use super::async_communication::{
    AsyncChannelConfig, AsyncEventReceiver, BufferedAsyncSender,
    SendEventError, TrySendEventError,
};

// Common type aliases for frequently used instantiations
pub type BufferedAsyncSenderF32 = BufferedAsyncSender<f32>;
pub type BufferedAsyncSenderF64 = BufferedAsyncSender<f64>;
pub type AsyncChannelConfigF32 = AsyncChannelConfig<f32>;
pub type AsyncChannelConfigF64 = AsyncChannelConfig<f64>;
pub type AsyncEventReceiverF32 = AsyncEventReceiver<RendererEvent, f32>;
pub type AsyncEventReceiverF64 = AsyncEventReceiver<RendererEvent, f64>;