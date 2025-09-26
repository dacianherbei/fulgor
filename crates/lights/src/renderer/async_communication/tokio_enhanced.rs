//! Optional tokio-enhanced features for BufferedAsyncSender
//!
//! This module provides enhanced timeout functionality when the "tokio-timeout" feature is enabled.

#[cfg(feature = "tokio-timeout")]
use super::sender::{BufferedAsyncSender, SendEventError};
#[cfg(feature = "tokio-timeout")]
use crate::renderer::RendererEvent;
#[cfg(feature = "tokio-timeout")]
use std::time::Duration;

#[cfg(feature = "tokio-timeout")]
impl<NumberType> BufferedAsyncSender<NumberType>
where
    NumberType: Copy + Clone + Send + Sync + Default + 'static,
{
    /// Send an event with precise tokio-based timeout
    pub async fn send_event_with_precise_timeout(
        &self,
        event: RendererEvent,
        timeout: Duration,
    ) -> Result<(), SendEventError> {
        match tokio::time::timeout(timeout, self.sender.send(event.clone())).await {
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

    /// Send multiple events with batch timeout
    pub async fn send_events_batch(
        &self,
        events: Vec<RendererEvent>,
        total_timeout: Duration,
    ) -> Result<usize, SendEventError> {
        let start_time = tokio::time::Instant::now();
        let mut sent_count = 0;

        for event in events {
            let remaining_time = total_timeout
                .checked_sub(start_time.elapsed())
                .unwrap_or(Duration::from_millis(0));

            if remaining_time.is_zero() {
                break;
            }

            match self.send_event_with_precise_timeout(event, remaining_time).await {
                Ok(()) => sent_count += 1,
                Err(SendEventError::Timeout) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(sent_count)
    }
}

#[cfg(not(feature = "tokio-timeout"))]
/// Placeholder when tokio-timeout feature is not enabled
pub fn tokio_features_not_available() {
    println!("Enable the 'tokio-timeout' feature for enhanced timeout functionality");
}