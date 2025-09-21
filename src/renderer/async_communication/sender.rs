//! BufferedAsyncSender implementation for fulgor::renderer
//!
//! Provides a templated async sender with buffering and statistics collection.

use super::types::AsyncChannelConfig;
use crate::renderer::RendererEvent;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;

// /// Buffered async sender for renderer events
// pub struct BufferedAsyncSender<NumberType = f64>
// where
//     NumberType: Copy + Clone + Send + Sync + 'static,
// {
//     sender: async_channel::Sender<RendererEvent>,
//     config: AsyncChannelConfig<NumberType>,
//     dropped_events: Arc<Mutex<u64>>,
// }
//
// impl<NumberType> BufferedAsyncSender<NumberType>
// where
//     NumberType: Copy + Clone + Send + Sync + Default + 'static,
// {
//     /// Create a new BufferedAsyncSender with the given configuration
//     ///
//     /// Returns a tuple of (sender, receiver) for async communication
//     pub fn new(
//         config: AsyncChannelConfig<NumberType>,
//     ) -> (Self, AsyncEventReceiver<RendererEvent, NumberType>) {
//         let buffer_size = if config.enable_backpressure {
//             config.maximum_buffer_size
//         } else {
//             // For non-backpressure mode, use unbounded channel
//             0
//         };
//
//         let (sender, receiver) = if buffer_size > 0 {
//             async_channel::bounded(buffer_size)
//         } else {
//             async_channel::unbounded()
//         };
//
//         let buffered_sender = Self {
//             sender,
//             config: config.clone(),
//             dropped_events: Arc::new(Mutex::new(0)),
//         };
//
//         let async_receiver = AsyncEventReceiver::new(receiver, config);
//
//         (buffered_sender, async_receiver)
//     }
//
//     /// Send an event asynchronously
//     pub async fn send_event(&self, event: RendererEvent) -> Result<(), SendEventError> {
//         match self.config.send_timeout {
//             Some(_timeout) => {
//                 // For minimal dependencies, we use try_send for timeout-like behavior
//                 // If the channel is full and backpressure is disabled, we drop the event
//                 match self.sender.try_send(event.clone()) {
//                     Ok(()) => Ok(()),
//                     Err(async_channel::TrySendError::Full(_)) => {
//                         if self.config.enable_backpressure {
//                             // With backpressure enabled, fall back to blocking send
//                             self.sender
//                                 .send(event)
//                                 .await
//                                 .map_err(|_| SendEventError::ChannelClosed)
//                         } else {
//                             // Drop the event and increment counter
//                             self.increment_dropped_events();
//                             Err(SendEventError::Timeout)
//                         }
//                     }
//                     Err(async_channel::TrySendError::Closed(_)) => Err(SendEventError::ChannelClosed),
//                 }
//             }
//             None => {
//                 // No timeout, send directly
//                 self.sender
//                     .send(event)
//                     .await
//                     .map_err(|_| SendEventError::ChannelClosed)
//             }
//         }
//     }
//
//     /// Try to send an event without blocking
//     pub fn try_send_event(&self, event: RendererEvent) -> Result<(), TrySendEventError> {
//         match self.sender.try_send(event) {
//             Ok(()) => Ok(()),
//             Err(async_channel::TrySendError::Full(dropped_event)) => {
//                 if !self.config.enable_backpressure {
//                     self.increment_dropped_events();
//                     Err(TrySendEventError::DroppedDueToBackpressure(dropped_event))
//                 } else {
//                     Err(TrySendEventError::WouldBlock(dropped_event))
//                 }
//             }
//             Err(async_channel::TrySendError::Closed(_)) => Err(TrySendEventError::ChannelClosed),
//         }
//     }
//
//     /// Get the number of dropped events
//     pub fn dropped_events_count(&self) -> u64 {
//         self.dropped_events
//             .lock()
//             .map(|count| *count)
//             .unwrap_or(0)
//     }
//
//     /// Get configuration reference
//     pub fn configuration(&self) -> &AsyncChannelConfig<NumberType> {
//         &self.config
//     }
//
//     /// Check if the channel is closed
//     pub fn is_channel_closed(&self) -> bool {
//         self.sender.is_closed()
//     }
//
//     /// Get approximate number of pending events in the channel
//     pub fn pending_events_count(&self) -> usize {
//         self.sender.len()
//     }
//
//     /// Helper method to increment dropped events counter
//     pub(crate) fn increment_dropped_events(&self) {
//         if let Ok(mut count) = self.dropped_events.lock() {
//             *count += 1;
//         }
//     }
// }

