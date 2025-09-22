//! Async communication module for fulgor::renderer
//!
//! Provides templated async communication primitives for event handling
//! with configurable precision and channel behavior.

pub mod sender;
pub mod receiver;
pub mod async_channel;
#[cfg(feature = "tokio-timeout")]
pub mod tokio_enhanced;

// Re-export commonly used types for convenience
pub use renderer::async_communication::async_channel::configuration::AsyncChannelConfig;
pub use renderer::async_communication::receiver::AsyncEventReceiver;
pub use sender::{BufferedAsyncSender};

#[cfg(feature = "tokio-timeout")]
pub use tokio_enhanced::*;
use crate::renderer;