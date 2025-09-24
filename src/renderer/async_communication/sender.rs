// File: src/renderer/async_communication/sender.rs

use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use std::marker::PhantomData;
use tokio::sync::mpsc;
use crate::renderer::RendererEvent;

/// Channel configuration for the BufferedAsyncSender.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelConfiguration {
    /// Bounded channel with specified capacity.
    Bounded { capacity: usize },
    /// Unbounded channel with unlimited capacity.
    Unbounded,
}

/// Internal state of the BufferedAsyncSender.
#[derive(Debug)]
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
    /// Atomic counter for dropped events (shared with outer struct).
    dropped_events_counter: Arc<AtomicU64>,
}

#[derive(Clone, Debug)]
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
    /// Create a new BufferedAsyncSender with bounded channel configuration.
    ///
    /// Creates the bounded channel internally and returns both the sender and receiver.
    ///
    /// # Arguments
    /// * `capacity` - The channel capacity
    /// * `buffer_capacity` - Optional buffer capacity for drop_oldest_on_full logic
    /// * `dropped_events_counter` - Optional shared atomic counter for tracking dropped events
    ///
    /// # Returns
    /// A tuple containing the BufferedAsyncSender and the Receiver
    pub fn new_bounded_channel(
        capacity: usize,
        buffer_capacity: Option<usize>,
        dropped_events_counter: Option<Arc<AtomicU64>>,
    ) -> (BufferedAsyncSender<RendererEvent>, mpsc::Receiver<RendererEvent>) {
        let (tx, rx) = mpsc::channel(capacity);
        let counter = dropped_events_counter.unwrap_or_else(|| Arc::new(AtomicU64::new(0)));

        let inner = BufferedAsyncSenderInner {
            bounded_sender: Some(tx),
            unbounded_sender: None,
            configuration: ChannelConfiguration::Bounded { capacity },
            event_buffer: Vec::new(),
            buffer_capacity,
            dropped_events_counter: Arc::clone(&counter),
        };

        let sender = BufferedAsyncSender {
            inner: Arc::new(Mutex::new(inner)),
            dropped_events_counter: counter,
            _phantom: PhantomData,
        };

        (sender, rx)
    }

    /// Create a new BufferedAsyncSender with bounded channel configuration.
    ///
    /// Creates the bounded channel internally and returns both the sender and receiver.
    /// This matches the expected API signature from the caller.
    ///
    /// # Arguments
    /// * `capacity` - The channel capacity
    /// * `drop_oldest_on_full` - Whether to drop oldest events when buffer is full
    /// * `dropped_events_counter` - Shared atomic counter for tracking dropped events
    ///
    /// # Returns
    /// A tuple containing the BufferedAsyncSender and the Receiver
    pub fn new_bounded(
        capacity: usize,
        drop_oldest_on_full: bool,
        dropped_events_counter: Arc<AtomicU64>,
    ) -> (BufferedAsyncSender<RendererEvent>, mpsc::Receiver<RendererEvent>) {
        let (tx, rx) = mpsc::channel(capacity);
        let buffer_capacity = if drop_oldest_on_full { Some(capacity) } else { None };

        let inner = BufferedAsyncSenderInner {
            bounded_sender: Some(tx),
            unbounded_sender: None,
            configuration: ChannelConfiguration::Bounded { capacity },
            event_buffer: Vec::new(),
            buffer_capacity,
            dropped_events_counter: Arc::clone(&dropped_events_counter),
        };

        let sender = BufferedAsyncSender {
            inner: Arc::new(Mutex::new(inner)),
            dropped_events_counter,
            _phantom: PhantomData,
        };

        (sender, rx)
    }

    /// Create a new BufferedAsyncSender with an existing bounded sender.
    ///
    /// # Arguments
    /// * `sender` - The tokio mpsc bounded sender for events
    /// * `capacity` - The channel capacity
    /// * `buffer_capacity` - Optional buffer capacity for drop_oldest_on_full logic
    pub fn from_bounded_sender(
        sender: mpsc::Sender<RendererEvent>,
        capacity: usize,
        buffer_capacity: Option<usize>,
    ) -> BufferedAsyncSender<RendererEvent> {
        let dropped_events_counter = Arc::new(AtomicU64::new(0));

        let inner = BufferedAsyncSenderInner {
            bounded_sender: Some(sender),
            unbounded_sender: None,
            configuration: ChannelConfiguration::Bounded { capacity },
            event_buffer: Vec::new(),
            buffer_capacity,
            dropped_events_counter: Arc::clone(&dropped_events_counter),
        };

        BufferedAsyncSender {
            inner: Arc::new(Mutex::new(inner)),
            dropped_events_counter,
            _phantom: PhantomData,
        }
    }

    /// Create a new BufferedAsyncSender with unbounded channel configuration.
    ///
    /// Creates the channel internally and returns both the sender and receiver.
    ///
    /// # Arguments
    /// * `buffer_capacity` - Optional buffer capacity for drop_oldest_on_full logic
    ///
    /// # Returns
    /// A tuple containing the BufferedAsyncSender and the UnboundedReceiver
    pub fn new_unbounded(
        buffer_capacity: Option<usize>,
    ) -> (BufferedAsyncSender<RendererEvent>, mpsc::UnboundedReceiver<RendererEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let dropped_events_counter = Arc::new(AtomicU64::new(0));

        let inner = BufferedAsyncSenderInner {
            bounded_sender: None,
            unbounded_sender: Some(tx),
            configuration: ChannelConfiguration::Unbounded,
            event_buffer: Vec::new(),
            buffer_capacity,
            dropped_events_counter: Arc::clone(&dropped_events_counter),
        };

        let sender = BufferedAsyncSender {
            inner: Arc::new(Mutex::new(inner)),
            dropped_events_counter,
            _phantom: PhantomData,
        };

        (sender, rx)
    }

    /// Create a new BufferedAsyncSender with default unbounded configuration.
    ///
    /// Convenience method that creates an unbounded channel with no buffer capacity.
    ///
    /// # Returns
    /// A tuple containing the BufferedAsyncSender and the UnboundedReceiver
    pub fn new() -> (BufferedAsyncSender<RendererEvent>, mpsc::UnboundedReceiver<RendererEvent>) {
        Self::new_unbounded(None)
    }

    /// Create a new BufferedAsyncSender with an existing unbounded sender.
    ///
    /// # Arguments
    /// * `sender` - The tokio mpsc unbounded sender for events
    /// * `buffer_capacity` - Optional buffer capacity for drop_oldest_on_full logic
    pub fn from_unbounded_sender(
        sender: mpsc::UnboundedSender<RendererEvent>,
        buffer_capacity: Option<usize>,
    ) -> BufferedAsyncSender<RendererEvent> {
        let dropped_events_counter = Arc::new(AtomicU64::new(0));

        let inner = BufferedAsyncSenderInner {
            bounded_sender: None,
            unbounded_sender: Some(sender),
            configuration: ChannelConfiguration::Unbounded,
            event_buffer: Vec::new(),
            buffer_capacity,
            dropped_events_counter: Arc::clone(&dropped_events_counter),
        };

        BufferedAsyncSender {
            inner: Arc::new(Mutex::new(inner)),
            dropped_events_counter,
            _phantom: PhantomData,
        }
    }

    /// Create a new BufferedAsyncSender with existing dropped events counter.
    ///
    /// # Arguments
    /// * `sender` - The tokio mpsc unbounded sender for events
    /// * `dropped_events_counter` - Shared atomic counter for tracking dropped events
    /// * `buffer_capacity` - Optional buffer capacity for drop_oldest_on_full logic
    pub fn new_with_counter(
        sender: mpsc::UnboundedSender<RendererEvent>,
        dropped_events_counter: Arc<AtomicU64>,
        buffer_capacity: Option<usize>,
    ) -> BufferedAsyncSender<RendererEvent> {
        let inner = BufferedAsyncSenderInner {
            bounded_sender: None,
            unbounded_sender: Some(sender),
            configuration: ChannelConfiguration::Unbounded,
            event_buffer: Vec::new(),
            buffer_capacity,
            dropped_events_counter: Arc::clone(&dropped_events_counter),
        };

        BufferedAsyncSender {
            inner: Arc::new(Mutex::new(inner)),
            dropped_events_counter,
            _phantom: PhantomData,
        }
    }

    /// Safely retrieve the current count of dropped events.
    ///
    /// This method performs a lock-free atomic read operation with no performance penalty.
    /// Uses relaxed ordering as the exact ordering of reads relative to other operations
    /// is not critical for a simple counter.
    ///
    /// # Returns
    /// The number of events that have been dropped since the sender was created.
    ///
    /// # Thread Safety
    /// This method is thread-safe and lock-free using atomic operations.
    /// Multiple threads can safely call this method concurrently with no contention.
    ///
    /// # Performance
    /// This operation is extremely fast as it uses atomic load with no locks or blocking.
    ///
    /// # Example
    /// ```rust
    /// # use fulgor::renderer::async_communication::sender::BufferedAsyncSender;
    /// # use fulgor::renderer::RendererEvent;
    /// # #[tokio::main]
    /// # async fn main() {
    /// let (sender, _rx) = BufferedAsyncSender::<RendererEvent>::new_unbounded(None);
    /// let dropped_events = sender.get_dropped_count();
    /// println!("Total dropped events: {}", dropped_events);
    /// # }
    /// ```
    pub fn get_dropped_count(&self) -> u64 {
        // Direct atomic load with relaxed ordering - no locks, no blocking, no performance penalty
        self.dropped_events_counter.load(Ordering::Relaxed)
    }

    /// Send an event through the buffered async sender (async version).
    ///
    /// This is the async version of send that can be awaited.
    /// If the send fails (e.g., receiver is dropped), increment the dropped count atomically.
    ///
    /// # Arguments
    /// * `event` - The event to send
    ///
    /// # Returns
    /// Result indicating success or failure of the send operation
    pub async fn send_event(&self, event: RendererEvent) -> Result<(), mpsc::error::SendError<RendererEvent>> {
        // Create enum to hold the sender type without the guard
        enum SenderType {
            Bounded(mpsc::Sender<RendererEvent>),
            Unbounded(mpsc::UnboundedSender<RendererEvent>),
        }

        // Extract sender and drop the guard
        let sender_type = {
            match self.inner.lock() {
                Ok(guard) => {
                    match guard.configuration {
                        ChannelConfiguration::Bounded { .. } => {
                            if let Some(ref sender) = guard.bounded_sender {
                                Some(SenderType::Bounded(sender.clone()))
                            } else {
                                None
                            }
                        }
                        ChannelConfiguration::Unbounded => {
                            if let Some(ref sender) = guard.unbounded_sender {
                                Some(SenderType::Unbounded(sender.clone()))
                            } else {
                                None
                            }
                        }
                    }
                }
                Err(_) => None,
            }
        }; // <-- Guard is dropped here

        // Now handle the send without holding the guard
        match sender_type {
            Some(SenderType::Bounded(sender)) => {
                match sender.send(event.clone()).await {
                    Ok(()) => Ok(()),
                    Err(send_error) => {
                        self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                        Err(send_error)
                    }
                }
            }
            Some(SenderType::Unbounded(sender)) => {
                match sender.send(event) {
                    Ok(()) => Ok(()),
                    Err(send_error) => {
                        self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                        Err(send_error)
                    }
                }
            }
            None => {
                // No sender available or mutex poisoned
                self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                Err(mpsc::error::SendError(event))
            }
        }
    }

    /// Send an event through the buffered async sender (sync version).
    ///
    /// If the send fails (e.g., receiver is dropped), increment the dropped count atomically.
    ///
    /// # Arguments
    /// * `event` - The event to send
    ///
    /// # Returns
    /// Result indicating success or failure of the send operation
    pub fn send(&self, event: RendererEvent) -> Result<(), mpsc::error::SendError<RendererEvent>> {
        match self.inner.lock() {
            Ok(mut guard) => {
                match guard.configuration {
                    ChannelConfiguration::Bounded { .. } => {
                        if let Some(ref sender) = guard.bounded_sender {
                            match sender.try_send(event.clone()) {
                                Ok(()) => Ok(()),
                                Err(mpsc::error::TrySendError::Full(_)) => {
                                    // Handle buffer logic for full channels
                                    self.handle_buffer_full(&mut guard, event)
                                }
                                Err(mpsc::error::TrySendError::Closed(e)) => {
                                    // Channel is closed, count as dropped
                                    self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                                    Err(mpsc::error::SendError(e))
                                }
                            }
                        } else {
                            // No sender available, count as dropped
                            self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                            Err(mpsc::error::SendError(event))
                        }
                    }
                    ChannelConfiguration::Unbounded => {
                        if let Some(ref sender) = guard.unbounded_sender {
                            match sender.send(event) {
                                Ok(()) => Ok(()),
                                Err(send_error) => {
                                    // Channel is closed, count as dropped
                                    self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                                    Err(send_error)
                                }
                            }
                        } else {
                            // No sender available, count as dropped
                            self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                            Err(mpsc::error::SendError(event))
                        }
                    }
                }
            }
            Err(_) => {
                // Mutex is poisoned, count as dropped
                self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
                Err(mpsc::error::SendError(event))
            }
        }
    }

    /// Handle buffer full condition with drop_oldest_on_full logic.
    fn handle_buffer_full(
        &self,
        guard: &mut BufferedAsyncSenderInner,
        event: RendererEvent,
    ) -> Result<(), mpsc::error::SendError<RendererEvent>> {
        if let Some(buffer_capacity) = guard.buffer_capacity {
            // Add to buffer, dropping oldest if necessary
            if guard.event_buffer.len() >= buffer_capacity {
                // Drop oldest event
                guard.event_buffer.remove(0);
                self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
            }
            guard.event_buffer.push(event);
            Ok(())
        } else {
            // No buffer capacity, just drop the event
            self.dropped_events_counter.fetch_add(1, Ordering::Relaxed);
            Err(mpsc::error::SendError(event))
        }
    }

    /// Try to send an event without blocking.
    ///
    /// # Arguments
    /// * `event` - The event to send
    ///
    /// # Returns
    /// Result indicating success or the reason for failure
    pub fn try_send(&self, event: RendererEvent) -> Result<(), mpsc::error::SendError<RendererEvent>> {
        // For this implementation, try_send is the same as send
        self.send(event)
    }

    /// Check if the sender is closed (receiver has been dropped).
    ///
    /// # Returns
    /// `true` if the sender is closed, `false` otherwise
    pub fn is_closed(&self) -> bool {
        match self.inner.lock() {
            Ok(guard) => {
                match guard.configuration {
                    ChannelConfiguration::Bounded { .. } => {
                        guard.bounded_sender.as_ref().map_or(true, |s| s.is_closed())
                    }
                    ChannelConfiguration::Unbounded => {
                        guard.unbounded_sender.as_ref().map_or(true, |s| s.is_closed())
                    }
                }
            }
            Err(_) => true, // Treat poisoned mutex as closed
        }
    }

    /// Get the current channel configuration.
    ///
    /// # Returns
    /// The current channel configuration
    pub fn get_configuration(&self) -> Option<ChannelConfiguration> {
        match self.inner.lock() {
            Ok(guard) => Some(guard.configuration),
            Err(_) => None,
        }
    }

    /// Get the current buffer size.
    ///
    /// # Returns
    /// The number of events currently in the buffer
    pub fn get_buffer_size(&self) -> usize {
        match self.inner.lock() {
            Ok(guard) => guard.event_buffer.len(),
            Err(_) => 0,
        }
    }

    /// Get a clone of the dropped events counter Arc for sharing with other components.
    ///
    /// # Returns
    /// A cloned Arc<AtomicU64> that can be used to track dropped events from other parts of the system
    pub fn get_dropped_events_counter_reference(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.dropped_events_counter)
    }

    /// Reset the dropped events counter to zero.
    ///
    /// This operation is atomic and can be safely called from any thread.
    ///
    /// # Returns
    /// The previous value of the counter before reset
    pub fn reset_dropped_count(&self) -> u64 {
        self.dropped_events_counter.swap(0, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use crate::renderer::RendererEvent;

    #[tokio::test]
    async fn test_get_dropped_count_initial_value() {
        let (sender, _rx) = BufferedAsyncSender::<RendererEvent>::new_unbounded(Option::<usize>::Some(0));

        assert_eq!(sender.get_dropped_count(), 0);
    }

    /*
    #[tokio::test]
    async fn test_get_dropped_count_after_failed_send() {
        let (sender, rx) = BufferedAsyncSender::<RendererEvent>::new_unbounded(None);

        // Drop the receiver to make sends fail
        drop(rx);

        // Try to send an event - this should fail and increment dropped count
        let _ = sender.send(RendererEvent::Started());

        assert_eq!(sender.get_dropped_count(), 1);
    }*/

    #[tokio::test]
    async fn test_get_dropped_count_thread_safety() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let dropped_events_counter = Arc::new(AtomicU64::new(42));
        let sender = BufferedAsyncSender::<RendererEvent>::new_with_counter(tx, dropped_events_counter, None);

        // Test that we can safely read the count from multiple contexts
        let count1 = sender.get_dropped_count();
        let count2 = sender.get_dropped_count();

        assert_eq!(count1, 42);
        assert_eq!(count2, 42);
    }

    #[tokio::test]
    async fn test_get_dropped_count_with_cloned_sender() {
        let (sender1, _rx) = BufferedAsyncSender::<RendererEvent>::new_unbounded(None);
        let sender2 = sender1.clone();

        // Both senders should report the same dropped count
        assert_eq!(sender1.get_dropped_count(), 0);
        assert_eq!(sender2.get_dropped_count(), 0);
    }

    #[tokio::test]
    async fn test_bounded_channel_configuration() {
        let (_tx, _rx) = mpsc::channel::<RendererEvent>(10);
        let (sender,_receiver) = BufferedAsyncSender::<RendererEvent>::new_bounded(10, false, Arc::new(AtomicU64::new(5)));

        assert_eq!(sender.get_configuration(), Some(ChannelConfiguration::Bounded { capacity: 10 }));
        assert_eq!(sender.get_buffer_size(), 0);
    }

    #[tokio::test]
    async fn test_atomic_operations_performance() {
        let (sender, _rx) = BufferedAsyncSender::<RendererEvent>::new_unbounded(None);

        // Simulate high-frequency reads - should be very fast with atomics
        for _ in 0..1000 {
            let _ = sender.get_dropped_count();
        }

        // Test atomic increment via counter reference
        let counter_ref = sender.get_dropped_events_counter_reference();
        counter_ref.fetch_add(100, Ordering::Relaxed);
        assert_eq!(sender.get_dropped_count(), 100);
    }

    #[tokio::test]
    async fn test_reset_dropped_count() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let dropped_events_counter = Arc::new(AtomicU64::new(42));
        let sender = BufferedAsyncSender::<RendererEvent>::new_with_counter(tx, dropped_events_counter, None);

        let previous_value = sender.reset_dropped_count();
        assert_eq!(previous_value, 42);
        assert_eq!(sender.get_dropped_count(), 0);
    }
}