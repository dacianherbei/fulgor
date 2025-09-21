//! Asynchronous event receiver implementation
//!
//! This module provides the AsyncEventReceiver struct which wraps
//! async_channel::Receiver to provide a clean interface for receiving events.

use async_channel;

/// Asynchronous event receiver that wraps an async_channel::Receiver
///
/// This struct provides a convenient interface for receiving events asynchronously
/// while maintaining the underlying channel's functionality. It is generic over
/// the event type to allow for flexible usage across different event systems.
///
/// # Type Parameters
///
/// * `EventType` - The type of events this receiver will handle
///
/// # Examples
///
/// ```rust
/// use fulgor::AsyncEventReceiver;
/// use fulgor::renderer::{RendererEvent, RendererKind};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
/// let event_receiver = AsyncEventReceiver::new(receiver);
///
/// // Send an event
/// sender.send(RendererEvent::FrameRendered {
///     renderer_kind: RendererKind::CpuReference,
///     frame_number: 1,
///     frame_time_microseconds: 0,
///     render_time_ns: 0,}).await?;
///
/// // Receive the event
/// match event_receiver.recv().await {
///     Ok(event) => println!("Received event: {:?}", event),
///     Err(e) => eprintln!("Failed to receive event: {}", e),
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct AsyncEventReceiver<EventType> {
    receiver: async_channel::Receiver<EventType>,
}

impl<EventType> AsyncEventReceiver<EventType> {
    /// Creates a new AsyncEventReceiver wrapping the provided async_channel::Receiver
    ///
    /// # Parameters
    ///
    /// * `receiver` - The async_channel::Receiver to wrap
    ///
    /// # Returns
    ///
    /// A new AsyncEventReceiver instance
    pub fn new(receiver: async_channel::Receiver<EventType>) -> Self {
        Self { receiver }
    }

    /// Asynchronously receives an event from the channel
    ///
    /// This method will wait until an event is available or the channel is closed.
    /// If the channel is closed and no more events are available, it returns an error.
    ///
    /// # Returns
    ///
    /// * `Ok(EventType)` - The received event
    /// * `Err(async_channel::RecvError)` - If the channel is closed and empty
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use fulgor::AsyncEventReceiver;
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver);
    /// match event_receiver.recv().await {
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
    pub async fn recv(&self) -> Result<EventType, async_channel::RecvError> {
        self.receiver.recv().await
    }

    /// Attempts to receive an event without blocking
    ///
    /// This method returns immediately with either an event if one is available,
    /// or an error indicating why no event could be received.
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
    /// # use fulgor::renderer::RendererEvent;
    /// # use async_channel::TryRecvError;
    /// # use fulgor::AsyncEventReceiver;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver);
    /// match event_receiver.try_recv() {
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
    pub fn try_recv(&self) -> Result<EventType, async_channel::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Checks if the channel is closed
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
    /// # use fulgor::AsyncEventReceiver;
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver);
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

    /// Returns the number of events currently in the channel
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
    /// # use fulgor::AsyncEventReceiver;
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver);
    /// let count = event_receiver.len();
    /// println!("Events in queue: {}", count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Checks if the channel is empty
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
    /// # use fulgor::AsyncEventReceiver;
    /// # use fulgor::renderer::RendererEvent;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
    /// # let event_receiver = AsyncEventReceiver::new(receiver);
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
}

impl<EventType> Clone for AsyncEventReceiver<EventType> {
    /// Creates a clone of this receiver
    ///
    /// Both the original and cloned receivers will receive the same events.
    /// This is useful for distributing events to multiple consumers.
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{RendererEvent, RendererKind};
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_async_event_receiver_basic_functionality() {
        let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
        let event_receiver = AsyncEventReceiver::new(receiver);

        // Test initial state
        assert!(event_receiver.is_empty());
        assert_eq!(event_receiver.len(), 0);
        assert!(!event_receiver.is_closed());

        // Send an event
        let test_event = RendererEvent::FrameRendered {
            renderer_kind: RendererKind::CpuReference, // TODO rederer will set its type
            frame_number: 42,
            frame_time_microseconds: 0, // TODO: pass event emit time
            render_time_ns: 1667
        };

        sender.send(test_event.clone()).await.expect("Failed to send event");

        // Test state after sending
        assert!(!event_receiver.is_empty());
        assert_eq!(event_receiver.len(), 1);

        // Test try_recv
        match event_receiver.try_recv() {
            Ok(received_event) => {
                assert_eq!(
                    std::mem::discriminant(&received_event),
                    std::mem::discriminant(&test_event)
                );
            }
            Err(e) => panic!("try_recv failed: {}", e),
        }

        // Test state after receiving
        assert!(event_receiver.is_empty());
        assert_eq!(event_receiver.len(), 0);
    }

