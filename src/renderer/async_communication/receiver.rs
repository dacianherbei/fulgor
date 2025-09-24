// File: src/renderer/async_communication/receiver.rs
//! Asynchronous event receiver implementation for templated event communication.
//!
//! This module provides the AsyncEventReceiver struct which wraps async_channel::Receiver
//! with additional configuration, statistics tracking, and Stream implementation for use
//! in async workflows.

use std::future::Future;
use async_channel;
use futures::Stream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use super::async_channel::AsyncChannelConfig;

/// Asynchronous event receiver with templated precision and rich configuration support.
///
/// This struct provides a comprehensive interface for receiving events asynchronously
/// while maintaining configuration, statistics, and supporting the futures::Stream trait.
/// It is generic over both the event type and numerical precision type to allow for
/// flexible usage across different event systems and precision requirements.
///
/// # Type Parameters
///
/// * `EventType` - The type of events this receiver will handle
/// * `NumberType` - The numerical type used for precision-dependent operations (default: f64)
///
/// # Examples
///
/// ```rust
/// use fulgor::renderer::async_communication::{AsyncEventReceiver, AsyncChannelConfig};
/// use fulgor::renderer::RendererEvent;
/// use fulgor::renderer::RendererId;
/// use futures::StreamExt;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = AsyncChannelConfig::bounded(1000);
/// let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
/// let mut event_receiver = AsyncEventReceiver::new(receiver, config);
///
/// // Send an event
/// sender.send(RendererEvent::FrameRendered {
///     id: RendererId(1),
///     frame_number: 1,
///     frame_time_microseconds: 0,
///     render_time_ns: 1667
/// }).await?;
///
/// // Receive using async method
/// match event_receiver.receive_event().await {
///     Ok(event) => println!("Received event: {:?}", event),
///     Err(e) => eprintln!("Failed to receive event: {}", e),
/// }
///
/// // Or use as a Stream
/// if let Some(event) = event_receiver.next().await {
///     println!("Streamed event: {:?}", event);
/// }
///
/// // Check statistics
/// println!("Events received: {}", event_receiver.received_events_count());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct AsyncEventReceiver<EventType>
where
    EventType: Clone + Send + Sync + 'static
{
    /// The underlying async channel receiver
    receiver: async_channel::Receiver<EventType>,

    /// Configuration for this receiver instance
    config: AsyncChannelConfig,

    /// Thread-safe counter for tracking received events
    received_events_count: Arc<Mutex<u64>>,
}

