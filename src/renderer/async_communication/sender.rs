//! BufferedAsyncSender implementation for fulgor::renderer
//!
//! Provides a templated async sender with buffering and statistics collection.

use super::types::{AsyncChannelConfig, AsyncEventReceiver};
use crate::renderer::RendererEvent;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::{time};

/// Buffered async sender for renderer events
pub struct BufferedAsyncSender<NumberType = f64>
where
    NumberType: Copy + Clone + Send + Sync + 'static,
{
    sender: async_channel::Sender<RendererEvent>,
    config: AsyncChannelConfig<NumberType>,
    dropped_events: Arc<Mutex<u64>>,
}

impl<NumberType> BufferedAsyncSender<NumberType>
where
    NumberType: Copy + Clone + Send + Sync + Default + 'static,
{
    /// Create a new BufferedAsyncSender with the given configuration
    ///
    /// Returns a tuple of (sender, receiver) for async communication
    pub fn new(
        config: AsyncChannelConfig<NumberType>,
    ) -> (Self, AsyncEventReceiver<RendererEvent, NumberType>) {
        let buffer_size = if config.enable_backpressure {
            config.maximum_buffer_size
        } else {
            // For non-backpressure mode, use unbounded channel
            0
        };

        let (sender, receiver) = if buffer_size > 0 {
            async_channel::bounded(buffer_size)
        } else {
            async_channel::unbounded()
        };

        let buffered_sender = Self {
            sender,
            config: config.clone(),
            dropped_events: Arc::new(Mutex::new(0)),
        };

        let async_receiver = AsyncEventReceiver::new(receiver, config);

        (buffered_sender, async_receiver)
    }

    /// Send an event asynchronously
    pub async fn send_event(&self, event: RendererEvent) -> Result<(), SendEventError> {
        match self.config.send_timeout {
            Some(timeout) => {
                match time::timeout(timeout, self.sender.send(event.clone())).await {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(_)) => Err(SendEventError::ChannelClosed),
                    Err(_) => {
                        // Timeout occurred
                        if !self.config.enable_backpressure {
                            // Drop the event and increment counter
                            self.increment_dropped_events();
                            Err(SendEventError::Timeout)
                        } else {
                            Err(SendEventError::Timeout)
                        }
                    }
                }
            }
            None => {
                // No timeout, send directly
                self.sender
                    .send(event)
                    .await
                    .map_err(|_| SendEventError::ChannelClosed)
            }
        }
    }

    /// Try to send an event without blocking
    pub fn try_send_event(&self, event: RendererEvent) -> Result<(), TrySendEventError> {
        match self.sender.try_send(event) {
            Ok(()) => Ok(()),
            Err(async_channel::TrySendError::Full(dropped_event)) => {
                if !self.config.enable_backpressure {
                    self.increment_dropped_events();
                    Err(TrySendEventError::DroppedDueToBackpressure(dropped_event))
                } else {
                    Err(TrySendEventError::WouldBlock(dropped_event))
                }
            }
            Err(async_channel::TrySendError::Closed(_)) => Err(TrySendEventError::ChannelClosed),
        }
    }

    /// Get the number of dropped events
    pub fn dropped_events_count(&self) -> u64 {
        self.dropped_events
            .lock()
            .map(|count| *count)
            .unwrap_or(0)
    }

    /// Get configuration reference
    pub fn configuration(&self) -> &AsyncChannelConfig<NumberType> {
        &self.config
    }

    /// Check if the channel is closed
    pub fn is_channel_closed(&self) -> bool {
        self.sender.is_closed()
    }

    /// Get approximate number of pending events in the channel
    pub fn pending_events_count(&self) -> usize {
        self.sender.len()
    }

    /// Helper method to increment dropped events counter
    fn increment_dropped_events(&self) {
        if let Ok(mut count) = self.dropped_events.lock() {
            *count += 1;
        }
    }
}

/// Error types for send operations
#[derive(Debug, Clone)]
pub enum SendEventError {
    /// Channel has been closed
    ChannelClosed,
    /// Send operation timed out
    Timeout,
}

#[derive(Debug)]
pub enum TrySendEventError {
    /// Channel is full and would block (with backpressure enabled)
    WouldBlock(RendererEvent),
    /// Event was dropped due to no backpressure policy
    DroppedDueToBackpressure(RendererEvent),
    /// Channel has been closed
    ChannelClosed,
}

// Implement std::ops for mathematical operations on NumberType when needed
impl<NumberType> std::ops::Add<NumberType> for &AsyncChannelConfig<NumberType>
where
    NumberType: Copy + Clone + Send + Sync + std::ops::Add<Output = NumberType> + 'static,
{
    type Output = NumberType;

    fn add(self, rhs: NumberType) -> Self::Output {
        self.precision_threshold + rhs
    }
}

impl<NumberType> std::ops::Mul<NumberType> for &AsyncChannelConfig<NumberType>
where
    NumberType: Copy + Clone + Send + Sync + std::ops::Mul<Output = NumberType> + 'static,
{
    type Output = NumberType;

    fn mul(self, rhs: NumberType) -> Self::Output {
        self.precision_threshold * rhs
    }
}