    #[tokio::test]
    async fn test_async_recv() {
        let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
        let event_receiver = AsyncEventReceiver::new(receiver);

        let test_event = RendererEvent::ViewportResized { width: 1920, height: 1080 };

        // Send event in background task
        let sender_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            sender.send(test_event).await.expect("Failed to send event");
        });

        // Receive event with timeout
        let result = timeout(Duration::from_millis(100), event_receiver.recv()).await;

        match result {
            Ok(Ok(_received_event)) => {
                // Success - event was received
            }
            Ok(Err(e)) => panic!("recv failed: {}", e),
            Err(_) => panic!("recv timed out"),
        }

        sender_task.await.expect("Sender task failed");
    }

    #[tokio::test]
    async fn test_try_recv_empty() {
        let (_sender, receiver) = async_channel::unbounded::<RendererEvent>();
        let event_receiver = AsyncEventReceiver::new(receiver);

        match event_receiver.try_recv() {
            Ok(_) => panic!("Expected empty channel"),
            Err(async_channel::TryRecvError::Empty) => {
                // Expected behavior
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_channel_closed() {
        let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
        let event_receiver = AsyncEventReceiver::new(receiver);

        assert!(!event_receiver.is_closed());

        // Drop sender to close channel
        drop(sender);

        assert!(event_receiver.is_closed());

        // Test recv on closed channel
        match event_receiver.recv().await {
            Ok(_) => panic!("Expected channel closed error"),
            Err(async_channel::RecvError) => {
                // Expected behavior
            }
        }

        // Test try_recv on closed channel
        match event_receiver.try_recv() {
            Ok(_) => panic!("Expected channel closed error"),
            Err(async_channel::TryRecvError::Closed) => {
                // Expected behavior
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_clone_receiver() {
        let (sender, receiver) = async_channel::unbounded::<RendererEvent>();
        let event_receiver1 = AsyncEventReceiver::new(receiver);
        let event_receiver2 = event_receiver1.clone();

        let test_event = RendererEvent::SplatDataUpdated { splat_count: 100000 };

        sender.send(test_event).await.expect("Failed to send event");

        // Both receivers should be able to receive the same event
        // (since they share the same underlying channel)
        let received1 = event_receiver1.try_recv();
        let received2 = event_receiver2.try_recv();

        // Only one should succeed since the event is consumed
        assert!(received1.is_ok() || received2.is_ok());
        assert!(!(received1.is_ok() && received2.is_ok()));
    }

    #[tokio::test]
    async fn test_with_custom_event_type() {
        #[derive(Debug, Clone, PartialEq)]
        enum TestEvent {
            Start,
            Data(i32),
            End,
        }

        let (sender, receiver) = async_channel::unbounded::<TestEvent>();
        let event_receiver = AsyncEventReceiver::new(receiver);

        let events = vec![
            TestEvent::Start,
            TestEvent::Data(42),
            TestEvent::Data(84),
            TestEvent::End,
        ];

        // Send all events
        for event in &events {
            sender.send(event.clone()).await.expect("Failed to send event");
        }

        // Verify length
        assert_eq!(event_receiver.len(), events.len());

        // Receive all events and verify order
        let mut received_events = Vec::new();
        for _ in 0..events.len() {
            match event_receiver.recv().await {
                Ok(event) => received_events.push(event),
                Err(e) => panic!("Failed to receive event: {}", e),
            }
        }

        assert_eq!(received_events, events);
        assert!(event_receiver.is_empty());
    }
}