impl<EventType> AsyncEventReceiver<EventType>
where
    EventType: Clone + Send + Sync + 'static
{
    /// Creates a new AsyncEventReceiver with the specified configuration.
    ///
    /// # Parameters
    ///
    /// * `receiver` - The async_channel::Receiver to wrap
    /// * `config` - Configuration for channel behavior and statistics
    ///
    /// # Returns
    ///
    /// A new AsyncEventReceiver instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fulgor::renderer::async_communication::{AsyncEventReceiver, AsyncChannelConfig};
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = AsyncChannelConfig::bounded(100);
    /// let (sender, receiver) = async_channel::bounded::<RendererEvent>(100);
    /// let event_receiver = AsyncEventReceiver::new(receiver, config);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        receiver: async_channel::Receiver<EventType>,
        config: AsyncChannelConfig,
    ) -> Self {
        Self {
            receiver,
            config,
            received_events_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Asynchronously receives an event from the channel.
    ///
    /// This method will wait until an event is available or the channel is closed.
    /// If the channel is closed and no more events are available, it returns an error.
    /// Event reception is counted for statistics tracking.
    ///
    /// # Returns
    ///
    /// * `Ok(EventType)` - The received event
    /// * `Err(async_channel::RecvError)` - If the channel is closed and empty
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fulgor::renderer::async_communication::{AsyncEventReceiver, AsyncChannelConfig};
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = AsyncChannelConfig::default();
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver, config);
    /// match event_receiver.receive_event().await {
    ///     Ok(event) => {
    ///         // Process the received event
    ///         println!("Received: {:?}", event);
    ///     }
    ///     Err(e) => {
    ///         // Handle receive error (channel closed)
    ///         eprintln!("Channel closed: {}", e);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn receive_event(&self) -> Result<EventType, async_channel::RecvError> {
        match self.receiver.recv().await {
            Ok(event) => {
                if let Ok(mut count) = self.received_events_count.lock() {
                    *count += 1;
                }
                Ok(event)
            }
            Err(error) => Err(error),
        }
    }

    /// Attempts to receive an event without blocking.
    ///
    /// This method returns immediately with either an event if one is available,
    /// or an error indicating why no event could be received. Event reception
    /// is counted for statistics tracking.
    ///
    /// # Returns
    ///
    /// * `Ok(EventType)` - The received event
    /// * `Err(async_channel::TryRecvError::Empty)` - If no events are currently available
    /// * `Err(async_channel::TryRecvError::Closed)` - If the channel is closed
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fulgor::renderer::async_communication::{AsyncEventReceiver, AsyncChannelConfig};
    /// # use fulgor::renderer::RendererEvent;
    /// # use async_channel::TryRecvError;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = AsyncChannelConfig::default();
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver, config);
    /// match event_receiver.try_receive_event() {
    ///     Ok(event) => {
    ///         println!("Received immediately: {:?}", event);
    ///     }
    ///     Err(TryRecvError::Empty) => {
    ///         println!("No events available right now");
    ///     }
    ///     Err(TryRecvError::Closed) => {
    ///         println!("Channel is closed");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_receive_event(&self) -> Result<EventType, async_channel::TryRecvError> {
        match self.receiver.try_recv() {
            Ok(event) => {
                if let Ok(mut count) = self.received_events_count.lock() {
                    *count += 1;
                }
                Ok(event)
            }
            Err(error) => Err(error),
        }
    }

    /// Gets the total number of events received by this receiver.
    ///
    /// This count represents all events that have been successfully received
    /// through either `receive_event()` or `try_receive_event()` methods,
    /// including events received via the Stream interface.
    ///
    /// # Returns
    ///
    /// The total number of events received, or 0 if the counter cannot be accessed
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fulgor::renderer::async_communication::{AsyncEventReceiver, AsyncChannelConfig};
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = AsyncChannelConfig::default();
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver, config);
    /// let count = event_receiver.received_events_count();
    /// println!("Total events received: {}", count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn received_events_count(&self) -> u64 {
        self.received_events_count
            .lock()
            .map(|count| *count)
            .unwrap_or(0)
    }

    /// Gets a reference to the configuration used by this receiver.
    ///
    /// # Returns
    ///
    /// A reference to the AsyncChannelConfig used by this receiver
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fulgor::renderer::async_communication::{AsyncEventReceiver, AsyncChannelConfig};
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = AsyncChannelConfig::default();
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver, config);
    /// let config = event_receiver.configuration();
    /// println!("Buffer size: {}", config.maximum_buffer_size);
    /// println!("Has backpressure: {}", config.has_backpressure());
    /// # Ok(())
    /// # }
    /// ```
    pub fn configuration(&self) -> &AsyncChannelConfig {
        &self.config
    }

    /// Checks if the underlying channel is closed.
    ///
    /// A channel is considered closed when all senders have been dropped.
    /// Once closed, no new events can be sent, but existing events in the
    /// channel can still be received.
    ///
    /// # Returns
    ///
    /// `true` if the channel is closed, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fulgor::renderer::async_communication::{AsyncEventReceiver, AsyncChannelConfig};
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = AsyncChannelConfig::default();
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver, config);
    /// if event_receiver.is_closed() {
    ///     println!("No more events will be sent");
    /// } else {
    ///     println!("Channel is still active");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_closed(&self) -> bool {
        self.receiver.is_closed()
    }

    /// Returns the number of events currently queued in the channel.
    ///
    /// This count represents events that are buffered and ready to be received.
    /// The actual number may change between calls due to concurrent operations.
    ///
    /// # Returns
    ///
    /// The number of events currently queued in the channel
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fulgor::renderer::async_communication::{AsyncEventReceiver, AsyncChannelConfig};
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = AsyncChannelConfig::default();
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver, config);
    /// let count = event_receiver.len();
    /// println!("Events in queue: {}", count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Checks if the channel queue is empty.
    ///
    /// Returns `true` if there are no events currently available to receive.
    /// This is equivalent to checking if `len() == 0`.
    ///
    /// # Returns
    ///
    /// `true` if no events are currently queued, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fulgor::renderer::async_communication::{AsyncEventReceiver, AsyncChannelConfig};
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = AsyncChannelConfig::default();
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver, config);
    /// if event_receiver.is_empty() {
    ///     println!("No events available");
    /// } else {
    ///     println!("Events are waiting to be processed");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }

    /// Internal method to increment the received events counter.
    ///
    /// This is used by the Stream implementation to maintain consistent
    /// statistics tracking across all reception methods.
    fn increment_received_count(&self) {
        if let Ok(mut count) = self.received_events_count.lock() {
            *count += 1;
        }
    }
}

impl<EventType> Clone for AsyncEventReceiver<EventType>
where
    EventType: Clone + Send + Sync + 'static,
    
