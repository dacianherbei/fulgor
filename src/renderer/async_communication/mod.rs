//! Async communication module for fulgor::renderer
//!
//! Provides templated async communication primitives for event handling
//! with configurable precision and channel behavior.

pub mod types;
pub mod sender;
pub mod receiver;
pub mod async_channel;

// Re-export commonly used types for convenience
pub use types::{AsyncChannelConfig, AsyncEventReceiver};
pub use sender::{BufferedAsyncSender, SendEventError, TrySendEventError};