//! Types for async communication in fulgor::renderer
//!
//! This module provides templated types for async event communication
//! with configurable precision and channel behavior.

use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Configuration for async channel behavior
#[derive(Debug, Clone)]
pub struct AsyncChannelConfig<NumberType = f64>
where
    NumberType: Copy + Clone + Send + Sync + 'static,
{
    /// Maximum buffer size before events start getting dropped
    pub maximum_buffer_size: usize,
    /// Timeout for send operations
    pub send_timeout: Option<Duration>,
    /// Whether to enable backpressure or drop events when full
    pub enable_backpressure: bool,
    /// Statistics collection interval
    pub statistics_interval: Duration,
    /// Precision-dependent threshold (templated for future use)
    pub precision_threshold: NumberType,
}

impl<NumberType> Default for AsyncChannelConfig<NumberType>
where
    NumberType: Copy + Clone + Send + Sync + Default + 'static,
{
    fn default() -> Self {
        Self {
            maximum_buffer_size: 1000,
            send_timeout: Some(Duration::from_millis(100)),
            enable_backpressure: false,
            statistics_interval: Duration::from_secs(1),
            precision_threshold: NumberType::default(),
        }
    }
}

/// Receiver side of async event communication
pub struct AsyncEventReceiver<EventType, NumberType = f64>
where
    EventType: Clone + Send + Sync + 'static,
    NumberType: Copy + Clone + Send + Sync + 'static,
{
    receiver: async_channel::Receiver<EventType>,
    config: AsyncChannelConfig<NumberType>,
    received_events_count: Arc<Mutex<u64>>,
}

impl<EventType, NumberType> AsyncEventReceiver<EventType, NumberType>
where
    EventType: Clone + Send + Sync + 'static,
    NumberType: Copy + Clone + Send + Sync + 'static,
{
    pub(crate) fn new(
        receiver: async_channel::Receiver<EventType>,
        config: AsyncChannelConfig<NumberType>,
    ) -> Self {
        Self {
            receiver,
            config,
            received_events_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Receive next event asynchronously
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

    /// Try to receive event without blocking
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

    /// Get total received events count
    pub fn received_events_count(&self) -> u64 {
        self.received_events_count
            .lock()
            .map(|count| *count)
            .unwrap_or(0)
    }

    /// Get configuration reference
    pub fn configuration(&self) -> &AsyncChannelConfig<NumberType> {
        &self.config
    }
}