/// Configuration for the BufferedAsyncSender channel behavior.
#[derive(Debug, Clone)]
pub enum ChannelConfiguration {
    /// Bounded channel with capacity and drop_oldest_on_full behavior.
    Bounded {
        capacity: usize,
        drop_oldest_on_full: bool
    },
    /// Unbounded channel for maximum throughput.
    Unbounded,
}

/// Internal state of the BufferedAsyncSender.
struct BufferedAsyncSenderInner {
    /// Optional bounded sender for bounded channel configuration.
    bounded_sender: Option<mpsc::Sender<RendererEvent>>,
    /// Optional unbounded sender for unbounded channel configuration.
    unbounded_sender: Option<mpsc::UnboundedSender<RendererEvent>>,
    /// Current channel configuration.
    configuration: ChannelConfiguration,
    /// Buffer for implementing drop_oldest_on_full logic.
    event_buffer: Vec<RendererEvent>,
    /// Maximum buffer size for bounded channels.
    buffer_capacity: Option<usize>,
}

/// Async sender for renderer events with buffering and drop policies.
///
/// Generic over event type `EventType` to support different event systems.
#[derive(Clone)]
pub struct BufferedAsyncSender<EventType = RendererEvent>
where
    EventType: Clone + Send + Sync + 'static,
{
    /// Shared inner state protected by mutex.
    inner: Arc<Mutex<BufferedAsyncSenderInner>>,
    /// Atomic counter for dropped events.
    dropped_events_counter: Arc<AtomicU64>,
    /// Phantom data to maintain generic type parameter.
    _phantom: std::marker::PhantomData<EventType>,
}

impl<EventType> BufferedAsyncSender<EventType>
where
    EventType: Clone + Send + Sync + 'static,
{
    /// Create a new BufferedAsyncSender with bounded channel.
    pub fn new_bounded(
        capacity: usize,
        drop_oldest_on_full: bool
    ) -> (Self, mpsc::Receiver<RendererEvent>) {
        let (sender, receiver) = mpsc::channel(capacity);

        let inner = BufferedAsyncSenderInner {
            bounded_sender: Some(sender),
            unbounded_sender: None,
            configuration: ChannelConfiguration::Bounded {
                capacity,
                drop_oldest_on_full
            },
            event_buffer: Vec::with_capacity(if drop_oldest_on_full { capacity } else { 0 }),
            buffer_capacity: Some(capacity),
        };

        let buffered_sender = Self {
            inner: Arc::new(Mutex::new(inner)),
            dropped_events_counter: Arc::new(AtomicU64::new(0)),
            _phantom: std::marker::PhantomData,
        };

        (buffered_sender, receiver)
    }

    /// Create a new BufferedAsyncSender with unbounded channel.
    pub fn new_unbounded() -> (Self, mpsc::UnboundedReceiver<RendererEvent>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        let inner = BufferedAsyncSenderInner {
            bounded_sender: None,
            unbounded_sender: Some(sender),
            configuration: ChannelConfiguration::Unbounded,
            event_buffer: Vec::new(),
            buffer_capacity: None,
        };

        let buffered_sender = Self {
            inner: Arc::new(Mutex::new(inner)),
            dropped_events_counter: Arc::new(AtomicU64::new(0)),
            _phantom: std::marker::PhantomData,
        };

        (buffered_sender, receiver)
    }

    /// Get the current count of dropped events.
    pub fn dropped_events_count(&self) -> u64 {
        self.dropped_events_counter.load(Ordering::Relaxed)
    }

    /// Reset the dropped events counter to zero.
    pub fn reset_dropped_events_counter(&self) {
        self.dropped_events_counter.store(0, Ordering::Relaxed);
    }

    /// Get the current channel configuration.
    pub fn configuration(&self) -> ChannelConfiguration {
        let inner = self.inner.lock().unwrap();
        inner.configuration.clone()
    }
}