{
    /// Creates a clone of this receiver.
    ///
    /// Both the original and cloned receivers will receive the same events.
    /// This is useful for distributing events to multiple consumers.
    /// The cloned receiver will have its own statistics counter.
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            config: self.config.clone(),
            received_events_count: Arc::new(Mutex::new(0)),
        }
    }
}

/// Implement Unpin for AsyncEventReceiver to enable proper Stream usage.
///
/// This is safe because AsyncEventReceiver doesn't contain any self-referential
/// data and can be moved freely. The underlying async_channel::Receiver handles
/// its own pinning internally.
impl<EventType> Unpin for AsyncEventReceiver<EventType>
where
    EventType: Clone + Send + Sync + 'static,
    
{}

/// Implementation of futures::Stream for AsyncEventReceiver.
///
/// This allows AsyncEventReceiver to be used with async stream combinators
/// and in for-await loops. The implementation:
/// - Tries to receive events without blocking first for efficiency
/// - Registers waker for future notifications when no events are available
/// - Returns Poll::Ready(None) when the channel is closed
/// - Maintains event statistics consistently with other reception methods
impl<EventType> Stream for AsyncEventReceiver<EventType>
where
    EventType: Clone + Send + Sync + 'static,
    
{
    type Item = EventType;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Try to receive an event without blocking first for efficiency
        match self.receiver.try_recv() {
            Ok(event) => {
                // Event available immediately, update statistics
                self.increment_received_count();
                Poll::Ready(Some(event))
            }
            Err(async_channel::TryRecvError::Empty) => {
                // No events available, but channel might still be open
                // Check if channel is closed after the empty check
                if self.receiver.is_closed() {
                    // Channel is closed and empty - end of stream
                    Poll::Ready(None)
                } else {
                    // Channel is open but empty, poll the receiver future to register waker
                    let receiver_future = self.receiver.recv();
                    futures::pin_mut!(receiver_future);

                    match receiver_future.poll(cx) {
                        Poll::Ready(Ok(event)) => {
                            // Event became available during polling
                            self.increment_received_count();
                            Poll::Ready(Some(event))
                        }
                        Poll::Ready(Err(_)) => {
                            // Channel closed during polling
                            Poll::Ready(None)
                        }
                        Poll::Pending => {
                            // Waker has been registered by the receiver's poll implementation
                            // Will be woken when an event arrives or channel closes
                            Poll::Pending
                        }
                    }
                }
            }
            Err(async_channel::TryRecvError::Closed) => {
                // Channel is closed - end of stream
                Poll::Ready(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{RendererEvent};
    use futures::StreamExt;
    use tokio::time::{Duration};
    use crate::renderer::manager::RendererId;

    #[tokio::test]
    async fn test_async_event_receiver_basic_functionality() {
        let config = AsyncChannelConfig::bounded(100);
        let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
        let event_receiver = AsyncEventReceiver::new(receiver, config);

        // Test initial state
        assert!(event_receiver.is_empty());
        assert_eq!(event_receiver.len(), 0);
        assert!(!event_receiver.is_closed());
        assert_eq!(event_receiver.received_events_count(), 0);

        // Send an event
        let test_event = RendererEvent::FrameRendered {
            id: RendererId(1),
            frame_number: 42,
            frame_time_microseconds: 0,
            render_time_ns: 1667,
        };

        sender
            .send(test_event.clone())
            .await
            .expect("Failed to send event");

        // Test state after sending
        assert!(!event_receiver.is_empty());
        assert_eq!(event_receiver.len(), 1);

        // Test try_receive_event
        match event_receiver.try_receive_event() {
            Ok(received_event) => {
                assert_eq!(
                    std::mem::discriminant(&received_event),
                    std::mem::discriminant(&test_event)
                );
            }
            Err(e) => panic!("try_receive_event failed: {}", e),
        }

        // Test state after receiving
        assert!(event_receiver.is_empty());
        assert_eq!(event_receiver.len(), 0);
        assert_eq!(event_receiver.received_events_count(), 1);
    }

    #[tokio::test]
    async fn test_stream_basic_functionality() {
        let config = AsyncChannelConfig::bounded(100);
        let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
        let mut event_receiver = AsyncEventReceiver::new(receiver, config);

        // Send some events
        let test_events = vec![
            RendererEvent::FrameRendered {
                id: RendererId(1),
                frame_number: 1,
                frame_time_microseconds: 0,
                render_time_ns: 1000,
            },
            RendererEvent::ViewportResized {
                id: RendererId(1),
                width: 1920,
                height: 1080,
            },
            RendererEvent::SplatDataUpdated {
                id: RendererId(1),
                splat_count: 50000
            },
        ];

        for event in &test_events {
            sender
                .send(event.clone())
                .await
                .expect("Failed to send event");
        }

        // Collect events using Stream API
        let mut collected_events = Vec::new();
        for _ in 0..test_events.len() {
            if let Some(event) = event_receiver.next().await {
                collected_events.push(event);
            }
        }

        assert_eq!(collected_events.len(), test_events.len());
        assert_eq!(event_receiver.received_events_count(), test_events.len() as u64);
    }

    #[tokio::test]
    async fn test_stream_channel_closed() {
        let config = AsyncChannelConfig::new(
            1000,
            Some(Duration::from_millis(100)),
            false,
            Duration::from_secs(1)
        );
        let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
        let mut event_receiver = AsyncEventReceiver::new(receiver, config);

        // Send one event then close channel
        let test_event = RendererEvent::ViewportResized {
            id: RendererId(1),
            width: 800,
            height: 600,
        };
        sender.send(test_event).await.expect("Failed to send event");
        drop(sender); // Close the channel

        // Should receive the event first
        let first_event = event_receiver.next().await;
        assert!(first_event.is_some());
        assert_eq!(event_receiver.received_events_count(), 1);

        // Then should receive None indicating stream end
        let second_event = event_receiver.next().await;
        assert!(second_event.is_none());
        assert!(event_receiver.is_closed());
    }

    #[tokio::test]
    async fn test_configuration_access() {
        let config = AsyncChannelConfig::bounded_with_backpressure(500);
        let (_, receiver) = async_channel::bounded::<RendererEvent>(100);
        let event_receiver = AsyncEventReceiver::new(receiver, config);

        let retrieved_config = event_receiver.configuration();
        assert_eq!(retrieved_config.maximum_buffer_size, 500);
        assert!(retrieved_config.has_backpressure());
    }

    #[tokio::test]
    async fn test_clone_receiver() {
        let config = AsyncChannelConfig::unbounded();
        let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
        let event_receiver1 = AsyncEventReceiver::new(receiver, config);
        let event_receiver2 = event_receiver1.clone();

        let test_event = RendererEvent::SplatDataUpdated {id: RendererId(1), splat_count: 100000 };

        sender.send(test_event).await.expect("Failed to send event");

        // Both receivers should be able to receive the same event
        // (since they share the same underlying channel)
        let received1 = event_receiver1.try_receive_event();
        let received2 = event_receiver2.try_receive_event();

        // Only one should succeed since the event is consumed
        assert!(received1.is_ok() || received2.is_ok());
        assert!(!(received1.is_ok() && received2.is_ok()));

        // Statistics should be independent
        if received1.is_ok() {
            assert_eq!(event_receiver1.received_events_count(), 1);
            assert_eq!(event_receiver2.received_events_count(), 0);
        } else {
            assert_eq!(event_receiver1.received_events_count(), 0);
            assert_eq!(event_receiver2.received_events_count(), 1);
        }
    }

    #[tokio::test]
    async fn test_with_custom_event_type() {
        #[derive(Debug, Clone, PartialEq)]
        enum CustomEvent {
            Start,
            Progress(u32),
            Complete,
        }

        let config = AsyncChannelConfig::bounded(10);
        let (sender, receiver) = async_channel::unbounded::<CustomEvent>();
        let mut event_receiver = AsyncEventReceiver::new(receiver, config);

        let test_events = vec![
            CustomEvent::Start,
            CustomEvent::Progress(50),
            CustomEvent::Progress(100),
            CustomEvent::Complete,
        ];

        // Send events
        for event in &test_events {
            sender
                .send(event.clone())
                .await
                .expect("Failed to send event");
        }

        // Collect using stream (manually collect to avoid consuming the receiver)
        let mut collected = Vec::new();
        for _ in 0..test_events.len() {
            if let Some(event) = event_receiver.next().await {
                collected.push(event);
            }
        }

        assert_eq!(collected, test_events);
        assert_eq!(event_receiver.received_events_count(), test_events.len() as u64);
    }

    #[tokio::test]
    async fn test_mixed_reception_methods() {
        let config = AsyncChannelConfig::default();
        let (sender, receiver) = async_channel::unbounded::<i32>();
        let mut event_receiver = AsyncEventReceiver::new(receiver, config);

        // Send multiple events
        for i in 1..=5 {
            sender.send(i).await.expect("Failed to send event");
        }

        // Mix different reception methods
        let _event1 = event_receiver.try_receive_event().expect("Should receive event");
        let _event2 = event_receiver.receive_event().await.expect("Should receive event");
        let _event3 = event_receiver.next().await.expect("Should receive event");

        // All should count towards statistics
        assert_eq!(event_receiver.received_events_count(), 3);
        assert_eq!(event_receiver.len(), 2); // 2 events remaining
    }
}