impl BufferedAsyncSender<RendererEvent> {
    /// Send an event through the configured channel.
    ///
    /// This method handles the core logic for both bounded and unbounded channels:
    /// - For bounded channels: implements drop_oldest_on_full logic when enabled
    /// - For unbounded channels: sends events asynchronously
    /// - Increments dropped_events counter when events are dropped
    pub async fn send_event(&self, event: RendererEvent) {
        let mut inner = self.inner.lock().unwrap();

        // Extract configuration values and clone senders to avoid borrowing conflicts
        let (is_bounded, capacity, drop_oldest_on_full, bounded_sender, unbounded_sender) = {
            let config = match &inner.configuration {
                ChannelConfiguration::Bounded { capacity, drop_oldest_on_full } => {
                    (true, *capacity, *drop_oldest_on_full)
                }
                ChannelConfiguration::Unbounded => (false, 0, false),
            };
            (
                config.0,
                config.1,
                config.2,
                inner.bounded_sender.clone(),
                inner.unbounded_sender.clone(),
            )
        };

        if is_bounded {
            if let Some(bounded_sender) = bounded_sender {
                // Try to send immediately
                match bounded_sender.try_send(event.clone()) {
                    Ok(()) => {
                        // Event sent successfully
                        return;
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        // Channel is full, handle according to drop policy
                        if drop_oldest_on_full {
                            // Implement drop_oldest_on_full logic
                            inner.event_buffer.push(event);

                            // If buffer exceeds capacity, remove oldest events
                            while inner.event_buffer.len() > capacity {
                                inner.event_buffer.remove(0);
                                self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                            }

                            // Try to flush buffer to channel
                            while let Some(buffered_event) = inner.event_buffer.first() {
                                match bounded_sender.try_send(buffered_event.clone()) {
                                    Ok(()) => {
                                        inner.event_buffer.remove(0);
                                    }
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        // Channel still full, keep remaining events in buffer
                                        break;
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => {
                                        // Channel closed, drop all buffered events
                                        let dropped_count = inner.event_buffer.len();
                                        inner.event_buffer.clear();
                                        self.dropped_events_counter.fetch_add(
                                            dropped_count as u64,
                                            Ordering::Relaxed
                                        );
                                        return;
                                    }
                                }
                            }
                        } else {
                            // Drop current event when channel is full and drop_oldest_on_full is false
                            self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        // Channel is closed, increment dropped counter
                        self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            } else {
                // No bounded sender available, drop event
                self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
            }
        } else {
            // Unbounded channel
            if let Some(unbounded_sender) = unbounded_sender {
                // Release the lock before awaiting
                drop(inner);

                // Send asynchronously for unbounded channels
                match unbounded_sender.send(event) {
                    Ok(()) => {
                        // Event sent successfully
                    }
                    Err(_) => {
                        // Channel is closed, increment dropped counter
                        self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            } else {
                // No unbounded sender available, drop event
                self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_bounded_channel_normal_send() {
        let (sender, mut receiver) = BufferedAsyncSender::new_bounded(5, false);
        let event = RendererEvent::Started(crate::renderer::RendererKind::CpuReference);

        sender.send_event(event.clone()).await;

        let received = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(received.is_ok());
        assert_eq!(sender.dropped_events_count(), 0);
    }

    #[tokio::test]
    async fn test_bounded_channel_drop_on_full() {
        let (sender, mut receiver) = BufferedAsyncSender::new_bounded(2, false);

        // Fill the channel
        for i in 0..3 {
            let event = RendererEvent::Started(crate::renderer::RendererKind::CpuReference);
            sender.send_event(event).await;
        }

        // Should have dropped one event
        assert_eq!(sender.dropped_events_count(), 1);
    }

    #[tokio::test]
    async fn test_bounded_channel_drop_oldest_on_full() {
        let (sender, mut receiver) = BufferedAsyncSender::new_bounded(2, true);

        // Send events to fill channel and buffer
        for i in 0..5 {
            let event = RendererEvent::Started(crate::renderer::RendererKind::CpuReference);
            sender.send_event(event).await;
        }

        // Should have dropped some events due to buffer overflow
        assert!(sender.dropped_events_count() > 0);
    }

    #[tokio::test]
    async fn test_unbounded_channel_send() {
        let (sender, mut receiver) = BufferedAsyncSender::new_unbounded();
        let event = RendererEvent::Started(crate::renderer::RendererKind::CpuReference);

        sender.send_event(event.clone()).await;

        let received = timeout(Duration::from_millis(100), receiver.recv()).await;
        assert!(received.is_ok());
        assert_eq!(sender.dropped_events_count(), 0);
    }

    #[tokio::test]
    async fn test_dropped_events_counter_reset() {
        let (sender, _receiver) = BufferedAsyncSender::new_bounded(1, false);

        // Force some drops
        for i in 0..3 {
            let event = RendererEvent::Started(crate::renderer::RendererKind::CpuReference);
            sender.send_event(event).await;
        }

        let dropped_before = sender.dropped_events_count();
        assert!(dropped_before > 0);

        sender.reset_dropped_events_counter();
        assert_eq!(sender.dropped_events_count(), 0);
    }
}